use chrono::{Datelike, Duration, NaiveDate};

/// Parse "YYYY-MM-DD" string into NaiveDate
pub fn parse_date(date_str: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
}

/// Format NaiveDate as "YYYY-MM-DD"
pub fn format_date(date: &NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Add/subtract days from a date string
pub fn add_days(date_str: &str, n: i64) -> String {
    parse_date(date_str)
        .map(|d| format_date(&(d + Duration::days(n))))
        .unwrap_or_default()
}

/// Day of week: Sun=0, Mon=1, ..., Sat=6 (JS convention for calendar grids)
pub fn day_of_week(date_str: &str) -> u32 {
    parse_date(date_str)
        .map(|d| d.weekday().num_days_from_sunday())
        .unwrap_or(0)
}

/// Signed difference in days between two date strings
pub fn days_between(a: &str, b: &str) -> Option<i64> {
    let da = parse_date(a)?;
    let db = parse_date(b)?;
    Some((db - da).num_days())
}

/// Days in a given month
pub fn days_in_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .and_then(|d| d.pred_opt())
    .map(|d| d.day())
    .unwrap_or(30)
}

/// Mon-Sun week range containing the given date
pub fn week_range(date_str: &str) -> (String, String) {
    let date = parse_date(date_str).unwrap_or_default();
    let days_since_monday = date.weekday().num_days_from_monday();
    let monday = date - Duration::days(days_since_monday as i64);
    let sunday = monday + Duration::days(6);
    (format_date(&monday), format_date(&sunday))
}

/// Grid range for a month calendar (starts Monday, ends Sunday, covers full weeks)
pub fn month_grid_range(year: i32, month: u32) -> (String, String) {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_default();
    let last_day = days_in_month(year, month);
    let last = NaiveDate::from_ymd_opt(year, month, last_day).unwrap_or_default();

    let start_offset = first.weekday().num_days_from_monday();
    let grid_start = first - Duration::days(start_offset as i64);

    let end_offset = 6 - last.weekday().num_days_from_monday();
    let grid_end = last + Duration::days(end_offset as i64);

    (format_date(&grid_start), format_date(&grid_end))
}

/// Previous month (year, month)
pub fn prev_month(year: i32, month: u32) -> (i32, u32) {
    if month == 1 { (year - 1, 12) } else { (year, month - 1) }
}

/// Next month (year, month)
pub fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 { (year + 1, 1) } else { (year, month + 1) }
}

/// Check if an item's deadline is overdue
pub fn is_overdue(item: &crate::Item, today: &str, now_time: &str) -> bool {
    if item.completed {
        return false;
    }
    match item.deadline.as_deref() {
        None => false,
        Some(d) if d < today => true,
        Some(d) if d == today => match item.deadline_time.as_deref() {
            Some(t) if !t.is_empty() => t < now_time,
            _ => false,
        },
        _ => false,
    }
}

/// Check overdue for a specific date field (date-only, no time)
pub fn is_overdue_for_date_type(date_val: Option<&str>, completed: bool, today: &str) -> bool {
    if completed {
        return false;
    }
    match date_val {
        None => false,
        Some(d) if d < today => true,
        _ => false,
    }
}

/// Check if item is upcoming (not completed, not overdue)
pub fn is_upcoming(item: &crate::Item, today: &str, now_time: &str) -> bool {
    !item.completed && !is_overdue(item, today, now_time)
}

