//! Pure iCalendar (RFC 5545) rendering of the personal calendar feed.
//!
//! There is no database access in this module: callers map their rows into
//! [`FeedItem`] and hand the slice to [`render_feed`]. Rendering is
//! deterministic for identical input, so the HTTP layer can derive a stable
//! `ETag` from the generated document — nothing here reads the clock.

use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use icalendar::{
    Alarm, Calendar, Component, DatePerhapsTime, Event, EventLike, Parameter, Property, Trigger,
};
use kartoteka_shared::{Locale, ReminderDefaults};

/// How long a timed event lasts.
const TIMED_EVENT_MINUTES: i64 = 30;
/// Refresh hint advertised to calendar clients.
const FEED_TTL: &str = "PT1H";
/// Widest DST gap we scan when a wall-clock time does not exist in the zone.
const MAX_DST_GAP_MINUTES: i64 = 180;
/// SQLite timestamp format (`datetime('now')`).
const SQLITE_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
/// iCalendar UTC date-time format.
const ICAL_UTC_FORMAT: &str = "%Y%m%dT%H%M%SZ";

/// One item, decoupled from the database row type.
///
/// Dates are `YYYY-MM-DD`, times are `HH:MM` or `HH:MM:SS`, and `updated_at`
/// is a SQLite (`YYYY-MM-DD HH:MM:SS`) UTC timestamp.
#[derive(Debug, Clone)]
pub struct FeedItem {
    pub id: String,
    pub list_name: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub updated_at: String,
}

/// Which date column an event was generated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Start,
    Deadline,
    Hard,
}

impl Kind {
    const ALL: [Self; 3] = [Self::Start, Self::Deadline, Self::Hard];

    /// Stable `UID` suffix, so regenerated feeds keep the same event identity.
    fn uid_suffix(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Deadline => "deadline",
            Self::Hard => "hard",
        }
    }

    fn summary_prefix(self) -> &'static str {
        match self {
            Self::Start => "",
            Self::Deadline => "⏰ ",
            Self::Hard => "‼ ",
        }
    }

    fn date_and_time(self, item: &FeedItem) -> (Option<&str>, Option<&str>) {
        match self {
            Self::Start => (item.start_date.as_deref(), item.start_time.as_deref()),
            Self::Deadline => (item.deadline.as_deref(), item.deadline_time.as_deref()),
            // There is no hard-deadline time column.
            Self::Hard => (item.hard_deadline.as_deref(), None),
        }
    }

    fn reminder_offsets(self, defaults: &ReminderDefaults) -> &[u32] {
        match self {
            Self::Start => &defaults.start_date,
            Self::Deadline => &defaults.deadline,
            Self::Hard => &defaults.hard_deadline,
        }
    }
}

/// Renders a complete `VCALENDAR` document with one `VEVENT` per present date
/// field (an item can therefore produce up to three events).
///
/// Local times are interpreted in `tz` and written as UTC; a wall-clock time that does not exist
/// (DST gap) is moved to the earliest valid instant, an ambiguous one resolves
/// to its first occurrence. Unparseable dates are skipped silently.
pub fn render_feed(
    calendar_name: &str,
    items: &[FeedItem],
    tz: Tz,
    reminders: &ReminderDefaults,
    locale: Locale,
) -> String {
    let mut calendar = Calendar::empty();
    calendar
        .append_property(Property::new("VERSION", "2.0"))
        .append_property(Property::new("PRODID", "-//Kartoteka//iCal Feed//EN"))
        .append_property(Property::new("CALSCALE", "GREGORIAN"))
        .append_property(Property::new("X-WR-CALNAME", calendar_name))
        .append_property(Property::new("X-WR-TIMEZONE", tz.name()))
        .append_property(
            Property::new("REFRESH-INTERVAL", FEED_TTL)
                .append_parameter(Parameter::new("VALUE", "DURATION"))
                .done(),
        )
        .append_property(Property::new("X-PUBLISHED-TTL", FEED_TTL));

    for item in items {
        for kind in Kind::ALL {
            if let Some(event) = build_event(item, kind, tz, reminders, locale) {
                calendar.push(event);
            }
        }
    }

    calendar.to_string()
}

