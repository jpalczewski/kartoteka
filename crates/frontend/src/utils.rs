use kartoteka_shared::types::DateItem;

pub fn group_by_list(items: Vec<DateItem>) -> Vec<(String, String, Vec<DateItem>)> {
    let mut groups: Vec<(String, String, Vec<DateItem>)> = Vec::new();
    for item in items {
        let lid = item.item.list_id.clone();
        if let Some(g) = groups.iter_mut().find(|(id, _, _)| id == &lid) {
            g.2.push(item);
        } else {
            groups.push((lid, item.list_name.clone(), vec![item]));
        }
    }
    groups
}

// ── Relative timestamps ───────────────────────────────────────────────────────

/// Coarse "how long ago" bucket for a UTC timestamp.
///
/// Coarse on purpose: the label is rendered during SSR and again during hydration, so a
/// per-second value would produce different text on both sides. A bucket changes at most
/// once a minute.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AgeBucket {
    JustNow,
    Minutes(i64),
    Hours(i64),
    Days(i64),
}

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;

/// Wall-clock milliseconds since the Unix epoch.
///
/// SSR reads the server clock; the browser build reads the same clock through
/// `Date.now()`, because `chrono`'s `Utc::now()` is not available on
/// `wasm32-unknown-unknown` without its `wasmbind` feature.
#[cfg(all(target_arch = "wasm32", feature = "hydrate"))]
pub fn now_millis() -> i64 {
    // `Date.now()` is an integer number of milliseconds, well inside `i64`.
    js_sys::Date::now() as i64
}

#[cfg(not(all(target_arch = "wasm32", feature = "hydrate")))]
pub fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Bucket the age of a UTC `"YYYY-MM-DD HH:MM:SS"` timestamp. `None` when it does not
/// parse, so the caller can fall back to the raw value.
pub fn age_bucket(timestamp_utc: &str, now_millis: i64) -> Option<AgeBucket> {
    use chrono::TimeZone as _;

    let naive = chrono::NaiveDateTime::parse_from_str(timestamp_utc, "%Y-%m-%d %H:%M:%S").ok()?;
    let then = chrono::Utc.from_utc_datetime(&naive).timestamp_millis();
    // A client clock behind the server (or a row marked "now") must not read as a
    // negative age.
    let elapsed = now_millis.saturating_sub(then).max(0);

    Some(if elapsed < MINUTE_MS {
        AgeBucket::JustNow
    } else if elapsed < HOUR_MS {
        AgeBucket::Minutes(elapsed / MINUTE_MS)
    } else if elapsed < DAY_MS {
        AgeBucket::Hours(elapsed / HOUR_MS)
    } else {
        AgeBucket::Days(elapsed / DAY_MS)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2024-05-03 12:00:00 UTC in milliseconds.
    fn base() -> i64 {
        use chrono::TimeZone as _;
        let naive = chrono::NaiveDate::from_ymd_opt(2024, 5, 3)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        chrono::Utc.from_utc_datetime(&naive).timestamp_millis()
    }

    #[test]
    fn age_bucket_covers_every_range() {
        let marked = "2024-05-03 12:00:00";
        assert_eq!(age_bucket(marked, base()), Some(AgeBucket::JustNow));
        assert_eq!(
            age_bucket(marked, base() + 59_000),
            Some(AgeBucket::JustNow),
            "under a minute stays 'just now'"
        );
        assert_eq!(
            age_bucket(marked, base() + 5 * MINUTE_MS),
            Some(AgeBucket::Minutes(5))
        );
        assert_eq!(
            age_bucket(marked, base() + 3 * HOUR_MS),
            Some(AgeBucket::Hours(3))
        );
        assert_eq!(
            age_bucket(marked, base() + 2 * DAY_MS),
            Some(AgeBucket::Days(2))
        );
    }

    #[test]
    fn age_bucket_clamps_future_timestamps() {
        let marked = "2024-05-03 12:00:00";
        assert_eq!(
            age_bucket(marked, base() - 10 * MINUTE_MS),
            Some(AgeBucket::JustNow),
            "a clock behind the server must not produce a negative age"
        );
    }

    #[test]
    fn age_bucket_rejects_unparsable_input() {
        assert_eq!(age_bucket("", base()), None);
        assert_eq!(age_bucket("yesterday", base()), None);
    }
}
