//! Icons and accent colours shared by items, lists and containers.

use crate::DomainError;

/// Longest accepted icon value. Covers `lucide:<name>` and `emoji:<e>` as well
/// as legacy bare emoji (a single grapheme cluster is a handful of code points).
const MAX_ICON_CHARS: usize = 64;

/// Normalize an icon value.
///
/// Accepted forms are `lucide:<name>`, `emoji:<emoji>` and a legacy bare emoji.
/// Nothing else is checked: an empty (or whitespace-only) value means "no icon",
/// and anything longer than [`MAX_ICON_CHARS`] characters is rejected so a
/// broken client cannot stuff arbitrary text into the column.
pub fn normalize_icon(icon: Option<&str>) -> Result<Option<String>, DomainError> {
    let Some(raw) = icon else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_ICON_CHARS {
        return Err(DomainError::Validation("icon_too_long"));
    }
    Ok(Some(trimmed.to_string()))
}

/// Normalize and validate an accent colour.
///
/// Only `#rrggbb` is accepted (same rule as the UI colour picker). An empty
/// value means "no colour" and clears the field.
pub fn normalize_color(color: Option<&str>) -> Result<Option<String>, DomainError> {
    let Some(raw) = color else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    validate_color(trimmed)?;
    Ok(Some(trimmed.to_string()))
}

/// `#rrggbb`: `#` followed by exactly six hex digits.
fn validate_color(color: &str) -> Result<(), DomainError> {
    let hex = color
        .strip_prefix('#')
        .ok_or(DomainError::Validation("color_invalid_format"))?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(DomainError::Validation("color_invalid_format"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_none_stays_none() {
        assert_eq!(normalize_icon(None).unwrap(), None);
        assert_eq!(normalize_icon(Some("")).unwrap(), None);
        assert_eq!(normalize_icon(Some("   ")).unwrap(), None);
    }

    #[test]
    fn icon_is_trimmed_and_passed_through() {
        assert_eq!(
            normalize_icon(Some("  lucide:home ")).unwrap().as_deref(),
            Some("lucide:home")
        );
        assert_eq!(
            normalize_icon(Some("emoji:🌱")).unwrap().as_deref(),
            Some("emoji:🌱")
        );
        assert_eq!(normalize_icon(Some("🌱")).unwrap().as_deref(), Some("🌱"));
    }

    #[test]
    fn icon_longer_than_limit_rejected() {
        let long = "a".repeat(MAX_ICON_CHARS + 1);
        assert!(matches!(
            normalize_icon(Some(&long)),
            Err(DomainError::Validation("icon_too_long"))
        ));
        let at_limit = "a".repeat(MAX_ICON_CHARS);
        assert!(normalize_icon(Some(&at_limit)).is_ok());
    }

    #[test]
    fn color_none_and_empty_clear() {
        assert_eq!(normalize_color(None).unwrap(), None);
        assert_eq!(normalize_color(Some("")).unwrap(), None);
        assert_eq!(normalize_color(Some("  ")).unwrap(), None);
    }

    #[test]
    fn color_six_hex_accepted() {
        assert_eq!(
            normalize_color(Some("#1a2b3c")).unwrap().as_deref(),
            Some("#1a2b3c")
        );
        assert_eq!(
            normalize_color(Some(" #ABCDEF ")).unwrap().as_deref(),
            Some("#ABCDEF")
        );
    }

    #[test]
    fn color_rejects_short_long_and_named() {
        for bad in ["#abc", "#1234567", "1a2b3c", "red", "#gggggg", "#12345z"] {
            assert!(
                matches!(
                    normalize_color(Some(bad)),
                    Err(DomainError::Validation("color_invalid_format"))
                ),
                "expected {bad} to be rejected"
            );
        }
    }
}
