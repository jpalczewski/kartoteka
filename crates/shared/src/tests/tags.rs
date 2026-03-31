use crate::*;

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
