use kartoteka_shared::{IconValue, parse_icon_value};
use leptos::prelude::*;

/// A lucide icon name, accepted from `&'static str`, `String` or `Signal<String>`.
///
/// `#[prop(into)] name: IconName` keeps call sites terse (`name="trash"`) while
/// still allowing reactive names (`name=move || ...`).
#[derive(Clone, Debug)]
pub struct IconName(pub Signal<String>);

impl From<&'static str> for IconName {
    fn from(name: &'static str) -> Self {
        Self(Signal::derive(move || name.to_string()))
    }
}

impl From<String> for IconName {
    fn from(name: String) -> Self {
        Self(Signal::derive(move || name.clone()))
    }
}

impl From<Signal<String>> for IconName {
    fn from(name: Signal<String>) -> Self {
        Self(name)
    }
}

/// SVG path data from Lucide (https://lucide.dev), ISC License,
/// Copyright (c) Lucide Contributors.
///
/// Each value is the inner markup of the corresponding Lucide icon's `<svg>`
/// element, copied verbatim from lucide-leptos 3.63.0. Only the curated set
/// below is compiled in; every other name renders nothing so a bad stored value
/// can never break a page.
const ICONS: &[(&str, &str)] = &[
    (
        "alarm-clock",
        r#"<circle cx="12" cy="13" r="8" /><path d="M12 9v4l2 2" /><path d="M5 3 2 6" /><path d="m22 6-3-3" /><path d="M6.38 18.7 4 21" /><path d="M17.64 18.67 20 21" />"#,
    ),
    (
        "archive",
        r#"<rect width="20" height="5" x="2" y="3" rx="1" /><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8" /><path d="M10 12h4" />"#,
    ),
    (
        "arrow-left",
        r#"<path d="m12 19-7-7 7-7" /><path d="M19 12H5" />"#,
    ),
    (
        "arrow-up-right",
        r#"<path d="M7 7h10v10" /><path d="M7 17 17 7" />"#,
    ),
    (
        "calendar",
        r#"<path d="M8 2v3" /><path d="M16 2v3" /><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M3 9h18" />"#,
    ),
    ("check", r#"<path d="M20 6 9 17l-5-5" />"#),
    (
        "check-check",
        r#"<path d="M18 6 7 17l-5-5" /><path d="m22 10-7.5 7.5L13 16" />"#,
    ),
    ("chevron-down", r#"<path d="m6 9 6 6 6-6" />"#),
    ("chevron-left", r#"<path d="m15 18-6-6 6-6" />"#),
    ("chevron-right", r#"<path d="m9 18 6-6-6-6" />"#),
    ("chevron-up", r#"<path d="m18 15-6-6-6 6" />"#),
    ("circle", r#"<circle cx="12" cy="12" r="10" />"#),
    (
        "circle-alert",
        r#"<circle cx="12" cy="12" r="10" /><line x1="12" x2="12" y1="8" y2="12" /><line x1="12" x2="12.01" y1="16" y2="16" />"#,
    ),
    (
        "circle-check",
        r#"<circle cx="12" cy="12" r="10" /><path d="m16 9-5.5 5.5L8 12" />"#,
    ),
    (
        "clock",
        r#"<circle cx="12" cy="12" r="10" /><path d="M12 6v6l4 2" />"#,
    ),
    (
        "clock-alert",
        r#"<path d="M12 6v6l4 2" /><path d="M20 12v5" /><path d="M20 21h.01" /><path d="M21.25 8.2A10 10 0 1 0 16 21.16" />"#,
    ),
    (
        "columns-3",
        r#"<rect width="18" height="18" x="3" y="3" rx="2" /><path d="M9 3v18" /><path d="M15 3v18" />"#,
    ),
    (
        "copy",
        r#"<rect width="14" height="14" x="8" y="8" rx="2" ry="2" /><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />"#,
    ),
    (
        "ellipsis",
        r#"<circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" /><circle cx="5" cy="12" r="1" />"#,
    ),
    (
        "ellipsis-vertical",
        r#"<circle cx="12" cy="12" r="1" /><circle cx="12" cy="5" r="1" /><circle cx="12" cy="19" r="1" />"#,
    ),
    (
        "external-link",
        r#"<path d="M15 3h6v6" /><path d="M10 14 21 3" /><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />"#,
    ),
    (
        "folder",
        r#"<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />"#,
    ),
    (
        "folder-kanban",
        r#"<path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" /><path d="M8 10v4" /><path d="M12 10v2" /><path d="M16 10v6" />"#,
    ),
    (
        "folder-open",
        r#"<path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2" />"#,
    ),
    (
        "grip-vertical",
        r#"<circle cx="9" cy="12" r="1" /><circle cx="9" cy="5" r="1" /><circle cx="9" cy="19" r="1" /><circle cx="15" cy="12" r="1" /><circle cx="15" cy="5" r="1" /><circle cx="15" cy="19" r="1" />"#,
    ),
    (
        "hourglass",
        r#"<path d="M5 22h14" /><path d="M5 2h14" /><path d="M17 22v-4.172a2 2 0 0 0-.586-1.414L12 12l-4.414 4.414A2 2 0 0 0 7 17.828V22" /><path d="M7 2v4.172a2 2 0 0 0 .586 1.414L12 12l4.414-4.414A2 2 0 0 0 17 6.172V2" />"#,
    ),
    (
        "house",
        r#"<path d="M15 21v-8a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v8" /><path d="M3 10a2 2 0 0 1 .709-1.528l7-6a2 2 0 0 1 2.582 0l7 6A2 2 0 0 1 21 10v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />"#,
    ),
    (
        "inbox",
        r#"<polyline points="22 12 16 12 14 15 10 15 8 12 2 12" /><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" />"#,
    ),
    (
        "layout-grid",
        r#"<rect width="7" height="7" x="3" y="3" rx="1" /><rect width="7" height="7" x="14" y="3" rx="1" /><rect width="7" height="7" x="14" y="14" rx="1" /><rect width="7" height="7" x="3" y="14" rx="1" />"#,
    ),
    (
        "link",
        r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" /><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />"#,
    ),
    (
        "list",
        r#"<path d="M3 5h.01" /><path d="M3 12h.01" /><path d="M3 19h.01" /><path d="M8 5h13" /><path d="M8 12h13" /><path d="M8 19h13" />"#,
    ),
    (
        "log-out",
        r#"<path d="m16 17 5-5-5-5" /><path d="M21 12H9" /><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />"#,
    ),
    (
        "luggage",
        r#"<path d="M6 20a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2" /><path d="M8 18V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v14" /><path d="M10 20h4" /><circle cx="16" cy="20" r="2" /><circle cx="8" cy="20" r="2" />"#,
    ),
    (
        "map-pin",
        r#"<path d="M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0" /><circle cx="12" cy="10" r="3" />"#,
    ),
    (
        "package",
        r#"<path d="M11 21.73a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73z" /><path d="M12 22V12" /><polyline points="3.29 7 12 12 20.71 7" /><path d="m7.5 4.27 9 5.15" />"#,
    ),
    (
        "package-open",
        r#"<path d="M12 22v-9" /><path d="M15.17 2.21a1.67 1.67 0 0 1 1.63 0L21 4.57a1.93 1.93 0 0 1 0 3.36L8.82 14.79a1.655 1.655 0 0 1-1.64 0L3 12.43a1.93 1.93 0 0 1 0-3.36z" /><path d="M20 13v3.87a2.06 2.06 0 0 1-1.11 1.83l-6 3.08a1.93 1.93 0 0 1-1.78 0l-6-3.08A2.06 2.06 0 0 1 4 16.87V13" /><path d="M21 12.43a1.93 1.93 0 0 0 0-3.36L8.83 2.2a1.64 1.64 0 0 0-1.63 0L3 4.57a1.93 1.93 0 0 0 0 3.36l12.18 6.86a1.636 1.636 0 0 0 1.63 0z" />"#,
    ),
    (
        "pencil",
        r#"<path d="M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z" /><path d="m15 5 4 4" />"#,
    ),
    (
        "pin",
        r#"<path d="M12 17v5" /><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" />"#,
    ),
    (
        "pin-off",
        r#"<path d="M12 17v5" /><path d="M15 9.34V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H7.89" /><path d="m2 2 20 20" /><path d="M9 9v1.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h11" />"#,
    ),
    (
        "play",
        r#"<path d="M5 5a2 2 0 0 1 3.008-1.728l11.997 6.998a2 2 0 0 1 .003 3.458l-12 7A2 2 0 0 1 5 19z" />"#,
    ),
    ("plus", r#"<path d="M5 12h14" /><path d="M12 5v14" />"#),
    (
        "repeat",
        r#"<path d="m17 2 4 4-4 4" /><path d="M3 11v-1a4 4 0 0 1 4-4h14" /><path d="m7 22-4-4 4-4" /><path d="M21 13v1a4 4 0 0 1-4 4H3" />"#,
    ),
    (
        "rotate-ccw",
        r#"<path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /><path d="M3 3v5h5" />"#,
    ),
    (
        "search",
        r#"<path d="m21 21-4.34-4.34" /><circle cx="11" cy="11" r="8" />"#,
    ),
    (
        "settings",
        r#"<path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915" /><circle cx="12" cy="12" r="3" />"#,
    ),
    (
        "shopping-cart",
        r#"<path d="m2.05 2.05 1.099-.028a1 1 0 0 1 1.008.815l2.69 14.347A1 1 0 0 0 7.83 18H18" /><path d="M4.563 5h16.435a1 1 0 0 1 .981 1.204l-1.026 6.226A2 2 0 0 1 18.962 14H6.25" /><circle cx="18" cy="20" r="2" /><circle cx="8" cy="20" r="2" />"#,
    ),
    (
        "shuffle",
        r#"<path d="m18 14 4 4-4 4" /><path d="m18 2 4 4-4 4" /><path d="M2 18h1.973a4 4 0 0 0 3.3-1.7l5.454-8.6a4 4 0 0 1 3.3-1.7H22" /><path d="M2 6h1.972a4 4 0 0 1 3.6 2.2" /><path d="M22 18h-6.041a4 4 0 0 1-3.3-1.8l-.359-.45" />"#,
    ),
    (
        "square",
        r#"<rect width="18" height="18" x="3" y="3" rx="2" />"#,
    ),
    (
        "square-check",
        r#"<rect width="18" height="18" x="3" y="3" rx="2" /><path d="m16 9-5.5 5.5L8 12" />"#,
    ),
    (
        "star",
        r#"<path d="M11.525 2.295a.53.53 0 0 1 .95 0l2.31 4.679a2.123 2.123 0 0 0 1.595 1.16l5.166.756a.53.53 0 0 1 .294.904l-3.736 3.638a2.123 2.123 0 0 0-.611 1.878l.882 5.14a.53.53 0 0 1-.771.56l-4.618-2.428a2.122 2.122 0 0 0-1.973 0L6.396 21.01a.53.53 0 0 1-.77-.56l.881-5.139a2.122 2.122 0 0 0-.611-1.879L2.16 9.795a.53.53 0 0 1 .294-.906l5.165-.755a2.122 2.122 0 0 0 1.597-1.16z" />"#,
    ),
    (
        "tag",
        r#"<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z" /><circle cx="7.5" cy="7.5" r=".5" fill="currentColor" />"#,
    ),
    (
        "timer",
        r#"<line x1="10" x2="14" y1="2" y2="2" /><line x1="12" x2="15" y1="14" y2="11" /><circle cx="12" cy="14" r="8" />"#,
    ),
    (
        "trash",
        r#"<path d="M10 11v6" /><path d="M14 11v6" /><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" /><path d="M3 6h18" /><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />"#,
    ),
    (
        "triangle-alert",
        r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" /><path d="M12 9v4" /><path d="M12 17h.01" />"#,
    ),
    (
        "undo-2",
        r#"<path d="M9 14 4 9l5-5" /><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11" />"#,
    ),
    ("x", r#"<path d="M18 6 6 18" /><path d="m6 6 12 12" />"#),
];

/// Maps an alias (from stored data or older lucide versions) to a canonical key.
fn resolve_alias(name: &str) -> &str {
    match name {
        "trash-2" => "trash",
        "more-horizontal" => "ellipsis",
        "more-vertical" => "ellipsis-vertical",
        "home" => "house",
        other => other,
    }
}

/// Returns the inner SVG markup for `name`, or an empty string when unknown.
fn icon_markup(name: &str) -> &'static str {
    ICONS
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, markup)| *markup)
        .unwrap_or("")
}

/// Renders the SVG of the curated lucide `name`, or nothing when it is unknown.
fn render_icon(name: &str, size: Signal<usize>, color: Signal<String>) -> AnyView {
    let markup = icon_markup(resolve_alias(name));

    if markup.is_empty() {
        return ().into_any();
    }

    view! {
        <svg
            class="lucide"
            xmlns="http://www.w3.org/2000/svg"
            width=size
            height=size
            viewBox="0 0 24 24"
            fill="none"
            stroke=color
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            inner_html=markup
        />
    }
    .into_any()
}

/// A single SVG icon from the curated lucide set.
///
/// `name` is a lucide name (`"trash"`, `"square-check"`); unknown names render
/// nothing. `class` is applied to the wrapping `<span>`, which is what makes an
/// icon inherit sizing/colour utilities from its parent button or chip.
#[component]
pub fn Icon(
    #[prop(into)] name: IconName,
    /// Pixel size of the square icon.
    #[prop(default = 18)]
    size: usize,
    /// Extra classes for the wrapping span.
    #[prop(default = String::new(), into)]
    class: String,
    /// CSS colour for the stroke; defaults to `currentColor`.
    #[prop(optional_no_strip)]
    color: Option<String>,
) -> impl IntoView {
    let IconName(name) = name;
    let color = color.unwrap_or_else(|| "currentColor".to_string());
    let size_signal = Signal::from(size);
    let color_signal = Signal::from(color);

    view! {
        <span class=format!("inline-flex shrink-0 items-center justify-center {class}")>
            {move || render_icon(&name.get(), size_signal, color_signal)}
        </span>
    }
}

/// An entity's stored icon: a lucide icon, an emoji, or a fallback lucide icon.
///
/// `raw` is the value as stored in the database (`lucide:<name>`,
/// `emoji:<glyph>` or a legacy bare emoji) and is parsed with
/// [`parse_icon_value`]; anything unusable falls back to `fallback`.
#[component]
pub fn EntityIcon(
    /// Raw stored icon value.
    #[prop(optional_no_strip)]
    raw: Option<String>,
    /// Lucide name used when `raw` is absent or malformed.
    #[prop(into)]
    fallback: IconName,
    /// CSS colour applied to the lucide stroke or the emoji text.
    #[prop(optional_no_strip)]
    color: Option<String>,
    /// Pixel size of the square icon.
    #[prop(default = 18)]
    size: usize,
    /// Extra classes for the wrapping element.
    #[prop(default = String::new(), into)]
    class: String,
) -> impl IntoView {
    let IconName(fallback) = fallback;

    view! {
        {move || match parse_icon_value(raw.as_deref()) {
            IconValue::Lucide(name) => view! {
                <Icon name=name.to_string() size=size class=class.clone() color=color.clone() />
            }
            .into_any(),
            IconValue::Emoji(emoji) => view! {
                <span
                    class=format!("inline-flex shrink-0 items-center justify-center {class}")
                    style=color.as_ref().map(|c| format!("color: {c};"))
                >
                    {emoji.to_string()}
                </span>
            }
            .into_any(),
            IconValue::None => view! {
                <Icon name=fallback size=size class=class.clone() color=color.clone() />
            }
            .into_any(),
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_table_has_56_unique_valid_entries() {
        assert_eq!(ICONS.len(), 56);

        let mut keys: Vec<&str> = ICONS.iter().map(|(key, _)| *key).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total, "ICONS contains duplicate keys");

        for (key, markup) in ICONS {
            assert!(!markup.is_empty(), "{key} has empty markup");
            assert!(
                markup.contains("<path")
                    || markup.contains("<rect")
                    || markup.contains("<circle")
                    || markup.contains("<line")
                    || markup.contains("<polyline"),
                "{key} markup has no known shape element"
            );
        }
    }

    #[test]
    fn aliases_resolve_to_known_icons() {
        let aliases = [
            ("trash-2", "trash"),
            ("more-horizontal", "ellipsis"),
            ("more-vertical", "ellipsis-vertical"),
            ("home", "house"),
        ];
        for (alias, canonical) in aliases {
            assert_eq!(resolve_alias(alias), canonical);
            assert!(
                ICONS.iter().any(|(key, _)| *key == canonical),
                "alias {alias} resolved to unknown icon {canonical}"
            );
        }
        assert_eq!(resolve_alias("trash"), "trash");
    }
}
