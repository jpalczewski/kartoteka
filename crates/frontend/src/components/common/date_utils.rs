use chrono::NaiveDate;
use kartoteka_shared::Item;

// Re-export shared date_utils for backward compatibility
pub use kartoteka_shared::date_utils::{
    add_days, days_between, days_in_month, is_overdue_for_date_type, month_grid_range, next_month,
    parse_date, prev_month, sort_by_deadline, week_range,
};

/// Get today's date as "YYYY-MM-DD" using JS Date
pub fn get_today_string() -> String {
    let d = js_sys::Date::new_0();
    kartoteka_shared::date_utils::format_date(
        &NaiveDate::from_ymd_opt(d.get_full_year() as i32, d.get_month() + 1, d.get_date())
            .unwrap_or_default(),
    )
}

/// Alias for get_today_string
pub fn get_today() -> String {
    get_today_string()
}

/// Current time as "HH:MM" using JS Date
pub fn current_time_hhmm() -> String {
    let now = js_sys::Date::new_0();
    format!("{:02}:{:02}", now.get_hours(), now.get_minutes())
}

/// Day of week: Mon=0, ..., Sun=6 (used by polish_day_of_week and calendar grid headers)
/// This wraps the shared day_of_week (Sun=0 convention) and converts to Mon=0.
pub fn day_of_week(date_str: &str) -> u32 {
    // shared: Sun=0, Mon=1, ..., Sat=6
    // Convert to Mon=0, ..., Sun=6: (dow + 6) % 7
    let shared_dow = kartoteka_shared::date_utils::day_of_week(date_str);
    (shared_dow + 6) % 7
}

/// Check if an item is overdue, using current time from JS Date
pub fn is_overdue(item: &Item, today: &str) -> bool {
    let now_time = current_time_hhmm();
    kartoteka_shared::date_utils::is_overdue(item, today, &now_time)
}

/// Check if an item is upcoming (not completed, not overdue), using current time from JS Date
pub fn is_upcoming(item: &Item, today: &str) -> bool {
    let now_time = current_time_hhmm();
    kartoteka_shared::date_utils::is_upcoming(item, today, &now_time)
}

/// Compute relative date string in Polish
pub fn relative_date(date_str: &str, today_str: &str) -> String {
    match days_between(today_str, date_str) {
        Some(diff) => match diff {
            0 => "dzisiaj".to_string(),
            1 => "jutro".to_string(),
            -1 => "wczoraj".to_string(),
            n if n > 1 => format!("za {} dni", n),
            n => format!("{} dni temu", -n),
        },
        None => String::new(),
    }
}

pub fn polish_month_abbr(month: u32) -> &'static str {
    match month {
        1 => "sty",
        2 => "lut",
        3 => "mar",
        4 => "kwi",
        5 => "maj",
        6 => "cze",
        7 => "lip",
        8 => "sie",
        9 => "wrz",
        10 => "paź",
        11 => "lis",
        12 => "gru",
        _ => "???",
    }
}

pub fn format_date_short(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return date_str.to_string();
    }
    let day: u32 = parts[2].parse().unwrap_or(0);
    let month: u32 = parts[1].parse().unwrap_or(0);
    format!("{} {}", day, polish_month_abbr(month))
}

pub fn format_polish_date(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return date_str.to_string();
    }
    let day = parts[2].trim_start_matches('0');
    let month = match parts[1] {
        "01" => "stycznia",
        "02" => "lutego",
        "03" => "marca",
        "04" => "kwietnia",
        "05" => "maja",
        "06" => "czerwca",
        "07" => "lipca",
        "08" => "sierpnia",
        "09" => "września",
        "10" => "października",
        "11" => "listopada",
        "12" => "grudnia",
        _ => parts[1],
    };
    format!("{day} {month} {}", parts[0])
}

/// Day of week short name in Polish: Mon=0, ..., Sun=6
pub fn polish_day_of_week(dow: u32) -> &'static str {
    match dow {
        0 => "Pn",
        1 => "Wt",
        2 => "Śr",
        3 => "Cz",
        4 => "Pt",
        5 => "Sb",
        6 => "Nd",
        _ => "??",
    }
}

/// Day of week full name in Polish: Mon=0, ..., Sun=6
pub fn polish_day_of_week_full(dow: u32) -> &'static str {
    match dow {
        0 => "poniedziałek",
        1 => "wtorek",
        2 => "środa",
        3 => "czwartek",
        4 => "piątek",
        5 => "sobota",
        6 => "niedziela",
        _ => "??",
    }
}

pub fn polish_month_name(month: u32) -> &'static str {
    match month {
        1 => "Styczeń",
        2 => "Luty",
        3 => "Marzec",
        4 => "Kwiecień",
        5 => "Maj",
        6 => "Czerwiec",
        7 => "Lipiec",
        8 => "Sierpień",
        9 => "Wrzesień",
        10 => "Październik",
        11 => "Listopad",
        12 => "Grudzień",
        _ => "???",
    }
}

/// A date badge descriptor for consistent rendering across components
pub struct DateBadge {
    pub css: &'static str,
    pub label: String,
    pub date_type: &'static str,
}

/// Build date badges for an item's dates. `skip` optionally excludes one date type.
pub fn item_date_badges(item: &Item, skip: Option<&str>) -> Vec<DateBadge> {
    let mut badges = Vec::new();
    if skip != Some("start") {
        if let Some(ref d) = item.start_date {
            let time_part = item
                .start_time
                .as_ref()
                .filter(|t| !t.is_empty())
                .map(|t| format!(" {t}"))
                .unwrap_or_default();
            badges.push(DateBadge {
                css: "badge badge-info badge-sm",
                label: format!("\u{1F4C5} {}{}", format_date_short(d), time_part),
                date_type: "start",
            });
        }
    }
    if skip != Some("deadline") {
        if let Some(ref d) = item.deadline {
            let time_part = item
                .deadline_time
                .as_ref()
                .filter(|t| !t.is_empty())
                .map(|t| format!(" {t}"))
                .unwrap_or_default();
            badges.push(DateBadge {
                css: "badge badge-warning badge-sm",
                label: format!("\u{23F0} {}{}", format_date_short(d), time_part),
                date_type: "deadline",
            });
        }
    }
    if skip != Some("hard_deadline") {
        if let Some(ref d) = item.hard_deadline {
            badges.push(DateBadge {
                css: "badge badge-error badge-sm",
                label: format!("\u{1F6A8} {}", format_date_short(d)),
                date_type: "hard_deadline",
            });
        }
    }
    badges
}
