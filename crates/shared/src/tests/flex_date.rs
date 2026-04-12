use crate::types::FlexDate;
use chrono::NaiveDate;

#[test]
fn day_roundtrip_serde() {
    let d = FlexDate::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
    let json = serde_json::to_string(&d).unwrap();
    assert_eq!(json, "\"2026-05-15\"");
    let parsed: FlexDate = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, d);
}

#[test]
fn week_roundtrip_serde() {
    let w = FlexDate::Week(2026, 20);
    let json = serde_json::to_string(&w).unwrap();
    assert_eq!(json, "\"2026-W20\"");
    let parsed: FlexDate = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, w);
}

#[test]
fn month_roundtrip_serde() {
    let m = FlexDate::Month(2026, 5);
    let json = serde_json::to_string(&m).unwrap();
    assert_eq!(json, "\"2026-05\"");
    let parsed: FlexDate = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, m);
}

#[test]
fn day_start_end_equal() {
    let d = FlexDate::Day(NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
    assert_eq!(d.start(), d.end());
}

#[test]
fn week_span_is_7_days() {
    let w = FlexDate::Week(2026, 20);
    let span = w.end().signed_duration_since(w.start()).num_days();
    assert_eq!(span, 6); // Mon-Sun inclusive = 6 day difference
}

#[test]
fn month_span() {
    let m = FlexDate::Month(2026, 2);
    assert_eq!(m.start(), NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
    assert_eq!(m.end(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
}

#[test]
fn is_fuzzy() {
    let day = FlexDate::Day(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
    let week = FlexDate::Week(2026, 1);
    let month = FlexDate::Month(2026, 1);
    assert!(!day.is_fuzzy());
    assert!(week.is_fuzzy());
    assert!(month.is_fuzzy());
}
