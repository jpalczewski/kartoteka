//! Parsing of the stored *icon value* strings used by lists and containers.
//!
//! A stored value is one of:
//!
//! - `lucide:<name>` — a lucide icon name such as `square-check`,
//! - `emoji:<glyph>` — an explicit emoji, e.g. `emoji:📦`,
//! - any other non-empty string — a legacy bare emoji, e.g. `📦`.
//!
//! Empty/blank values and malformed `lucide:` names mean "no icon", so callers
//! can fall back to a per-entity default (`default_list_icon`,
//! `default_container_icon`).

/// A parsed icon value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconValue<'a> {
    /// A lucide icon name (matches `[a-z0-9-]+`).
    Lucide(&'a str),
    /// A literal emoji/glyph to render as text.
    Emoji(&'a str),
    /// Nothing to render.
    None,
}

/// Returns `true` for a valid lucide icon name: non-empty, lowercase
/// alphanumerics and `-` only.
fn is_lucide_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Parses a stored icon value, returning a borrowed [`IconValue`].
///
/// Surrounding whitespace is ignored. An empty (or whitespace-only) value, a
/// malformed `lucide:` name and an empty `emoji:` value all parse to
/// [`IconValue::None`].
pub fn parse_icon_value(raw: Option<&str>) -> IconValue<'_> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return IconValue::None;
    };

    if let Some(name) = raw.strip_prefix("lucide:") {
        let name = name.trim();
        return if is_lucide_name(name) {
            IconValue::Lucide(name)
        } else {
            IconValue::None
        };
    }

    if let Some(emoji) = raw.strip_prefix("emoji:") {
        let emoji = emoji.trim();
        return if emoji.is_empty() {
            IconValue::None
        } else {
            IconValue::Emoji(emoji)
        };
    }

    IconValue::Emoji(raw)
}

/// Default lucide icon for a list type (`checklist`, `shopping`, `schedule`,
/// `habits`, `log`, `notes`, …). Unknown types get `list`.
///
/// The Polish aliases (`zakupy`, `pakowanie`, `terminarz`) are accepted as well
/// so callers can use either vocabulary.
pub fn default_list_icon(list_type: &str) -> &'static str {
    match list_type.trim().to_ascii_lowercase().as_str() {
        "checklist" => "square-check",
        "shopping" | "zakupy" => "shopping-cart",
        "packing" | "pakowanie" => "luggage",
        "schedule" | "habits" | "terminarz" => "calendar",
        "log" | "dziennik" => "timer",
        _ => "list",
    }
}

/// Default lucide icon for a container: `folder-kanban` for projects, `folder`
/// for plain folders (`status.is_some()` means project).
pub fn default_container_icon(is_project: bool) -> &'static str {
    if is_project {
        "folder-kanban"
    } else {
        "folder"
    }
}