/// Sort items by deadline (earliest first, items without deadline last)
pub fn sort_by_deadline(items: &mut [crate::Item]) {
    items.sort_by(|a, b| {
        match (&a.deadline, &b.deadline) {
            (Some(da), Some(db)) => da.cmp(db),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        assert_eq!(parse_date("2026-03-29"), Some(NaiveDate::from_ymd_opt(2026, 3, 29).unwrap()));
        assert_eq!(parse_date("invalid"), None);
        assert_eq!(parse_date(""), None);
    }

    #[test]
    fn test_format_date() {
        let d = NaiveDate::from_ymd_opt(2026, 3, 5).unwrap();
        assert_eq!(format_date(&d), "2026-03-05");
    }

    #[test]
    fn test_add_days() {
        assert_eq!(add_days("2026-03-29", 1), "2026-03-30");
        assert_eq!(add_days("2026-03-31", 1), "2026-04-01");
        assert_eq!(add_days("2026-01-01", -1), "2025-12-31");
        assert_eq!(add_days("2026-02-28", 1), "2026-03-01");
    }

    #[test]
    fn test_add_days_leap_year() {
        assert_eq!(add_days("2024-02-28", 1), "2024-02-29");
        assert_eq!(add_days("2024-02-29", 1), "2024-03-01");
    }

    #[test]
    fn test_day_of_week() {
        // Sun=0, Mon=1, ..., Sat=6 (JS convention for calendar grids)
        assert_eq!(day_of_week("2026-03-29"), 0); // Sunday
        assert_eq!(day_of_week("2026-03-23"), 1); // Monday
        assert_eq!(day_of_week("2026-03-28"), 6); // Saturday
    }

    #[test]
    fn test_days_between() {
        assert_eq!(days_between("2026-03-29", "2026-03-29"), Some(0));
        assert_eq!(days_between("2026-03-29", "2026-03-30"), Some(1));
        assert_eq!(days_between("2026-03-30", "2026-03-29"), Some(-1));
    }

    #[test]
    fn test_week_range() {
        // 2026-03-29 is Sunday, week should be Mon 2026-03-23 to Sun 2026-03-29
        let (start, end) = week_range("2026-03-29");
        assert_eq!(start, "2026-03-23");
        assert_eq!(end, "2026-03-29");
    }

    #[test]
    fn test_week_range_monday() {
        let (start, end) = week_range("2026-03-23");
        assert_eq!(start, "2026-03-23");
        assert_eq!(end, "2026-03-29");
    }

    #[test]
    fn test_month_grid_range() {
        // March 2026: starts on Sunday, so grid starts Mon Feb 23
        let (start, end) = month_grid_range(2026, 3);
        let start_date = parse_date(&start).unwrap();
        let end_date = parse_date(&end).unwrap();
        assert_eq!(start_date.weekday(), chrono::Weekday::Mon);
        assert_eq!(end_date.weekday(), chrono::Weekday::Sun);
    }

    #[test]
    fn test_is_overdue_past_deadline() {
        let item = crate::Item {
            id: "1".into(), list_id: "l1".into(), title: "t".into(),
            description: None, completed: false, position: 0,
            quantity: None, actual_quantity: None, unit: None,
            start_date: None, start_time: None,
            deadline: Some("2026-03-28".into()), deadline_time: None,
            hard_deadline: None,
            created_at: "2026-03-01".into(), updated_at: "2026-03-01".into(),
        };
        assert!(is_overdue(&item, "2026-03-29", "12:00"));
    }

    #[test]
    fn test_is_overdue_same_day_with_time() {
        let item = crate::Item {
            id: "1".into(), list_id: "l1".into(), title: "t".into(),
            description: None, completed: false, position: 0,
            quantity: None, actual_quantity: None, unit: None,
            start_date: None, start_time: None,
            deadline: Some("2026-03-29".into()), deadline_time: Some("10:00".into()),
            hard_deadline: None,
            created_at: "2026-03-01".into(), updated_at: "2026-03-01".into(),
        };
        assert!(is_overdue(&item, "2026-03-29", "12:00"));
        assert!(!is_overdue(&item, "2026-03-29", "09:00"));
    }

    #[test]
    fn test_is_overdue_completed_not_overdue() {
        let item = crate::Item {
            id: "1".into(), list_id: "l1".into(), title: "t".into(),
            description: None, completed: true, position: 0,
            quantity: None, actual_quantity: None, unit: None,
            start_date: None, start_time: None,
            deadline: Some("2026-03-28".into()), deadline_time: None,
            hard_deadline: None,
            created_at: "2026-03-01".into(), updated_at: "2026-03-01".into(),
        };
        assert!(!is_overdue(&item, "2026-03-29", "12:00"));
    }

    #[test]
    fn test_is_overdue_for_date_type() {
        assert!(is_overdue_for_date_type(Some("2026-03-28"), false, "2026-03-29"));
        assert!(!is_overdue_for_date_type(Some("2026-03-28"), true, "2026-03-29")); // completed
        assert!(!is_overdue_for_date_type(Some("2026-03-30"), false, "2026-03-29")); // future
        assert!(!is_overdue_for_date_type(None, false, "2026-03-29")); // no date
    }

    #[test]
    fn test_is_upcoming() {
        let item = crate::Item {
            id: "1".into(), list_id: "l1".into(), title: "t".into(),
            description: None, completed: false, position: 0,
            quantity: None, actual_quantity: None, unit: None,
            start_date: None, start_time: None,
            deadline: Some("2026-03-30".into()), deadline_time: None,
            hard_deadline: None,
            created_at: "2026-03-01".into(), updated_at: "2026-03-01".into(),
        };
        assert!(is_upcoming(&item, "2026-03-29", "12:00"));
    }

    #[test]
    fn test_sort_by_deadline() {
        let mut items = vec![
            crate::Item {
                id: "1".into(), list_id: "l".into(), title: "no deadline".into(),
                description: None, completed: false, position: 0,
                quantity: None, actual_quantity: None, unit: None,
                start_date: None, start_time: None,
                deadline: None, deadline_time: None, hard_deadline: None,
                created_at: "".into(), updated_at: "".into(),
            },
            crate::Item {
                id: "2".into(), list_id: "l".into(), title: "early".into(),
                description: None, completed: false, position: 0,
                quantity: None, actual_quantity: None, unit: None,
                start_date: None, start_time: None,
                deadline: Some("2026-03-01".into()), deadline_time: None, hard_deadline: None,
                created_at: "".into(), updated_at: "".into(),
            },
            crate::Item {
                id: "3".into(), list_id: "l".into(), title: "late".into(),
                description: None, completed: false, position: 0,
                quantity: None, actual_quantity: None, unit: None,
                start_date: None, start_time: None,
                deadline: Some("2026-03-15".into()), deadline_time: None, hard_deadline: None,
                created_at: "".into(), updated_at: "".into(),
            },
        ];
        sort_by_deadline(&mut items);
        assert_eq!(items[0].id, "2"); // earliest deadline first
        assert_eq!(items[1].id, "3");
        assert_eq!(items[2].id, "1"); // no deadline last
    }

    #[test]
    fn test_prev_next_month() {
        assert_eq!(prev_month(2026, 3), (2026, 2));
        assert_eq!(prev_month(2026, 1), (2025, 12));
        assert_eq!(next_month(2026, 12), (2027, 1));
        assert_eq!(next_month(2026, 3), (2026, 4));
    }
}