fn build_event(
    item: &FeedItem,
    kind: Kind,
    tz: Tz,
    reminders: &ReminderDefaults,
    locale: Locale,
) -> Option<Event> {
    let (date, time) = kind.date_and_time(item);
    let date = NaiveDate::parse_from_str(date?.trim(), "%Y-%m-%d").ok()?;

    let uid_base = format!("{}-{}", item.id, kind.uid_suffix());
    let stamp = parse_stamp(&item.updated_at);
    let summary = build_summary(item, kind);

    let mut event = Event::with_uid(&format!("{uid_base}@kartoteka"));
    event
        .summary(&summary)
        .description(&build_description(item, locale))
        .timestamp(stamp)
        .last_modified(stamp);

    match time
        .and_then(parse_time)
        .and_then(|time| resolve_local(tz, date.and_time(time)))
    {
        Some(start) => {
            let end = start + Duration::minutes(TIMED_EVENT_MINUTES);
            // Emitted in UTC: a TZID would require a matching VTIMEZONE block.
            event
                .starts(start.with_timezone(&Utc))
                .ends(end.with_timezone(&Utc));
        }
        None => {
            event
                .starts(DatePerhapsTime::Date(date))
                .ends(DatePerhapsTime::Date(date.succ_opt()?));
        }
    }

    if !item.completed {
        for (index, minutes) in kind.reminder_offsets(reminders).iter().enumerate() {
            event.alarm(display_alarm(&summary, *minutes, &uid_base, index, stamp));
        }
    }

    Some(event)
}

fn build_summary(item: &FeedItem, kind: Kind) -> String {
    let summary = format!("{}{}", kind.summary_prefix(), item.title);
    if item.completed {
        format!("✓ {summary}")
    } else {
        summary
    }
}

/// Label for the list name in an event description.
fn list_label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "List",
        Locale::Pl => "Lista",
    }
}

fn build_description(item: &FeedItem, locale: Locale) -> String {
    let label = list_label(locale);
    match item
        .description
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        Some(text) => format!("{label}: {}\n\n{text}", item.list_name),
        None => format!("{label}: {}", item.list_name),
    }
}

/// Builds a `VALARM` that fires `minutes` before the event.
///
/// `icalendar` always writes `DTSTAMP` (from `Utc::now()`) and a random `UID`
/// into a `VALARM` that does not carry them, which would make the feed
/// non-deterministic, so both are set explicitly from stable input.
fn display_alarm(
    summary: &str,
    minutes: u32,
    uid_base: &str,
    index: usize,
    stamp: DateTime<Utc>,
) -> Alarm {
    let mut alarm = Alarm::display(summary, Trigger::Duration(Duration::zero(), None));
    // `Trigger`'s writer renders ISO 8601 durations in seconds; feeds expect minutes.
    alarm.add_property("TRIGGER", format!("-PT{minutes}M"));
    alarm.add_property("DTSTAMP", stamp.format(ICAL_UTC_FORMAT).to_string());
    alarm.add_property("UID", format!("{uid_base}-alarm{}@kartoteka", index + 1));
    alarm.done()
}

/// Resolves a wall-clock time in `tz`, preferring the earliest valid instant.
fn resolve_local(tz: Tz, naive: NaiveDateTime) -> Option<DateTime<Tz>> {
    match tz.from_local_datetime(&naive) {
        LocalResult::Single(date_time) => Some(date_time),
        LocalResult::Ambiguous(earliest, _latest) => Some(earliest),
        // DST gap: the wall-clock time never happens, take the first instant that does.
        LocalResult::None => (1..=MAX_DST_GAP_MINUTES).find_map(|minutes| {
            tz.from_local_datetime(&(naive + Duration::minutes(minutes)))
                .earliest()
        }),
    }
}

fn parse_time(raw: &str) -> Option<NaiveTime> {
    let raw = raw.trim();
    NaiveTime::parse_from_str(raw, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(raw, "%H:%M"))
        .ok()
}

/// Parses an `updated_at` value; falls back to the epoch so output stays deterministic.
fn parse_stamp(updated_at: &str) -> DateTime<Utc> {
    let raw = updated_at.trim();
    NaiveDateTime::parse_from_str(raw, SQLITE_DATETIME_FORMAT)
        .map(|date_time| date_time.and_utc())
        .or_else(|_| {
            DateTime::parse_from_rfc3339(raw).map(|date_time| date_time.with_timezone(&Utc))
        })
        .unwrap_or(DateTime::UNIX_EPOCH)
}

#[cfg(test)]
mod tests;
