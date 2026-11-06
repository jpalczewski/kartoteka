use super::*;
use chrono::Offset as _;
use chrono_tz::{Europe, Tz};

const UPDATED_AT: &str = "2024-05-01 08:00:00";
const ITEM_ID: &str = "11111111-1111-1111-1111-111111111111";

fn base_item() -> FeedItem {
    FeedItem {
        id: ITEM_ID.to_string(),
        list_name: "Praca".to_string(),
        title: "Zadanie".to_string(),
        description: None,
        completed: false,
        start_date: None,
        start_time: None,
        deadline: None,
        deadline_time: None,
        hard_deadline: None,
        updated_at: UPDATED_AT.to_string(),
    }
}

fn render(items: &[FeedItem], reminders: &ReminderDefaults) -> String {
    render_feed("Moja lista", items, Europe::Warsaw, reminders)
}

fn events(feed: &str) -> Vec<&str> {
    feed.split("BEGIN:VEVENT").skip(1).collect()
}

fn only_event(feed: &str) -> String {
    let events = events(feed);
    assert_eq!(events.len(), 1, "expected exactly one VEVENT in:\n{feed}");
    events[0].to_string()
}

/// Properties of the component itself, ignoring nested `VALARM` children.
fn lines<'a>(block: &'a str, key: &str) -> Vec<&'a str> {
    let head = block.split("BEGIN:VALARM").next().unwrap_or(block);
    head.lines().filter(|line| line.starts_with(key)).collect()
}

#[test]
fn calendar_header_is_complete_and_valid() {
    let feed = render(&[], &ReminderDefaults::default());
    assert!(feed.starts_with("BEGIN:VCALENDAR\r\n"));
    assert!(feed.ends_with("END:VCALENDAR\r\n"));
    assert_eq!(lines(&feed, "VERSION:"), vec!["VERSION:2.0"]);
    assert_eq!(lines(&feed, "PRODID:").len(), 1);
    assert_eq!(
        lines(&feed, "X-WR-CALNAME:"),
        vec!["X-WR-CALNAME:Moja lista"]
    );
    assert_eq!(
        lines(&feed, "X-WR-TIMEZONE:"),
        vec!["X-WR-TIMEZONE:Europe/Warsaw"]
    );
    assert_eq!(
        lines(&feed, "REFRESH-INTERVAL"),
        vec!["REFRESH-INTERVAL;VALUE=DURATION:PT1H"]
    );
    assert_eq!(
        lines(&feed, "X-PUBLISHED-TTL:"),
        vec!["X-PUBLISHED-TTL:PT1H"]
    );
    assert!(events(&feed).is_empty());
}

#[test]
fn all_day_event_uses_next_day_as_end() {
    let item = FeedItem {
        hard_deadline: Some("2024-05-07".to_string()),
        ..base_item()
    };
    let event = only_event(&render(&[item], &ReminderDefaults::default()));
    assert_eq!(
        lines(&event, "DTSTART"),
        vec!["DTSTART;VALUE=DATE:20240507"]
    );
    assert_eq!(lines(&event, "DTEND"), vec!["DTEND;VALUE=DATE:20240508"]);
}

#[test]
fn timed_event_is_utc_and_lasts_thirty_minutes() {
    let item = FeedItem {
        start_date: Some("2024-05-06".to_string()),
        start_time: Some("09:30:00".to_string()),
        ..base_item()
    };
    let event = only_event(&render(&[item], &ReminderDefaults::default()));
    assert_eq!(lines(&event, "DTSTART"), vec!["DTSTART:20240506T073000Z"]);
    assert_eq!(lines(&event, "DTEND"), vec!["DTEND:20240506T080000Z"]);
}

#[test]
fn invalid_time_falls_back_to_all_day() {
    let item = FeedItem {
        deadline: Some("2024-05-07".to_string()),
        deadline_time: Some("25:99".to_string()),
        ..base_item()
    };
    let event = only_event(&render(&[item], &ReminderDefaults::default()));
    assert_eq!(
        lines(&event, "DTSTART"),
        vec!["DTSTART;VALUE=DATE:20240507"]
    );
    assert!(lines(&event, "DTEND")[0].contains("VALUE=DATE"));
}

