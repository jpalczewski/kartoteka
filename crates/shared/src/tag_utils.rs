use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::types::Tag;

// ── Colours ───────────────────────────────────────────────────────────────────

/// Colour assumed for a tag that has none stored.
///
/// Every component that renders a tag falls back to this one value, so a tag
/// without a colour looks the same on the tags page, the detail page and the
/// filter bars.
pub const DEFAULT_TAG_COLOR: &str = "#6b7280";

/// Colours offered by the tag colour picker.
///
/// Mid-tone hues that stay legible on the dark theme and produce a readable
/// foreground through [`readable_text_color`] — no neon or pastel extremes.
pub const TAG_PALETTE: &[&str] = &[
    "#6b7280", // slate
    "#ef4444", // red
    "#f97316", // orange
    "#f59e0b", // amber
    "#eab308", // yellow
    "#84cc16", // lime
    "#22c55e", // green
    "#14b8a6", // teal
    "#06b6d4", // cyan
    "#3b82f6", // blue
    "#8b5cf6", // violet
    "#ec4899", // pink
];

/// Foreground colours [`readable_text_color`] chooses between.
const DARK_TEXT: &str = "#111827";
const LIGHT_TEXT: &str = "#f9fafb";

/// A dark or light text colour with the better WCAG contrast on `hex`.
///
/// `hex` accepts `#rgb`, `#rrggbb`, and the same without the leading `#`.
/// Unparsable input returns the light colour: on the dark theme a light label
/// stays legible far more often than a dark one.
pub fn readable_text_color(hex: &str) -> &'static str {
    let Some((r, g, b)) = parse_hex_color(hex) else {
        return LIGHT_TEXT;
    };
    let background = relative_luminance(r, g, b);
    let on_dark = contrast_ratio(background, relative_luminance(0x11, 0x18, 0x27));
    let on_light = contrast_ratio(background, relative_luminance(0xf9, 0xfa, 0xfb));
    if on_dark >= on_light {
        DARK_TEXT
    } else {
        LIGHT_TEXT
    }
}

/// Parses `#rgb` / `#rrggbb` (leading `#` optional) into its RGB channels.
fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let trimmed = hex.trim();
    let digits = trimmed.strip_prefix('#').unwrap_or(trimmed);
    let nibble = |c: char| c.to_digit(16).and_then(|v| u8::try_from(v).ok());
    match digits.len() {
        3 => {
            let mut channels = [0u8; 3];
            for (index, c) in digits.chars().enumerate() {
                channels[index] = nibble(c)? * 17;
            }
            Some((channels[0], channels[1], channels[2]))
        }
        6 => {
            let byte = |range: std::ops::Range<usize>| {
                digits
                    .get(range)
                    .and_then(|part| u8::from_str_radix(part, 16).ok())
            };
            Some((byte(0..2)?, byte(2..4)?, byte(4..6)?))
        }
        _ => None,
    }
}

/// WCAG 2 relative luminance of an sRGB colour.
fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    fn channel(value: u8) -> f64 {
        let v = f64::from(value) / 255.0;
        if v <= 0.039_28 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

/// WCAG 2 contrast ratio between two relative luminances.
fn contrast_ratio(a: f64, b: f64) -> f64 {
    let (bright, dark) = if a > b { (a, b) } else { (b, a) };
    (bright + 0.05) / (dark + 0.05)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FilterMode {
    #[default]
    Listwise,
    Itemwise,
    Joined,
}

/// Returns the full ancestor path of a tag joined by `sep` (root → leaf).
/// E.g. with sep `"\\"`: "Hobby\\Ogród\\Warzywa"
pub fn ancestor_path(tag: &Tag, all_tags: &[Tag], sep: &str) -> String {
    let by_id: HashMap<&str, &Tag> = all_tags.iter().map(|t| (t.id.as_str(), t)).collect();
    ancestor_path_with_map(tag, &by_id, sep)
}

/// Builds a map of `tag_id → ancestor_path` for all tags in one pass.
/// Use this when you need paths for many tags — avoids rebuilding the lookup map per tag.
pub fn build_ancestor_map(all_tags: &[Tag], sep: &str) -> HashMap<String, String> {
    let by_id: HashMap<&str, &Tag> = all_tags.iter().map(|t| (t.id.as_str(), t)).collect();
    all_tags
        .iter()
        .map(|tag| (tag.id.clone(), ancestor_path_with_map(tag, &by_id, sep)))
        .collect()
}

fn ancestor_path_with_map<'a>(tag: &'a Tag, by_id: &HashMap<&str, &'a Tag>, sep: &str) -> String {
    let mut parts = vec![tag.name.clone()];
    let mut current = tag;
    while let Some(ref pid) = current.parent_tag_id {
        match by_id.get(pid.as_str()) {
            Some(&parent) => {
                parts.push(parent.name.clone());
                current = parent;
            }
            None => break,
        }
    }
    parts.reverse();
    parts.join(sep)
}
