use std::collections::HashSet;

use chrono::{Datelike, NaiveDate};
use leptos_router::params::ParamsMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Month,
    Week,
}

impl ViewMode {
    pub fn parse(value: &str) -> Self {
        match value {
            "week" => Self::Week,
            _ => Self::Month,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Month => "month",
            Self::Week => "week",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarRouteState {
    pub selected_date: String,
    pub view_mode: ViewMode,
    pub hidden_lists: HashSet<String>,
    pub hidden_tags: HashSet<String>,
    pub show_completed: bool,
}

impl CalendarRouteState {
    pub fn new(selected_date: String) -> Self {
        Self {
            selected_date,
            view_mode: ViewMode::Month,
            hidden_lists: HashSet::new(),
            hidden_tags: HashSet::new(),
            show_completed: true,
        }
    }
}

pub fn parse_calendar_date_parts(year: &str, month: &str, day: &str) -> Option<String> {
    let year: i32 = year.parse().ok()?;
    let month: u32 = month.parse().ok()?;
    let day: u32 = day.parse().ok()?;
    NaiveDate::from_ymd_opt(year, month, day).map(|date| date.format("%Y-%m-%d").to_string())
}

fn parse_query_set(value: Option<&str>) -> HashSet<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn calendar_state_from_parts(
    selected_date: Option<String>,
    view: Option<&str>,
    hidden_lists: Option<&str>,
    hidden_tags: Option<&str>,
    show_completed: Option<&str>,
    today: &str,
) -> CalendarRouteState {
    let selected_date = selected_date.unwrap_or_else(|| today.to_string());

    CalendarRouteState {
        selected_date,
        view_mode: ViewMode::parse(view.unwrap_or("month")),
        hidden_lists: parse_query_set(hidden_lists),
        hidden_tags: parse_query_set(hidden_tags),
        show_completed: show_completed.unwrap_or("1") != "0",
    }
}

pub fn calendar_state_from_maps(
    params: &ParamsMap,
    query: &ParamsMap,
    today: &str,
) -> CalendarRouteState {
    calendar_state_from_parts(
        parse_calendar_date_parts(
            params.get_str("year").unwrap_or_default(),
            params.get_str("month").unwrap_or_default(),
            params.get_str("day").unwrap_or_default(),
        ),
        query.get_str("view"),
        query.get_str("hidden_lists"),
        query.get_str("hidden_tags"),
        query.get_str("show_completed"),
        today,
    )
}

fn serialize_query_set(set: &HashSet<String>) -> Option<String> {
    if set.is_empty() {
        return None;
    }

    let mut values: Vec<_> = set.iter().cloned().collect();
    values.sort();
    Some(values.join(","))
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

pub fn calendar_query_pairs(state: &CalendarRouteState) -> Vec<(&'static str, String)> {
    let mut pairs = vec![("view", state.view_mode.as_str().to_string())];
    if !state.show_completed {
        pairs.push(("show_completed", "0".to_string()));
    }
    if let Some(hidden_lists) = serialize_query_set(&state.hidden_lists) {
        pairs.push(("hidden_lists", hidden_lists));
    }
    if let Some(hidden_tags) = serialize_query_set(&state.hidden_tags) {
        pairs.push(("hidden_tags", hidden_tags));
    }
    pairs
}

pub fn calendar_href(state: &CalendarRouteState) -> String {
    let mut path = "/calendar".to_string();
    let Ok(date) = NaiveDate::parse_from_str(&state.selected_date, "%Y-%m-%d") else {
        return path;
    };

    path.push_str(&format!(
        "/{:04}/{:02}/{:02}",
        date.year(),
        date.month(),
        date.day()
    ));

    let query = calendar_query_pairs(state);
    if query.is_empty() {
        return path;
    }

    let query = query
        .into_iter()
        .map(|(key, value)| format!("{key}={}", encode_query_component(&value)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{path}?{query}")
}

pub fn calendar_state_with_selected_date(
    state: &CalendarRouteState,
    selected_date: String,
) -> CalendarRouteState {
    CalendarRouteState {
        selected_date,
        ..state.clone()
    }
}

pub fn calendar_state_with_view_mode(
    state: &CalendarRouteState,
    view_mode: ViewMode,
) -> CalendarRouteState {
    CalendarRouteState {
        view_mode,
        ..state.clone()
    }
}

pub fn calendar_state_toggle_hidden_list(
    state: &CalendarRouteState,
    list_id: &str,
) -> CalendarRouteState {
    let mut next = state.clone();
    if !next.hidden_lists.remove(list_id) {
        next.hidden_lists.insert(list_id.to_string());
    }
    next
}

pub fn calendar_state_toggle_hidden_tag(
    state: &CalendarRouteState,
    tag_id: &str,
) -> CalendarRouteState {
    let mut next = state.clone();
    if !next.hidden_tags.remove(tag_id) {
        next.hidden_tags.insert(tag_id.to_string());
    }
    next
}

pub fn calendar_state_toggle_show_completed(state: &CalendarRouteState) -> CalendarRouteState {
    CalendarRouteState {
        show_completed: !state.show_completed,
        ..state.clone()
    }
}

pub fn shift_month_clamped(date: &str, month_delta: i32) -> String {
    let Ok(parsed) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return date.to_string();
    };

    let (year, month) = if month_delta < 0 {
        if parsed.month() == 1 {
            (parsed.year() - 1, 12)
        } else {
            (parsed.year(), parsed.month() - 1)
        }
    } else if parsed.month() == 12 {
        (parsed.year() + 1, 1)
    } else {
        (parsed.year(), parsed.month() + 1)
    };

    let clamped_day = parsed.day().min(days_in_month(year, month));
    format!("{year:04}-{month:02}-{clamped_day:02}")
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    let current_month = NaiveDate::from_ymd_opt(year, month, 1);

    match (current_month, next_month) {
        (Some(_), Some(next)) => (next - chrono::Days::new(1)).day(),
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_state_from_parts_parses_route_and_filters() {
        let state = calendar_state_from_parts(
            Some("2026-04-12".into()),
            Some("week"),
            Some("b,a"),
            Some("tag-2,tag-1"),
            Some("0"),
            "2026-04-04",
        );

        assert_eq!(state.selected_date, "2026-04-12");
        assert_eq!(state.view_mode, ViewMode::Week);
        assert_eq!(state.hidden_lists.len(), 2);
        assert!(state.hidden_lists.contains("a"));
        assert!(state.hidden_tags.contains("tag-1"));
        assert!(!state.show_completed);
    }

    #[test]
    fn calendar_href_serializes_canonical_route() {
        let mut state = CalendarRouteState::new("2026-04-12".into());
        state.view_mode = ViewMode::Week;
        state.show_completed = false;
        state.hidden_lists = HashSet::from(["list-b".into(), "list-a".into()]);

        assert_eq!(
            calendar_href(&state),
            "/calendar/2026/04/12?view=week&show_completed=0&hidden_lists=list-a%2Clist-b"
        );
    }

    #[test]
    fn shift_month_clamped_keeps_day_when_possible() {
        assert_eq!(shift_month_clamped("2026-03-31", 1), "2026-04-30");
        assert_eq!(shift_month_clamped("2026-05-30", -1), "2026-04-30");
    }

    #[test]
    fn calendar_state_toggle_hidden_list_adds_and_removes_id() {
        let state = CalendarRouteState::new("2026-04-12".into());
        let hidden = calendar_state_toggle_hidden_list(&state, "list-a");
        let visible = calendar_state_toggle_hidden_list(&hidden, "list-a");

        assert!(hidden.hidden_lists.contains("list-a"));
        assert!(!visible.hidden_lists.contains("list-a"));
        assert_eq!(visible.selected_date, state.selected_date);
    }

    #[test]
    fn calendar_state_toggle_hidden_tag_adds_and_removes_id() {
        let state = CalendarRouteState::new("2026-04-12".into());
        let hidden = calendar_state_toggle_hidden_tag(&state, "tag-a");
        let visible = calendar_state_toggle_hidden_tag(&hidden, "tag-a");

        assert!(hidden.hidden_tags.contains("tag-a"));
        assert!(!visible.hidden_tags.contains("tag-a"));
        assert_eq!(visible.view_mode, state.view_mode);
    }

    #[test]
    fn calendar_state_toggle_show_completed_preserves_other_fields() {
        let mut state = CalendarRouteState::new("2026-04-12".into());
        state.view_mode = ViewMode::Week;
        state.hidden_lists.insert("list-a".into());
        state.hidden_tags.insert("tag-a".into());

        let next = calendar_state_toggle_show_completed(&state);

        assert!(!next.show_completed);
        assert_eq!(next.selected_date, state.selected_date);
        assert_eq!(next.view_mode, state.view_mode);
        assert_eq!(next.hidden_lists, state.hidden_lists);
        assert_eq!(next.hidden_tags, state.hidden_tags);
    }

    #[test]
    fn calendar_state_with_selected_date_preserves_filters_and_view_mode() {
        let mut state = CalendarRouteState::new("2026-04-12".into());
        state.view_mode = ViewMode::Week;
        state.hidden_lists.insert("list-a".into());
        state.hidden_tags.insert("tag-a".into());
        state.show_completed = false;

        let next = calendar_state_with_selected_date(&state, "2026-04-15".into());

        assert_eq!(next.selected_date, "2026-04-15");
        assert_eq!(next.view_mode, ViewMode::Week);
        assert_eq!(next.hidden_lists, state.hidden_lists);
        assert_eq!(next.hidden_tags, state.hidden_tags);
        assert!(!next.show_completed);
    }
}
