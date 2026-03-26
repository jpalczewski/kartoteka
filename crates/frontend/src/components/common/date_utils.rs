use kartoteka_shared::Item;

pub fn get_today_string() -> String {
    let today = js_sys::Date::new_0();
    let year = today.get_full_year() as i32;
    let month = today.get_month() + 1;
    let day = today.get_date();
    format!("{:04}-{:02}-{:02}", year, month, day)
}

pub fn get_today() -> String {
    get_today_string()
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

pub fn days_in_month(year: i32, month: u32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

pub fn date_to_days(date_str: &str) -> Option<i32> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: i32 = parts[2].parse().ok()?;

    let mut total: i32 = 0;
    for yr in 0..y {
        if (yr % 4 == 0 && yr % 100 != 0) || yr % 400 == 0 {
            total += 366;
        } else {
            total += 365;
        }
    }
    for mo in 1..m {
        total += days_in_month(y, mo);
    }
    total += d;
    Some(total)
}

pub fn relative_date(date_str: &str, today_str: &str) -> String {
    let date_days = date_to_days(date_str);
    let today_days = date_to_days(today_str);
    match (date_days, today_days) {
        (Some(d), Some(t)) => {
            let diff = d - t;
            match diff {
                0 => "dzisiaj".to_string(),
                1 => "jutro".to_string(),
                -1 => "wczoraj".to_string(),
                n if n > 1 => format!("za {} dni", n),
                n => format!("{} dni temu", -n),
            }
        }
        _ => String::new(),
    }
}

pub fn current_time_hhmm() -> String {
    let now = js_sys::Date::new_0();
    format!("{:02}:{:02}", now.get_hours(), now.get_minutes())
}

pub fn is_overdue(item: &Item, today: &str) -> bool {
    if item.completed {
        return false;
    }
    match item.due_date.as_deref() {
        None => false,
        Some(d) if d < today => true,
        Some(d) if d == today => match item.due_time.as_deref() {
            Some(t) if !t.is_empty() => t < current_time_hhmm().as_str(),
            _ => false,
        },
        _ => false,
    }
}

pub fn is_upcoming(item: &Item, today: &str) -> bool {
    !item.completed && !is_overdue(item, today)
}

pub fn sort_by_due_date(items: &mut [Item]) {
    items.sort_by(|a, b| {
        let da = a.due_date.as_deref().unwrap_or("9999-99-99");
        let db = b.due_date.as_deref().unwrap_or("9999-99-99");
        da.cmp(db)
    });
}

// === Calendar navigation utilities ===

pub fn parse_date(date_str: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    Some((y, m, d))
}

fn format_ymd(y: i32, m: u32, d: u32) -> String {
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Day of week: 0=Mon, 6=Sun
/// Uses Tomohiko Sakamoto's algorithm
pub fn day_of_week(date_str: &str) -> u32 {
    let (mut y, m, d) = parse_date(date_str).unwrap_or((2026, 1, 1));
    let t = [0i32, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    if m < 3 {
        y -= 1;
    }
    let dow = ((y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + d as i32) % 7 + 7) % 7;
    // dow: 0=Sun, 1=Mon, ..., 6=Sat → convert to 0=Mon, 6=Sun
    ((dow + 6) % 7) as u32
}

/// Add (or subtract) days from a date string
pub fn add_days(date_str: &str, n: i32) -> String {
    let (mut y, mut m, mut d) = parse_date(date_str).unwrap_or((2026, 1, 1));

    if n >= 0 {
        for _ in 0..n {
            d += 1;
            let dim = days_in_month(y, m) as u32;
            if d > dim {
                d = 1;
                m += 1;
                if m > 12 {
                    m = 1;
                    y += 1;
                }
            }
        }
    } else {
        for _ in 0..(-n) {
            if d == 1 {
                if m == 1 {
                    m = 12;
                    y -= 1;
                } else {
                    m -= 1;
                }
                d = days_in_month(y, m) as u32;
            } else {
                d -= 1;
            }
        }
    }

    format_ymd(y, m, d)
}

/// Returns (first_monday, last_sunday) for the calendar grid of a given month
pub fn month_grid_range(year: i32, month: u32) -> (String, String) {
    let first_day = format_ymd(year, month, 1);
    let dow = day_of_week(&first_day);
    let first_monday = add_days(&first_day, -(dow as i32));

    let dim = days_in_month(year, month) as u32;
    let last_day = format_ymd(year, month, dim);
    let last_dow = day_of_week(&last_day);
    let last_sunday = add_days(&last_day, (6 - last_dow) as i32);

    (first_monday, last_sunday)
}

/// Returns (monday, sunday) of the week containing the given date
pub fn week_range(date_str: &str) -> (String, String) {
    let dow = day_of_week(date_str);
    let monday = add_days(date_str, -(dow as i32));
    let sunday = add_days(date_str, (6 - dow) as i32);
    (monday, sunday)
}

pub fn prev_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    }
}

pub fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

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
