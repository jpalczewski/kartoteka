//! Item estimates are stored as whole minutes. These helpers turn user input into minutes
//! and minutes into the short label shown on a chip.

/// Longest estimate accepted: 1000 hours, far beyond any task and safe against overflow.
pub const MAX_ESTIMATE_MINUTES: i32 = 60_000;

/// Parse an estimate typed by the user into minutes.
///
/// Accepted forms (case-insensitive, spaces ignored): `90` and `90m` (minutes), `2h`, `1.5h` /
/// `1,5h` (hours, decimal), `1h30m`. Returns `None` for blank input, zero, negative values,
/// anything unparsable and values above [`MAX_ESTIMATE_MINUTES`].
pub fn parse_estimate_minutes(input: &str) -> Option<i32> {
    let text: String = input
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .replace(',', ".");
    if text.is_empty() {
        return None;
    }

    let minutes = if let Some((hours, rest)) = text.split_once('h') {
        let hours: f64 = hours.parse().ok()?;
        let extra = match rest {
            "" => 0.0,
            rest => rest.strip_suffix('m')?.parse::<f64>().ok()?,
        };
        hours * 60.0 + extra
    } else {
        text.strip_suffix('m')
            .unwrap_or(&text)
            .parse::<f64>()
            .ok()?
    };

    let minutes = minutes.round();
    (minutes >= 1.0 && minutes <= f64::from(MAX_ESTIMATE_MINUTES)).then_some(minutes as i32)
}

/// Short label for an estimate: `45m`, `2h`, `1h 30m`.
pub fn format_estimate_minutes(minutes: i32) -> String {
    let (hours, rest) = (minutes / 60, minutes % 60);
    match (hours, rest) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// The estimate as the editor shows it (`1h30m` style, re-parsable by [`parse_estimate_minutes`]).
pub fn estimate_input_text(minutes: i32) -> String {
    format_estimate_minutes(minutes).replace(' ', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minutes_hours_and_mixed_forms() {
        assert_eq!(parse_estimate_minutes("90"), Some(90));
        assert_eq!(parse_estimate_minutes(" 45 m "), Some(45));
        assert_eq!(parse_estimate_minutes("2h"), Some(120));
        assert_eq!(parse_estimate_minutes("1,5H"), Some(90));
        assert_eq!(parse_estimate_minutes("1h30m"), Some(90));
        assert_eq!(parse_estimate_minutes("1h 15"), None);
    }

    #[test]
    fn rejects_blank_zero_negative_and_garbage() {
        for bad in [
            "", "  ", "0", "0h", "-5", "abc", "h", "1x", "99999h", "1h5x",
        ] {
            assert_eq!(parse_estimate_minutes(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn formats_and_round_trips() {
        assert_eq!(format_estimate_minutes(45), "45m");
        assert_eq!(format_estimate_minutes(120), "2h");
        assert_eq!(format_estimate_minutes(90), "1h 30m");
        for minutes in [1, 45, 60, 90, 125, 1000] {
            assert_eq!(
                parse_estimate_minutes(&estimate_input_text(minutes)),
                Some(minutes)
            );
        }
    }
}