#[test]
fn dst_gap_wall_clock_moves_to_earliest_valid_instant() {
    // 2024-03-31 02:30 does not exist in Europe/Warsaw (clocks jump to 03:00).
    let item = FeedItem {
        deadline: Some("2024-03-31".to_string()),
        deadline_time: Some("02:30".to_string()),
        ..base_item()
    };
    let event = only_event(&render(&[item], &ReminderDefaults::default()));
    assert_eq!(lines(&event, "DTSTART"), vec!["DTSTART:20240331T010000Z"]);
}

#[test]
fn dst_ambiguity_resolves_to_earliest_instant() {
    let naive = NaiveDate::from_ymd_opt(2024, 10, 27)
        .unwrap()
        .and_hms_opt(2, 30, 0)
        .unwrap();
    let resolved = resolve_local(Europe::Warsaw, naive).unwrap();
    // First occurrence is still on summer time (+02:00), the second one is +01:00.
    assert_eq!(resolved.offset().fix().local_minus_utc(), 2 * 3600);
    assert_eq!(resolved.naive_local(), naive);

    // The rendered event is the first occurrence, in UTC.
    let item = FeedItem {
        deadline: Some("2024-10-27".to_string()),
        deadline_time: Some("02:30".to_string()),
        ..base_item()
    };
    let event = only_event(&render(&[item], &ReminderDefaults::default()));
    assert_eq!(lines(&event, "DTSTART"), vec!["DTSTART:20241027T003000Z"]);
}

#[test]
fn one_item_produces_three_events_with_distinct_uids() {
    let item = FeedItem {
        start_date: Some("2024-05-06".to_string()),
        deadline: Some("2024-05-07".to_string()),
        hard_deadline: Some("2024-05-10".to_string()),
        ..base_item()
    };
    let feed = render(&[item], &ReminderDefaults::default());
    let events = events(&feed);
    assert_eq!(events.len(), 3);

    let uids: Vec<&str> = events
        .iter()
        .flat_map(|event| lines(event, "UID:"))
        .collect();
    assert_eq!(
        uids,
        vec![
            format!("UID:{ITEM_ID}-start@kartoteka"),
            format!("UID:{ITEM_ID}-deadline@kartoteka"),
            format!("UID:{ITEM_ID}-hard@kartoteka"),
        ]
    );

    let summaries: Vec<&str> = events
        .iter()
        .flat_map(|event| lines(event, "SUMMARY:"))
        .collect();
    assert_eq!(
        summaries,
        vec!["SUMMARY:Zadanie", "SUMMARY:⏰ Zadanie", "SUMMARY:‼ Zadanie"]
    );
}

#[test]
fn missing_dates_produce_no_events() {
    let feed = render(&[base_item()], &ReminderDefaults::default());
    assert!(events(&feed).is_empty());
}

#[test]
fn completed_items_get_a_check_mark_and_no_alarms() {
    let item = FeedItem {
        completed: true,
        deadline: Some("2024-05-07".to_string()),
        hard_deadline: Some("2024-05-10".to_string()),
        ..base_item()
    };
    let feed = render(&[item], &ReminderDefaults::default());
    assert!(!feed.contains("BEGIN:VALARM"));
    assert!(!feed.contains("TRIGGER"));
    let summaries: Vec<&str> = events(&feed)
        .iter()
        .flat_map(|event| lines(event, "SUMMARY:"))
        .collect();
    assert_eq!(
        summaries,
        vec!["SUMMARY:✓ ⏰ Zadanie", "SUMMARY:✓ ‼ Zadanie"]
    );
}

#[test]
fn reminders_follow_defaults_per_date_field() {
    let item = FeedItem {
        start_date: Some("2024-05-06".to_string()),
        deadline: Some("2024-05-07".to_string()),
        hard_deadline: Some("2024-05-10".to_string()),
        ..base_item()
    };
    let feed = render(&[item], &ReminderDefaults::default());
    let triggers: Vec<&str> = feed
        .lines()
        .filter(|line| line.starts_with("TRIGGER"))
        .collect();
    // start_date: none, deadline: 60, hard_deadline: 1440 and 60.
    assert_eq!(
        triggers,
        vec!["TRIGGER:-PT60M", "TRIGGER:-PT1440M", "TRIGGER:-PT60M"]
    );

    let alarms: Vec<&str> = feed
        .lines()
        .filter(|line| line.starts_with("ACTION:"))
        .collect();
    assert_eq!(alarms, vec!["ACTION:DISPLAY"; 3]);

    // Every alarm repeats the event summary as its description.
    let descriptions: Vec<&str> = feed
        .lines()
        .filter(|line| line.starts_with("DESCRIPTION:⏰"))
        .collect();
    assert_eq!(descriptions.len(), 1);
}

