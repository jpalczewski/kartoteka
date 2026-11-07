//! Human-readable, locale-aware date labels for item chips.
//!
//! All functions are pure: `today` is always passed in, so this module is
//! WASM-safe (no system clock, no `Local::now()`).

use crate::locale::Locale;
use crate::types::FlexDate;
use chrono::{Datelike, Duration, NaiveDate};

const PL_SHORT_MONTHS: [&str; 12] = [
    "sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru",
];
const EN_SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const PL_FULL_MONTHS: [&str; 12] = [
    "Styczeń",
    "Luty",
    "Marzec",
    "Kwiecień",
    "Maj",
    "Czerwiec",
    "Lipiec",
    "Sierpień",
    "Wrzesień",
    "Październik",
    "Listopad",
    "Grudzień",
];
const EN_FULL_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const PL_WEEKDAYS: [&str; 7] = ["pon", "wt", "śr", "czw", "pt", "sob", "nd"];
const EN_WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Short human label for a (possibly fuzzy) date relative to `today`, for chips in item rows.
pub fn format_date_human(date: &FlexDate, today: NaiveDate, locale: Locale) -> String {
    match date {
        FlexDate::Day(d) => format_day(*d, today, locale),
        FlexDate::Week(year, week) => format_week(*year, *week, today, locale),
        FlexDate::Month(year, month) => format_month(*year, *month, today, locale),
    }
}

/// Relative urgency for styling chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateUrgency {
    /// The date is already in the past.
    Overdue,
    /// The date is exactly today.
    Today,
    /// Due soon (within the next three days, or the current fuzzy period).
    Soon,
    /// Due later.
    Later,
    /// No urgency (the item is completed).
    None,
}

/// Classify a date's urgency relative to `today`, for styling chips.
pub fn date_urgency(date: &FlexDate, today: NaiveDate, completed: bool) -> DateUrgency {
    if completed {
        return DateUrgency::None;
    }

    match date {
        FlexDate::Day(d) => {
            if *d < today {
                DateUrgency::Overdue
            } else if *d == today {
                DateUrgency::Today
            } else if *d <= today + Duration::days(3) {
                DateUrgency::Soon
            } else {
                DateUrgency::Later
            }
        }
        FlexDate::Week(..) | FlexDate::Month(..) => {
            if date.end() < today {
                DateUrgency::Overdue
            } else if today >= date.start() && today <= date.end() {
                DateUrgency::Soon
            } else {
                DateUrgency::Later
            }
        }
    }
}

fn relative(pl: &str, en: &str, locale: Locale) -> String {
    match locale {
        Locale::Pl => pl.to_string(),
        Locale::En => en.to_string(),
    }
}

fn weekday_name(idx: usize, locale: Locale) -> &'static str {
    match locale {
        Locale::Pl => PL_WEEKDAYS[idx],
        Locale::En => EN_WEEKDAYS[idx],
    }
}

fn format_day(d: NaiveDate, today: NaiveDate, locale: Locale) -> String {
    let delta = (d - today).num_days();
    match delta {
        0 => return relative("dziś", "today", locale),
        1 => return relative("jutro", "tomorrow", locale),
        -1 => return relative("wczoraj", "yesterday", locale),
        2..=6 => {
            let idx = d.weekday().num_days_from_monday() as usize;
            return weekday_name(idx, locale).to_string();
        }
        _ => {}
    }

    let month = (d.month() - 1) as usize;
    let same_year = d.year() == today.year();
    match locale {
        Locale::Pl => {
            if same_year {
                format!("{} {}", d.day(), PL_SHORT_MONTHS[month])
            } else {
                format!("{} {} {}", d.day(), PL_SHORT_MONTHS[month], d.year())
            }
        }
        Locale::En => {
            if same_year {
                format!("{} {}", EN_SHORT_MONTHS[month], d.day())
            } else {
                format!("{} {}, {}", EN_SHORT_MONTHS[month], d.day(), d.year())
            }
        }
    }
}

fn format_week(year: u16, week: u8, today: NaiveDate, locale: Locale) -> String {
    let date = FlexDate::Week(year, week);
    let today_monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    let week_diff = (date.start() - today_monday).num_days() / 7;

    match week_diff {
        0 => return relative("ten tydzień", "this week", locale),
        1 => return relative("przyszły tydzień", "next week", locale),
        -1 => return relative("zeszły tydzień", "last week", locale),
        _ => {}
    }

    let same_iso_year = i32::from(year) == today.iso_week().year();
    match locale {
        Locale::Pl => {
            if same_iso_year {
                format!("tydz. {week}")
            } else {
                format!("tydz. {week} {year}")
            }
        }
        Locale::En => {
            if same_iso_year {
                format!("Week {week}")
            } else {
                format!("Week {week}, {year}")
            }
        }
    }
}

fn format_month(year: u16, month: u8, today: NaiveDate, locale: Locale) -> String {
    if i32::from(year) == today.year() && u32::from(month) == today.month() {
        return relative("ten miesiąc", "this month", locale);
    }

    let (next_year, next_month) = if today.month() == 12 {
        (today.year() + 1, 1)
    } else {
        (today.year(), today.month() + 1)
    };
    if i32::from(year) == next_year && u32::from(month) == next_month {
        return relative("przyszły miesiąc", "next month", locale);
    }

    let idx = (month - 1) as usize;
    let same_year = i32::from(year) == today.year();
    match locale {
        Locale::Pl => {
            if same_year {
                PL_FULL_MONTHS[idx].to_string()
            } else {
                format!("{} {}", PL_FULL_MONTHS[idx], year)
            }
        }
        Locale::En => {
            if same_year {
                EN_FULL_MONTHS[idx].to_string()
            } else {
                format!("{} {}", EN_FULL_MONTHS[idx], year)
            }
        }
    }
}
