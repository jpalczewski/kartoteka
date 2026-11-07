use crate::Locale;
use crate::human_date::DateUrgency;
use crate::item_display::{
    DateChip, DateChipKind, default_unit, is_item_overdue, item_date_chips, item_href,
    quantity_label,
};
use crate::types::{FlexDate, Item};
use chrono::{Duration, NaiveDate};

// Saturday 2026-11-07; ISO week 45 is Mon 2026-11-02 .. Sun 2026-11-08.
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 7).unwrap()
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn day(date: NaiveDate) -> Option<FlexDate> {
    Some(FlexDate::Day(date))
}

fn item() -> Item {
    Item {
        id: "i1".into(),
        list_id: "l1".into(),
        title: "t".into(),
        description: None,
        completed: false,
        status: "todo".into(),
        icon: None,
        position: 0,
        quantity: None,
        actual_quantity: None,
        unit: None,
        start_date: None,
        start_time: None,
        deadline: None,
        deadline_time: None,
        hard_deadline: None,
        estimated_duration: None,
        location_id: None,
        recurrence_rule: None,
        recurrence_mode: None,
        created_at: "2026-03-01".into(),
        updated_at: "2026-03-01".into(),
    }
}

// --- href ---

#[test]
fn href_points_at_item_detail_page() {
    assert_eq!(item_href(&item()), "/lists/l1/items/i1");
}

// --- overdue ---

#[test]
fn overdue_day_deadline_past_current_future() {
    let mut it = item();
    it.deadline = day(d(2026, 11, 6));
    assert!(is_item_overdue(&it, today()));

    it.deadline = day(today());
    assert!(!is_item_overdue(&it, today()));

    it.deadline = day(d(2026, 11, 8));
    assert!(!is_item_overdue(&it, today()));
}

#[test]
fn overdue_week_deadline_only_after_its_last_day() {
    let mut it = item();
    // Week 45 = 2026-11-02 .. 2026-11-08: still inside the period on "today".
    it.deadline = Some(FlexDate::Week(2026, 45));
    assert!(!is_item_overdue(&it, today()));
    // Last day of the period itself is not overdue.
    assert!(!is_item_overdue(&it, d(2026, 11, 8)));
    // First day after the period is overdue.
    assert!(is_item_overdue(&it, d(2026, 11, 9)));

    // Past week.
    it.deadline = Some(FlexDate::Week(2026, 44));
    assert!(is_item_overdue(&it, today()));

    // Future week.
    it.deadline = Some(FlexDate::Week(2026, 46));
    assert!(!is_item_overdue(&it, today()));
}

#[test]
fn overdue_month_deadline_only_after_its_last_day() {
    let mut it = item();
    // Current month: still inside the period.
    it.deadline = Some(FlexDate::Month(2026, 11));
    assert!(!is_item_overdue(&it, today()));
    assert!(!is_item_overdue(&it, d(2026, 11, 30)));
    assert!(is_item_overdue(&it, d(2026, 12, 1)));

    // Past month.
    it.deadline = Some(FlexDate::Month(2026, 10));
    assert!(is_item_overdue(&it, today()));

    // Future month.
    it.deadline = Some(FlexDate::Month(2026, 12));
    assert!(!is_item_overdue(&it, today()));
}

#[test]
fn overdue_hard_deadline_past_makes_item_overdue() {
    let mut it = item();
    it.hard_deadline = day(d(2026, 11, 1));
    it.deadline = day(d(2026, 11, 20));
    assert!(is_item_overdue(&it, today()));
}

#[test]
fn overdue_hard_deadline_future_or_absent_is_not_overdue() {
    let mut it = item();
    assert!(!is_item_overdue(&it, today()));

    it.hard_deadline = day(d(2026, 11, 20));
    assert!(!is_item_overdue(&it, today()));
}

#[test]
fn overdue_without_any_deadline_is_false() {
    let mut it = item();
    it.start_date = day(d(2026, 1, 1));
    assert!(!is_item_overdue(&it, today()));
}

