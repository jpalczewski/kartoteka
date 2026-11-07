use crate::Locale;
use crate::human_date::{DateUrgency, date_urgency, format_date_human};
use crate::types::FlexDate;
use chrono::{Datelike, Duration, NaiveDate, Weekday};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn day(d: NaiveDate) -> FlexDate {
    FlexDate::Day(d)
}

fn week(year: u16, week: u8) -> FlexDate {
    FlexDate::Week(year, week)
}

fn month(year: u16, month: u8) -> FlexDate {
    FlexDate::Month(year, month)
}

#[test]
fn fixed_today_is_a_saturday() {
    let today = d(2026, 11, 7);
    assert_eq!(today.weekday(), Weekday::Sat);
    assert_eq!(today.iso_week().year(), 2026);
    assert_eq!(today.iso_week().week(), 45);
}

#[test]
fn day_today_and_adjacent() {
    let today = d(2026, 11, 7);
    assert_eq!(format_date_human(&day(today), today, Locale::Pl), "dziś");
    assert_eq!(format_date_human(&day(today), today, Locale::En), "today");

    assert_eq!(
        format_date_human(&day(today + Duration::days(1)), today, Locale::Pl),
        "jutro"
    );
    assert_eq!(
        format_date_human(&day(today + Duration::days(1)), today, Locale::En),
        "tomorrow"
    );

    assert_eq!(
        format_date_human(&day(today - Duration::days(1)), today, Locale::Pl),
        "wczoraj"
    );
    assert_eq!(
        format_date_human(&day(today - Duration::days(1)), today, Locale::En),
        "yesterday"
    );
}

#[test]
fn day_weekdays_2_to_6() {
    // 2026-11-07 is a Saturday, so deltas 2..=6 cover Monday..Friday.
    let today = d(2026, 11, 7);
    let cases = [
        (2, "pon", "Mon"),
        (3, "wt", "Tue"),
        (4, "śr", "Wed"),
        (5, "czw", "Thu"),
        (6, "pt", "Fri"),
    ];
    for (delta, pl, en) in cases {
        let date = day(today + Duration::days(delta));
        assert_eq!(format_date_human(&date, today, Locale::Pl), pl);
        assert_eq!(format_date_human(&date, today, Locale::En), en);
    }
}

#[test]
fn day_weekdays_weekend() {
    // From a Thursday, deltas 2 and 3 land on Saturday and Sunday.
    let today = d(2026, 11, 5);
    assert_eq!(today.weekday(), Weekday::Thu);

    assert_eq!(
        format_date_human(&day(today + Duration::days(2)), today, Locale::Pl),
        "sob"
    );
    assert_eq!(
        format_date_human(&day(today + Duration::days(2)), today, Locale::En),
        "Sat"
    );
    assert_eq!(
        format_date_human(&day(today + Duration::days(3)), today, Locale::Pl),
        "nd"
    );
    assert_eq!(
        format_date_human(&day(today + Duration::days(3)), today, Locale::En),
        "Sun"
    );
}

#[test]
fn day_same_year_day_month() {
    let today = d(2026, 11, 7);

    // Past, beyond "yesterday".
    assert_eq!(
        format_date_human(&day(d(2026, 10, 1)), today, Locale::Pl),
        "1 paź"
    );
    assert_eq!(
        format_date_human(&day(d(2026, 10, 1)), today, Locale::En),
        "Oct 1"
    );

    assert_eq!(
        format_date_human(&day(d(2026, 5, 12)), today, Locale::Pl),
        "12 maj"
    );
    assert_eq!(
        format_date_human(&day(d(2026, 5, 12)), today, Locale::En),
        "May 12"
    );

    // Future, beyond a week.
    assert_eq!(
        format_date_human(&day(d(2026, 11, 20)), today, Locale::Pl),
        "20 lis"
    );
    assert_eq!(
        format_date_human(&day(d(2026, 11, 20)), today, Locale::En),
        "Nov 20"
    );
}

#[test]
fn day_different_year() {
    let today = d(2026, 11, 7);

    assert_eq!(
        format_date_human(&day(d(2027, 5, 12)), today, Locale::Pl),
        "12 maj 2027"
    );
    assert_eq!(
        format_date_human(&day(d(2027, 5, 12)), today, Locale::En),
        "May 12, 2027"
    );

    assert_eq!(
        format_date_human(&day(d(2025, 1, 3)), today, Locale::Pl),
        "3 sty 2025"
    );
    assert_eq!(
        format_date_human(&day(d(2025, 1, 3)), today, Locale::En),
        "Jan 3, 2025"
    );
}

