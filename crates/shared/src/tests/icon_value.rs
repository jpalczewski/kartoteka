use crate::icon_value::{IconValue, default_container_icon, default_list_icon, parse_icon_value};

#[test]
fn parses_lucide_names() {
    assert_eq!(
        parse_icon_value(Some("lucide:square-check")),
        IconValue::Lucide("square-check")
    );
    assert_eq!(
        parse_icon_value(Some("lucide:list")),
        IconValue::Lucide("list")
    );
    assert_eq!(parse_icon_value(Some("lucide:x2")), IconValue::Lucide("x2"));
    assert_eq!(
        parse_icon_value(Some("  lucide:folder-kanban  ")),
        IconValue::Lucide("folder-kanban")
    );
}

#[test]
fn rejects_malformed_lucide_names() {
    assert_eq!(parse_icon_value(Some("lucide:")), IconValue::None);
    assert_eq!(parse_icon_value(Some("lucide:  ")), IconValue::None);
    assert_eq!(
        parse_icon_value(Some("lucide:Square-Check")),
        IconValue::None
    );
    assert_eq!(parse_icon_value(Some("lucide:foo_bar")), IconValue::None);
    assert_eq!(parse_icon_value(Some("lucide:foo bar")), IconValue::None);
    assert_eq!(parse_icon_value(Some("lucide:📦")), IconValue::None);
}

#[test]
fn parses_explicit_emoji() {
    assert_eq!(parse_icon_value(Some("emoji:📦")), IconValue::Emoji("📦"));
    assert_eq!(parse_icon_value(Some("emoji: ⏱️ ")), IconValue::Emoji("⏱️"));
    assert_eq!(parse_icon_value(Some("emoji:")), IconValue::None);
    assert_eq!(parse_icon_value(Some("emoji:   ")), IconValue::None);
}

#[test]
fn legacy_bare_emoji_is_emoji() {
    assert_eq!(parse_icon_value(Some("📦")), IconValue::Emoji("📦"));
    assert_eq!(parse_icon_value(Some("  🚀  ")), IconValue::Emoji("🚀"));
    // Bare words are still treated as emoji — legacy data may contain anything.
    assert_eq!(parse_icon_value(Some("folder")), IconValue::Emoji("folder"));
}

#[test]
fn none_and_empty_are_none() {
    assert_eq!(parse_icon_value(None), IconValue::None);
    assert_eq!(parse_icon_value(Some("")), IconValue::None);
    assert_eq!(parse_icon_value(Some("   ")), IconValue::None);
}

#[test]
fn default_list_icons() {
    assert_eq!(default_list_icon("checklist"), "square-check");
    assert_eq!(default_list_icon("shopping"), "shopping-cart");
    assert_eq!(default_list_icon("schedule"), "calendar");
    assert_eq!(default_list_icon("habits"), "calendar");
    assert_eq!(default_list_icon("log"), "timer");
    assert_eq!(default_list_icon("notes"), "list");
    assert_eq!(default_list_icon("zakupy"), "shopping-cart");
    assert_eq!(default_list_icon("pakowanie"), "luggage");
    assert_eq!(default_list_icon("terminarz"), "calendar");
    assert_eq!(default_list_icon(""), "list");
    assert_eq!(default_list_icon("unknown"), "list");
}

#[test]
fn default_container_icons() {
    assert_eq!(default_container_icon(false), "folder");
    assert_eq!(default_container_icon(true), "folder-kanban");
}