#[test]
fn overdue_completed_is_never_overdue() {
    let mut it = item();
    it.completed = true;
    it.deadline = day(d(2026, 1, 1));
    it.hard_deadline = day(d(2026, 1, 1));
    assert!(!is_item_overdue(&it, today()));
}

#[test]
fn overdue_ignores_deadline_time() {
    // Date granularity: 00:00 on today's date is not overdue, and a time earlier than "now"
    // does not make a same-day deadline overdue.
    let mut it = item();
    it.deadline = day(today());
    it.deadline_time = Some("00:00".into());
    assert!(!is_item_overdue(&it, today()));
}

// --- date chips ---

#[test]
fn chips_empty_when_item_has_no_dates() {
    assert!(item_date_chips(&item(), today(), Locale::En).is_empty());
}

#[test]
fn chips_order_is_hard_deadline_deadline_start() {
    let mut it = item();
    it.start_date = day(d(2026, 11, 1));
    it.deadline = day(d(2026, 11, 20));
    it.hard_deadline = day(d(2026, 11, 30));

    let kinds: Vec<DateChipKind> = item_date_chips(&it, today(), Locale::En)
        .into_iter()
        .map(|c| c.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            DateChipKind::HardDeadline,
            DateChipKind::Deadline,
            DateChipKind::Start
        ]
    );
}

#[test]
fn chips_skip_absent_dates() {
    let mut it = item();
    it.deadline = day(d(2026, 11, 20));
    let chips = item_date_chips(&it, today(), Locale::En);
    assert_eq!(chips.len(), 1);
    assert_eq!(chips[0].kind, DateChipKind::Deadline);
}

#[test]
fn chip_labels_pl_and_en() {
    let mut it = item();
    it.deadline = day(d(2026, 11, 20));
    it.hard_deadline = Some(FlexDate::Week(2026, 45));
    it.start_date = day(today() + Duration::days(1));

    let pl = item_date_chips(&it, today(), Locale::Pl);
    assert_eq!(pl[0].label, "ten tydzień");
    assert_eq!(pl[1].label, "20 lis");
    assert_eq!(pl[2].label, "jutro");

    let en = item_date_chips(&it, today(), Locale::En);
    assert_eq!(en[0].label, "this week");
    assert_eq!(en[1].label, "Nov 20");
    assert_eq!(en[2].label, "tomorrow");
}

#[test]
fn chip_time_comes_from_matching_field_only() {
    let mut it = item();
    it.hard_deadline = day(d(2026, 11, 20));
    it.deadline = day(d(2026, 11, 21));
    it.start_date = day(d(2026, 11, 22));
    it.deadline_time = Some("17:30".into());
    it.start_time = Some("09:00".into());

    let chips = item_date_chips(&it, today(), Locale::En);
    // Hard deadline never carries a time, even when deadline_time is set.
    assert_eq!(chips[0].time, None);
    assert_eq!(chips[1].time.as_deref(), Some("17:30"));
    assert_eq!(chips[2].time.as_deref(), Some("09:00"));
}

#[test]
fn chip_blank_times_become_none_and_are_trimmed() {
    let mut it = item();
    it.deadline = day(d(2026, 11, 20));
    it.start_date = day(d(2026, 11, 22));
    it.deadline_time = Some("   ".into());
    it.start_time = Some(" 08:15 ".into());

    let chips = item_date_chips(&it, today(), Locale::En);
    assert_eq!(chips[0].time, None);
    assert_eq!(chips[1].time.as_deref(), Some("08:15"));
}

#[test]
fn chip_deadline_urgency_follows_date_urgency() {
    let cases = [
        (d(2026, 11, 6), DateUrgency::Overdue),
        (d(2026, 11, 7), DateUrgency::Today),
        (d(2026, 11, 9), DateUrgency::Soon),
        (d(2026, 11, 20), DateUrgency::Later),
    ];
    for (date, expected) in cases {
        let mut it = item();
        it.deadline = day(date);
        let chips = item_date_chips(&it, today(), Locale::En);
        assert_eq!(chips[0].urgency, expected, "deadline {date}");
    }
}

