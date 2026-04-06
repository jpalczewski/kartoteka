use std::collections::BTreeSet;

use leptos_router::params::ParamsMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionFilter {
    All,
    Open,
    Done,
}

impl CompletionFilter {
    pub fn parse(value: Option<&str>) -> Self {
        match value {
            Some("open") => Self::Open,
            Some("done") => Self::Done,
            _ => Self::All,
        }
    }

    pub fn as_query_value(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::Open => Some("open"),
            Self::Done => Some("done"),
        }
    }

    pub fn as_api_value(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::Open => Some("false"),
            Self::Done => Some("true"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRouteState {
    pub query: Option<String>,
    pub search_title: bool,
    pub search_description: bool,
    pub tag_ids: BTreeSet<String>,
    pub completed: CompletionFilter,
    pub include_archived: bool,
}

impl Default for SearchRouteState {
    fn default() -> Self {
        Self {
            query: None,
            search_title: true,
            search_description: true,
            tag_ids: BTreeSet::new(),
            completed: CompletionFilter::All,
            include_archived: false,
        }
    }
}

impl SearchRouteState {
    pub fn has_search(&self) -> bool {
        self.query.is_some() || !self.tag_ids.is_empty() || self.completed != CompletionFilter::All
    }
}

fn parse_query_set(value: Option<&str>) -> BTreeSet<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
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

pub fn search_state_from_query_map(query: &ParamsMap) -> SearchRouteState {
    search_state_from_parts(
        query.get_str("query"),
        query.get_str("search_title"),
        query.get_str("search_description"),
        query.get_str("tag_ids"),
        query.get_str("completed"),
        query.get_str("include_archived"),
    )
}

pub fn search_state_from_parts(
    query: Option<&str>,
    search_title: Option<&str>,
    search_description: Option<&str>,
    tag_ids: Option<&str>,
    completed: Option<&str>,
    include_archived: Option<&str>,
) -> SearchRouteState {
    let query_value = query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    SearchRouteState {
        query: query_value,
        search_title: search_title != Some("0"),
        search_description: search_description != Some("0"),
        tag_ids: parse_query_set(tag_ids),
        completed: CompletionFilter::parse(completed),
        include_archived: include_archived == Some("1"),
    }
}

pub fn search_query_pairs(state: &SearchRouteState) -> Vec<(&'static str, String)> {
    let mut pairs = Vec::new();
    if let Some(query) = &state.query {
        pairs.push(("query", query.clone()));
    }
    if !state.search_title {
        pairs.push(("search_title", "0".to_string()));
    }
    if !state.search_description {
        pairs.push(("search_description", "0".to_string()));
    }
    if !state.tag_ids.is_empty() {
        let tag_ids = state.tag_ids.iter().cloned().collect::<Vec<_>>().join(",");
        pairs.push(("tag_ids", tag_ids));
    }
    if let Some(completed) = state.completed.as_query_value() {
        pairs.push(("completed", completed.to_string()));
    }
    if state.include_archived {
        pairs.push(("include_archived", "1".to_string()));
    }
    pairs
}

pub fn search_href(state: &SearchRouteState) -> String {
    let query = search_query_pairs(state);
    if query.is_empty() {
        return "/search".to_string();
    }

    let query = query
        .into_iter()
        .map(|(key, value)| format!("{key}={}", encode_query_component(&value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("/search?{query}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_state_parses_query_map() {
        let state = search_state_from_parts(
            Some("milk"),
            Some("0"),
            None,
            Some("t1,t2"),
            Some("done"),
            Some("1"),
        );

        assert_eq!(state.query.as_deref(), Some("milk"));
        assert!(!state.search_title);
        assert!(state.search_description);
        assert!(state.tag_ids.contains("t1"));
        assert_eq!(state.completed, CompletionFilter::Done);
        assert!(state.include_archived);
    }

    #[test]
    fn search_href_serializes_non_default_state() {
        let mut state = SearchRouteState {
            query: Some("milk bread".into()),
            search_title: false,
            search_description: true,
            tag_ids: BTreeSet::new(),
            completed: CompletionFilter::Open,
            include_archived: true,
        };
        state.tag_ids.insert("t2".into());
        state.tag_ids.insert("t1".into());

        let href = search_href(&state);

        assert_eq!(
            href,
            "/search?query=milk%20bread&search_title=0&tag_ids=t1%2Ct2&completed=open&include_archived=1"
        );
    }
}