#[test]
fn custom_reminders_are_rendered_for_every_date_field() {
    let reminders = ReminderDefaults {
        start_date: vec![10, 0],
        deadline: vec![],
        hard_deadline: vec![40320],
    };
    let item = FeedItem {
        start_date: Some("2024-05-06".to_string()),
        deadline: Some("2024-05-07".to_string()),
        hard_deadline: Some("2024-05-10".to_string()),
        ..base_item()
    };
    let feed = render(&[item], &reminders);
    let triggers: Vec<&str> = feed
        .lines()
        .filter(|line| line.starts_with("TRIGGER"))
        .collect();
    assert_eq!(
        triggers,
        vec!["TRIGGER:-PT10M", "TRIGGER:-PT0M", "TRIGGER:-PT40320M"]
    );
    assert_eq!(feed.matches("BEGIN:VALARM").count(), 3);
}

#[test]
fn text_values_are_escaped() {
    let item = FeedItem {
        title: "Zakupy, mleko; chleb\nmasło".to_string(),
        description: Some("Linia 1\nLinia 2".to_string()),
        start_date: Some("2024-05-06".to_string()),
        ..base_item()
    };
    let feed = render(&[item], &ReminderDefaults::default());
    assert!(
        feed.contains("SUMMARY:Zakupy\\, mleko\\; chleb\\nmasło\r\n"),
        "{feed}"
    );
    assert!(
        feed.contains("DESCRIPTION:List: Praca\\n\\nLinia 1\\nLinia 2\r\n"),
        "{feed}"
    );
}

#[test]
fn description_contains_list_name() {
    let event = only_event(&render(
        &[FeedItem {
            start_date: Some("2024-05-06".to_string()),
            ..base_item()
        }],
        &ReminderDefaults::default(),
    ));
    assert_eq!(
        lines(&event, "DESCRIPTION"),
        vec!["DESCRIPTION:List: Praca"]
    );
}

#[test]
fn invalid_dates_are_skipped_without_panicking() {
    let item = FeedItem {
        start_date: Some("not-a-date".to_string()),
        deadline: Some("2024-02-31".to_string()),
        hard_deadline: Some("2024-05-10".to_string()),
        ..base_item()
    };
    let feed = render(&[item], &ReminderDefaults::default());
    let events = events(&feed);
    assert_eq!(events.len(), 1);
    assert_eq!(
        lines(events[0], "UID:"),
        vec![format!("UID:{ITEM_ID}-hard@kartoteka")]
    );
}

#[test]
fn timestamps_come_from_updated_at() {
    let item = FeedItem {
        hard_deadline: Some("2024-05-10".to_string()),
        updated_at: "2024-05-01 08:00:00".to_string(),
        ..base_item()
    };
    let event = only_event(&render(&[item], &ReminderDefaults::default()));
    assert_eq!(lines(&event, "DTSTAMP:"), vec!["DTSTAMP:20240501T080000Z"]);
    assert_eq!(
        lines(&event, "LAST-MODIFIED:"),
        vec!["LAST-MODIFIED:20240501T080000Z"]
    );
}

#[test]
fn rendering_is_deterministic() {
    let items = vec![
        FeedItem {
            start_date: Some("2024-05-06".to_string()),
            start_time: Some("09:30".to_string()),
            deadline: Some("2024-05-07".to_string()),
            hard_deadline: Some("2024-05-10".to_string()),
            description: Some("Opis".to_string()),
            ..base_item()
        },
        FeedItem {
            id: "22222222-2222-2222-2222-222222222222".to_string(),
            title: "Drugie".to_string(),
            deadline: Some("2024-05-08".to_string()),
            ..base_item()
        },
    ];
    let reminders = ReminderDefaults::default();
    let first = render(&items, &reminders);
    let second = render(&items, &reminders);
    assert_eq!(first, second);
}

#[test]
fn timezone_name_and_zone_are_reflected() {
    let item = FeedItem {
        deadline: Some("2024-05-07".to_string()),
        deadline_time: Some("12:00".to_string()),
        ..base_item()
    };
    let feed = render_feed("Moja lista", &[item], Tz::UTC, &ReminderDefaults::default());
    assert!(feed.contains("X-WR-TIMEZONE:UTC\r\n"));
    assert!(feed.contains("DTSTART:20240507T120000Z\r\n"));
}