#[test]
fn chip_deadline_urgency_is_none_when_completed() {
    let mut it = item();
    it.completed = true;
    it.deadline = day(d(2026, 11, 6));
    it.hard_deadline = day(d(2026, 11, 6));
    let chips = item_date_chips(&it, today(), Locale::En);
    assert!(chips.iter().all(|c| c.urgency == DateUrgency::None));
}

#[test]
fn chip_start_urgency_is_today_only_when_start_is_today() {
    let cases = [
        (d(2026, 11, 6), DateUrgency::None),
        (d(2026, 11, 7), DateUrgency::Today),
        (d(2026, 11, 8), DateUrgency::None),
        (d(2026, 12, 1), DateUrgency::None),
    ];
    for (date, expected) in cases {
        let mut it = item();
        it.start_date = day(date);
        let chips = item_date_chips(&it, today(), Locale::En);
        assert_eq!(chips[0].urgency, expected, "start {date}");
    }
}

#[test]
fn chip_start_urgency_is_none_when_completed() {
    let mut it = item();
    it.completed = true;
    it.start_date = day(today());
    let chips = item_date_chips(&it, today(), Locale::En);
    assert_eq!(chips[0].urgency, DateUrgency::None);
}

#[test]
fn chip_start_fuzzy_period_containing_today_is_not_highlighted() {
    // Only an exact-day start is highlighted; a week that contains today is `Soon` for
    // deadlines but gets no start highlight.
    let mut it = item();
    it.start_date = Some(FlexDate::Week(2026, 45));
    let chips = item_date_chips(&it, today(), Locale::En);
    assert_eq!(chips[0].urgency, DateUrgency::None);
}

#[test]
fn chip_struct_fields_are_populated() {
    let mut it = item();
    it.deadline = day(d(2026, 11, 7));
    it.deadline_time = Some("12:00".into());
    let chips = item_date_chips(&it, today(), Locale::Pl);
    assert_eq!(
        chips,
        vec![DateChip {
            kind: DateChipKind::Deadline,
            label: "dziś".into(),
            time: Some("12:00".into()),
            urgency: DateUrgency::Today,
        }]
    );
}

// --- quantity ---

#[test]
fn quantity_label_actual_over_target() {
    assert_eq!(
        quantity_label(Some(3), Some(5), Some("szt."), Locale::Pl),
        Some("3/5 szt.".into())
    );
}

#[test]
fn quantity_label_target_only() {
    assert_eq!(
        quantity_label(None, Some(5), Some("szt."), Locale::Pl),
        Some("5 szt.".into())
    );
}

#[test]
fn quantity_label_actual_only() {
    assert_eq!(
        quantity_label(Some(3), None, Some("szt."), Locale::Pl),
        Some("3 szt.".into())
    );
}

#[test]
fn quantity_label_none_when_neither_set() {
    assert_eq!(quantity_label(None, None, Some("szt."), Locale::Pl), None);
    assert_eq!(quantity_label(None, None, None, Locale::En), None);
}

#[test]
fn quantity_label_trims_unit() {
    assert_eq!(
        quantity_label(None, Some(2), Some("  kg  "), Locale::En),
        Some("2 kg".into())
    );
}

#[test]
fn quantity_label_blank_or_missing_unit_uses_locale_default() {
    assert_eq!(
        quantity_label(Some(1), Some(4), None, Locale::Pl),
        Some("1/4 szt.".into())
    );
    assert_eq!(
        quantity_label(Some(1), Some(4), Some("   "), Locale::En),
        Some("1/4 pcs".into())
    );
    assert_eq!(
        quantity_label(None, Some(4), Some(""), Locale::En),
        Some("4 pcs".into())
    );
}

#[test]
fn default_unit_per_locale() {
    assert_eq!(default_unit(Locale::Pl), "szt.");
    assert_eq!(default_unit(Locale::En), "pcs");
}
