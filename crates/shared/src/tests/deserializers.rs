use crate::ListFeature;
use serde::Deserialize;

// --- bool_from_number (D1 returns booleans as 0.0/1.0) ---

#[derive(Deserialize)]
struct BoolWrap {
    #[serde(deserialize_with = "crate::bool_from_number")]
    v: bool,
}

#[test]
fn bool_from_number_zero_float() {
    let t: BoolWrap = serde_json::from_str(r#"{"v": 0.0}"#).unwrap();
    assert!(!t.v);
}

#[test]
fn bool_from_number_one_float() {
    let t: BoolWrap = serde_json::from_str(r#"{"v": 1.0}"#).unwrap();
    assert!(t.v);
}

#[test]
fn bool_from_number_integer() {
    let t: BoolWrap = serde_json::from_str(r#"{"v": 1}"#).unwrap();
    assert!(t.v);
    let t: BoolWrap = serde_json::from_str(r#"{"v": 0}"#).unwrap();
    assert!(!t.v);
}

#[test]
fn bool_from_number_native_bool() {
    let t: BoolWrap = serde_json::from_str(r#"{"v": true}"#).unwrap();
    assert!(t.v);
    let t: BoolWrap = serde_json::from_str(r#"{"v": false}"#).unwrap();
    assert!(!t.v);
}

#[test]
fn bool_from_number_string_is_false() {
    let t: BoolWrap = serde_json::from_str(r#"{"v": "yes"}"#).unwrap();
    assert!(!t.v);
}

// --- u32_from_number ---

#[derive(Deserialize)]
struct U32Wrap {
    #[serde(deserialize_with = "crate::u32_from_number")]
    v: u32,
}

#[test]
fn u32_from_number_float() {
    let t: U32Wrap = serde_json::from_str(r#"{"v": 42.0}"#).unwrap();
    assert_eq!(t.v, 42);
}

#[test]
fn u32_from_number_integer() {
    let t: U32Wrap = serde_json::from_str(r#"{"v": 7}"#).unwrap();
    assert_eq!(t.v, 7);
}

#[test]
fn u32_from_number_non_numeric_is_zero() {
    let t: U32Wrap = serde_json::from_str(r#"{"v": "nope"}"#).unwrap();
    assert_eq!(t.v, 0);
}

// --- features_from_json ---

#[derive(Deserialize)]
struct FeatWrap {
    #[serde(deserialize_with = "crate::features_from_json")]
    f: Vec<ListFeature>,
}

#[test]
fn features_from_json_string() {
    let json = r#"{"f": "[{\"name\":\"quantity\",\"config\":{}}]"}"#;
    let t: FeatWrap = serde_json::from_str(json).unwrap();
    assert_eq!(t.f.len(), 1);
    assert_eq!(t.f[0].name, "quantity");
}

#[test]
fn features_from_json_array() {
    let json = r#"{"f": [{"name":"deadlines","config":{}}]}"#;
    let t: FeatWrap = serde_json::from_str(json).unwrap();
    assert_eq!(t.f.len(), 1);
    assert_eq!(t.f[0].name, "deadlines");
}

#[test]
fn features_from_json_null() {
    let t: FeatWrap = serde_json::from_str(r#"{"f": null}"#).unwrap();
    assert!(t.f.is_empty());
}

#[test]
fn features_from_json_empty_string() {
    let t: FeatWrap = serde_json::from_str(r#"{"f": "[]"}"#).unwrap();
    assert!(t.f.is_empty());
}
