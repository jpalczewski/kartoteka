use crate::dto::requests::CreateTagRequest;

#[test]
fn create_tag_request_accepts_missing_color() {
    let req: CreateTagRequest = serde_json::from_str(r#"{"name": "Pilne"}"#).unwrap();
    assert_eq!(req.name, "Pilne");
    assert!(req.color.is_none());
}

#[test]
fn create_tag_request_preserves_explicit_color() {
    let req: CreateTagRequest =
        serde_json::from_str(r##"{"name": "Pilne", "color": "#ff0000"}"##).unwrap();
    assert_eq!(req.color.as_deref(), Some("#ff0000"));
}

// ── Tag colours ───────────────────────────────────────────────────────────────

use crate::{DEFAULT_TAG_COLOR, TAG_PALETTE, readable_text_color};

#[test]
fn readable_text_color_picks_dark_text_on_light_background() {
    // Amber and yellow are light enough that a white label would be unreadable.
    assert_eq!(readable_text_color("#eab308"), "#111827");
    assert_eq!(readable_text_color("#fde047"), "#111827");
    assert_eq!(readable_text_color("#ffffff"), "#111827");
}

#[test]
fn readable_text_color_picks_light_text_on_dark_background() {
    assert_eq!(readable_text_color("#1e3a8a"), "#f9fafb");
    assert_eq!(readable_text_color("#000000"), "#f9fafb");
    // The neutral default sits on the dark side too, which is what the dark theme expects.
    assert_eq!(readable_text_color(DEFAULT_TAG_COLOR), "#f9fafb");
}

#[test]
fn readable_text_color_expands_short_hex() {
    assert_eq!(readable_text_color("#fff"), readable_text_color("#ffffff"));
    assert_eq!(readable_text_color("0f0"), readable_text_color("#00ff00"));
}

#[test]
fn readable_text_color_falls_back_to_light_on_invalid_input() {
    for invalid in ["", "nope", "#12", "#12345", "#gggggg", "rgb(1,2,3)"] {
        assert_eq!(
            readable_text_color(invalid),
            "#f9fafb",
            "{invalid:?} should fall back to the light text colour"
        );
    }
}

#[test]
fn readable_text_color_always_reaches_contrast_of_at_least_three_to_one() {
    // Sanity check over the whole palette: every swatch must produce a label with
    // enough contrast to read, which is what the badge contrast bug was about.
    for hex in TAG_PALETTE.iter().copied().chain([DEFAULT_TAG_COLOR]) {
        let text = readable_text_color(hex);
        let background = luminance(hex);
        let foreground = luminance(text);
        let (bright, dark) = if background > foreground {
            (background, foreground)
        } else {
            (foreground, background)
        };
        let ratio = (bright + 0.05) / (dark + 0.05);
        assert!(
            ratio >= 3.0,
            "{hex} with {text} only reaches a contrast ratio of {ratio:.2}"
        );
    }
}

#[test]
fn palette_is_plain_lowercase_hex_without_duplicates() {
    for hex in TAG_PALETTE {
        assert!(
            hex.len() == 7
                && hex.starts_with('#')
                && hex[1..].chars().all(|c| c.is_ascii_hexdigit()),
            "{hex} is not a #rrggbb colour"
        );
        assert_eq!(*hex, hex.to_ascii_lowercase());
    }
    let unique: std::collections::HashSet<_> = TAG_PALETTE.iter().collect();
    assert_eq!(unique.len(), TAG_PALETTE.len(), "palette has duplicates");
    assert!(TAG_PALETTE.contains(&DEFAULT_TAG_COLOR));
}

/// Test-local relative luminance, kept independent of the implementation under test.
fn luminance(hex: &str) -> f64 {
    let digits = hex.trim_start_matches('#');
    let channel = |index: usize| {
        let part = &digits[index * 2..index * 2 + 2];
        let value = f64::from(u8::from_str_radix(part, 16).unwrap()) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2)
}
