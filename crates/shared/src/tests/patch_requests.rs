use crate::*;

#[test]
fn update_container_request_omits_missing_optional_fields() {
    let req = UpdateContainerRequest {
        name: Some("Projekt".into()),
        description: None,
        status: None,
    };

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({ "name": "Projekt" }));
}

#[test]
fn update_list_request_preserves_explicit_null_for_nullable_fields() {
    let req = UpdateListRequest {
        name: None,
        description: Some(None),
        list_type: None,
        archived: None,
    };

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({ "description": null }));
}

#[test]
fn update_item_request_omits_unset_nullable_fields() {
    let req = UpdateItemRequest {
        completed: Some(true),
        ..Default::default()
    };

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({ "completed": true }));
}

#[test]
fn update_item_request_preserves_explicit_clear() {
    let req = UpdateItemRequest {
        deadline: Some(None),
        ..Default::default()
    };

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({ "deadline": null }));
}

#[test]
fn update_tag_request_omits_parent_when_not_being_changed() {
    let req = UpdateTagRequest {
        name: None,
        color: Some("#123456".into()),
        parent_tag_id: None,
    };

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({ "color": "#123456" }));
}

#[test]
fn update_tag_request_preserves_explicit_parent_clear() {
    let req = UpdateTagRequest {
        name: None,
        color: None,
        parent_tag_id: Some(None),
    };

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json, serde_json::json!({ "parent_tag_id": null }));
}
