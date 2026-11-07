use crate::dto::requests::{
    UpdateItemRequest, UpdateListRequest, UpdateLocationRequest, UpdateTagRequest,
};

#[test]
fn update_list_request_preserves_explicit_null_for_nullable_fields() {
    let req = UpdateListRequest {
        name: None,
        description: Some(None),
        list_type: None,
        archived: None,
        location_id: None,
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

// --- UpdateLocationRequest (double-option over a JSON server-fn body) ---

#[test]
fn update_location_request_round_trips_keep_clear_and_set() {
    // keep, clear and set must all survive serde_json, which is what the JSON-encoded
    // `update_location_sf` server function puts on the wire.
    let req = UpdateLocationRequest {
        name: None,
        alias: Some(None),
        region: Some("Małopolska".into()),
        address: Some(Some("ul. Kowalska 1".into())),
    };

    let json = serde_json::to_string(&req).unwrap();
    assert_eq!(
        json,
        r#"{"alias":null,"region":"Małopolska","address":"ul. Kowalska 1"}"#
    );

    let back: UpdateLocationRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, None, "absent field must mean keep");
    assert_eq!(back.alias, Some(None), "explicit null must mean clear");
    assert_eq!(back.region.as_deref(), Some("Małopolska"));
    assert_eq!(back.address, Some(Some("ul. Kowalska 1".to_string())));
}

#[test]
fn update_location_request_serializes_every_field_when_all_are_set() {
    let req = UpdateLocationRequest {
        name: Some("Kraków".into()),
        alias: Some(Some("Praca".into())),
        region: Some("Małopolska".into()),
        address: Some(None),
    };

    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "name": "Kraków",
            "alias": "Praca",
            "region": "Małopolska",
            "address": null,
        })
    );

    let back: UpdateLocationRequest = serde_json::from_value(json).unwrap();
    assert_eq!(back.name.as_deref(), Some("Kraków"));
    assert_eq!(back.alias, Some(Some("Praca".to_string())));
    assert_eq!(back.region.as_deref(), Some("Małopolska"));
    assert_eq!(back.address, Some(None));
}

#[test]
fn update_location_request_empty_payload_touches_nothing() {
    let json = serde_json::to_string(&UpdateLocationRequest::default()).unwrap();
    assert_eq!(json, "{}");

    let back: UpdateLocationRequest = serde_json::from_str("{}").unwrap();
    assert_eq!(back.name, None);
    assert_eq!(back.alias, None);
    assert_eq!(back.region, None);
    assert_eq!(back.address, None);
}

#[test]
fn update_location_request_constructors_set_one_field_each() {
    let json = serde_json::to_value(UpdateLocationRequest::rename("Gdańsk")).unwrap();
    assert_eq!(json, serde_json::json!({ "name": "Gdańsk" }));

    let json = serde_json::to_value(UpdateLocationRequest::set_alias(None)).unwrap();
    assert_eq!(json, serde_json::json!({ "alias": null }));

    let json = serde_json::to_value(UpdateLocationRequest::set_alias(Some("Dom".into()))).unwrap();
    assert_eq!(json, serde_json::json!({ "alias": "Dom" }));

    let json = serde_json::to_value(UpdateLocationRequest::set_region("Pomorskie")).unwrap();
    assert_eq!(json, serde_json::json!({ "region": "Pomorskie" }));

    let json = serde_json::to_value(UpdateLocationRequest::set_address(None)).unwrap();
    assert_eq!(json, serde_json::json!({ "address": null }));
}