#[test]
fn day_year_boundary_weekday_wins_over_year() {
    let today = d(2026, 12, 30);
    assert_eq!(today.weekday(), Weekday::Wed);

    // 2027-01-01 is two days away, so it renders as a weekday, not a year date.
    assert_eq!(
        format_date_human(&day(d(2027, 1, 1)), today, Locale::Pl),
        "pt"
    );
    assert_eq!(
        format_date_human(&day(d(2027, 1, 1)), today, Locale::En),
        "Fri"
    );

    // Seven days away falls through to the year-suffixed form.
    assert_eq!(
        format_date_human(&day(d(2027, 1, 6)), today, Locale::Pl),
        "6 sty 2027"
    );
    assert_eq!(
        format_date_human(&day(d(2027, 1, 6)), today, Locale::En),
        "Jan 6, 2027"
    );
}

#[test]
fn week_this_next_last() {
    let today = d(2026, 11, 7);
    assert_eq!(
        (today.iso_week().year(), today.iso_week().week()),
        (2026, 45)
    );

    let this = week(2026, 45);
    assert_eq!(format_date_human(&this, today, Locale::Pl), "ten tydzień");
    assert_eq!(format_date_human(&this, today, Locale::En), "this week");

    let next = week(2026, 46);
    assert_eq!(
        format_date_human(&next, today, Locale::Pl),
        "przyszły tydzień"
    );
    assert_eq!(format_date_human(&next, today, Locale::En), "next week");

    let last = week(2026, 44);
    assert_eq!(
        format_date_human(&last, today, Locale::Pl),
        "zeszły tydzień"
    );
    assert_eq!(format_date_human(&last, today, Locale::En), "last week");
}

#[test]
fn week_otherwise_same_and_different_iso_year() {
    let today = d(2026, 11, 7);

    let same_year = week(2026, 10);
    assert_eq!(format_date_human(&same_year, today, Locale::Pl), "tydz. 10");
    assert_eq!(format_date_human(&same_year, today, Locale::En), "Week 10");

    let other_year = week(2025, 52);
    assert_eq!(
        format_date_human(&other_year, today, Locale::Pl),
        "tydz. 52 2025"
    );
    assert_eq!(
        format_date_human(&other_year, today, Locale::En),
        "Week 52, 2025"
    );
}

#[test]
fn week_iso_year_boundary_w53_to_w01() {
    // 2026-12-30 sits inside ISO week 2026-W53; the next ISO week is 2027-W01.
    let today = d(2026, 12, 30);
    assert_eq!(
        (today.iso_week().year(), today.iso_week().week()),
        (2026, 53)
    );

    assert_eq!(
        format_date_human(&week(2026, 53), today, Locale::Pl),
        "ten tydzień"
    );
    assert_eq!(
        format_date_human(&week(2026, 53), today, Locale::En),
        "this week"
    );

    assert_eq!(
        format_date_human(&week(2027, 1), today, Locale::Pl),
        "przyszły tydzień"
    );
    assert_eq!(
        format_date_human(&week(2027, 1), today, Locale::En),
        "next week"
    );

    assert_eq!(
        format_date_human(&week(2026, 52), today, Locale::Pl),
        "zeszły tydzień"
    );
    assert_eq!(
        format_date_human(&week(2026, 52), today, Locale::En),
        "last week"
    );

    // A non-adjacent week in the next ISO year still gets its year suffix.
    assert_eq!(
        format_date_human(&week(2027, 2), today, Locale::Pl),
        "tydz. 2 2027"
    );
    assert_eq!(
        format_date_human(&week(2027, 2), today, Locale::En),
        "Week 2, 2027"
    );
}

#[test]
fn month_this_next_and_otherwise() {
    let today = d(2026, 11, 7);

    assert_eq!(
        format_date_human(&month(2026, 11), today, Locale::Pl),
        "ten miesiąc"
    );
    assert_eq!(
        format_date_human(&month(2026, 11), today, Locale::En),
        "this month"
    );

    assert_eq!(
        format_date_human(&month(2026, 12), today, Locale::Pl),
        "przyszły miesiąc"
    );
    assert_eq!(
        format_date_human(&month(2026, 12), today, Locale::En),
        "next month"
    );

    // Previous month is not special: it falls through to the full name.
    assert_eq!(
        format_date_human(&month(2026, 10), today, Locale::Pl),
        "Październik"
    );
    assert_eq!(
        format_date_human(&month(2026, 10), today, Locale::En),
        "October"
    );

    assert_eq!(
        format_date_human(&month(2026, 1), today, Locale::Pl),
        "Styczeń"
    );
    assert_eq!(
        format_date_human(&month(2026, 1), today, Locale::En),
        "January"
    );

    assert_eq!(
        format_date_human(&month(2027, 5), today, Locale::Pl),
        "Maj 2027"
    );
    assert_eq!(
        format_date_human(&month(2027, 5), today, Locale::En),
        "May 2027"
    );
}

