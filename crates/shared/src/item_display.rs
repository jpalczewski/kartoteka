//! Pure display helpers for item rows: detail links, overdue state, date chips and quantity labels.
//!
//! Like `human_date`, everything here takes `today` explicitly, so the module is WASM-safe and
//! deterministic in tests.

use crate::human_date::{DateUrgency, date_urgency, format_date_human};
use crate::locale::Locale;
use crate::types::{FlexDate, Item};
use chrono::NaiveDate;

/// Link to an item's detail page.
pub fn item_href(item: &Item) -> String {
    format!("/lists/{}/items/{}", item.list_id, item.id)
}

/// Whether an item is overdue on `today`.
///
/// Date granularity only: times are ignored. A `Week`/`Month` deadline counts as overdue only once
/// its LAST day is before `today`, so a fuzzy deadline is not overdue while we are still inside
/// its period. Completed items are never overdue. Uses the same rule as
/// `date_urgency(..) == DateUrgency::Overdue`.
pub fn is_item_overdue(item: &Item, today: NaiveDate) -> bool {
    [item.deadline.as_ref(), item.hard_deadline.as_ref()]
        .into_iter()
        .flatten()
        .any(|date| date_urgency(date, today, item.completed) == DateUrgency::Overdue)
}

/// Which item date a [`DateChip`] represents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateChipKind {
    HardDeadline,
    Deadline,
    Start,
}

/// One date chip shown on an item row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateChip {
    pub kind: DateChipKind,
    /// Human label, from `format_date_human`.
    pub label: String,
    /// The matching time ("HH:MM"), trimmed. `None` when blank or not applicable.
    pub time: Option<String>,
    /// Urgency for styling. Start chips are only ever `Today` or `None` (see [`item_date_chips`]).
    pub urgency: DateUrgency,
}

/// Date chips for an item, in the order hard deadline, deadline, start. Absent dates are skipped.
///
/// - `label` comes from `format_date_human`.
/// - `time` is `deadline_time` for the deadline chip and `start_time` for the start chip. The hard
///   deadline has no time chip. Blank times become `None`.
/// - `urgency` is `date_urgency` for deadlines. Start chips only ever get `Today` (the start date
///   is today) and `None` otherwise: a start date in the past or future is not a warning.
pub fn item_date_chips(item: &Item, today: NaiveDate, locale: Locale) -> Vec<DateChip> {
    let mut chips = Vec::with_capacity(3);

    if let Some(date) = &item.hard_deadline {
        chips.push(deadline_chip(
            DateChipKind::HardDeadline,
            date,
            None,
            item,
            today,
            locale,
        ));
    }
    if let Some(date) = &item.deadline {
        chips.push(deadline_chip(
            DateChipKind::Deadline,
            date,
            item.deadline_time.as_deref(),
            item,
            today,
            locale,
        ));
    }
    if let Some(date) = &item.start_date {
        let urgency = match date_urgency(date, today, item.completed) {
            DateUrgency::Today => DateUrgency::Today,
            _ => DateUrgency::None,
        };
        chips.push(DateChip {
            kind: DateChipKind::Start,
            label: format_date_human(date, today, locale),
            time: clean_time(item.start_time.as_deref()),
            urgency,
        });
    }

    chips
}

fn deadline_chip(
    kind: DateChipKind,
    date: &FlexDate,
    time: Option<&str>,
    item: &Item,
    today: NaiveDate,
    locale: Locale,
) -> DateChip {
    DateChip {
        kind,
        label: format_date_human(date, today, locale),
        time: clean_time(time),
        urgency: date_urgency(date, today, item.completed),
    }
}

fn clean_time(time: Option<&str>) -> Option<String> {
    time.map(str::trim)
        .filter(|t| !t.is_empty())
        .map(String::from)
}

/// Unit shown when an item has none: "szt." in Polish, "pcs" in English.
pub fn default_unit(locale: Locale) -> &'static str {
    match locale {
        Locale::Pl => "szt.",
        Locale::En => "pcs",
    }
}

/// Quantity label such as `"3/5 szt."` (actual/target), `"5 szt."` (target only), `"3 szt."`
/// (actual only), or `None` when neither is set.
///
/// The unit is trimmed; a blank or missing unit falls back to [`default_unit`].
pub fn quantity_label(
    actual: Option<i32>,
    target: Option<i32>,
    unit: Option<&str>,
    locale: Locale,
) -> Option<String> {
    let unit = unit
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| default_unit(locale));

    let number = match (actual, target) {
        (Some(a), Some(t)) => format!("{a}/{t}"),
        (None, Some(t)) => t.to_string(),
        (Some(a), None) => a.to_string(),
        (None, None) => return None,
    };
    Some(format!("{number} {unit}"))
}