#[test]
fn month_december_to_january_next_month() {
    let today = d(2026, 12, 15);

    assert_eq!(
        format_date_human(&month(2026, 12), today, Locale::Pl),
        "ten miesiąc"
    );
    assert_eq!(
        format_date_human(&month(2026, 12), today, Locale::En),
        "this month"
    );

    assert_eq!(
        format_date_human(&month(2027, 1), today, Locale::Pl),
        "przyszły miesiąc"
    );
    assert_eq!(
        format_date_human(&month(2027, 1), today, Locale::En),
        "next month"
    );

    // Previous month (same year) renders as a plain full name.
    assert_eq!(
        format_date_human(&month(2026, 11), today, Locale::Pl),
        "Listopad"
    );
    assert_eq!(
        format_date_human(&month(2026, 11), today, Locale::En),
        "November"
    );
}

#[test]
fn leap_day_formatting() {
    let today = d(2024, 2, 29);
    assert_eq!(today.weekday(), Weekday::Thu);

    assert_eq!(format_date_human(&day(today), today, Locale::Pl), "dziś");
    assert_eq!(format_date_human(&day(today), today, Locale::En), "today");
    assert_eq!(
        format_date_human(&day(d(2024, 3, 1)), today, Locale::Pl),
        "jutro"
    );
    assert_eq!(
        format_date_human(&day(d(2024, 3, 1)), today, Locale::En),
        "tomorrow"
    );
    assert_eq!(
        format_date_human(&day(d(2024, 2, 28)), today, Locale::Pl),
        "wczoraj"
    );
    assert_eq!(
        format_date_human(&day(d(2024, 2, 28)), today, Locale::En),
        "yesterday"
    );

    assert_eq!(
        format_date_human(&month(2024, 2), today, Locale::Pl),
        "ten miesiąc"
    );
    assert_eq!(
        format_date_human(&month(2024, 2), today, Locale::En),
        "this month"
    );

    // The leap day itself as a short month/day label.
    let earlier = d(2024, 1, 15);
    assert_eq!(
        format_date_human(&day(d(2024, 2, 29)), earlier, Locale::Pl),
        "29 lut"
    );
    assert_eq!(
        format_date_human(&day(d(2024, 2, 29)), earlier, Locale::En),
        "Feb 29"
    );
}

#[test]
fn urgency_day_variants() {
    let today = d(2026, 11, 7);

    assert_eq!(
        date_urgency(&day(today - Duration::days(1)), today, false),
        DateUrgency::Overdue
    );
    assert_eq!(date_urgency(&day(today), today, false), DateUrgency::Today);
    assert_eq!(
        date_urgency(&day(today + Duration::days(1)), today, false),
        DateUrgency::Soon
    );
    assert_eq!(
        date_urgency(&day(today + Duration::days(3)), today, false),
        DateUrgency::Soon
    );
    assert_eq!(
        date_urgency(&day(today + Duration::days(4)), today, false),
        DateUrgency::Later
    );
}

#[test]
fn urgency_completed_is_none() {
    let today = d(2026, 11, 7);
    assert_eq!(
        date_urgency(&day(today - Duration::days(1)), today, true),
        DateUrgency::None
    );
    assert_eq!(date_urgency(&day(today), today, true), DateUrgency::None);
    assert_eq!(
        date_urgency(&week(2026, 40), today, true),
        DateUrgency::None
    );
    assert_eq!(
        date_urgency(&month(2026, 10), today, true),
        DateUrgency::None
    );
}

#[test]
fn urgency_fuzzy_variants() {
    let today = d(2026, 11, 7);

    // A past month ends before today.
    assert_eq!(
        date_urgency(&month(2026, 10), today, false),
        DateUrgency::Overdue
    );

    // The current month and current week contain today → Soon, never Today.
    assert_eq!(
        date_urgency(&month(2026, 11), today, false),
        DateUrgency::Soon
    );
    assert_eq!(
        date_urgency(&week(2026, 45), today, false),
        DateUrgency::Soon
    );

    // Fuzzy dates entirely in the future (even close ones) are Later.
    assert_eq!(
        date_urgency(&month(2026, 12), today, false),
        DateUrgency::Later
    );
    assert_eq!(
        date_urgency(&week(2026, 46), today, false),
        DateUrgency::Later
    );
    assert_eq!(
        date_urgency(&month(2027, 1), today, false),
        DateUrgency::Later
    );
}

#[test]
fn out_of_range_values_fall_back_without_panicking() {
    let today = NaiveDate::from_ymd_opt(2026, 11, 7).unwrap();
    for (date, raw) in [
        (FlexDate::Month(2026, 13), "2026-13"),
        (FlexDate::Month(2026, 0), "2026-00"),
        (FlexDate::Week(2026, 54), "2026-W54"),
        (FlexDate::Week(2026, 0), "2026-W00"),
    ] {
        assert_eq!(format_date_human(&date, today, Locale::Pl), raw);
        assert_eq!(date_urgency(&date, today, false), DateUrgency::None);
    }
}
