use crate::*;

// --- UpdateItemRequest Option<Option<String>> ---

#[test]
fn update_item_absent_field_is_none() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{}"#).unwrap();
    assert!(req.deadline.is_none());
    assert!(req.title.is_none());
}

#[test]
fn update_item_null_field_is_none() {
    // serde flattens Option<Option<String>> — null becomes None, same as absent.
    // To distinguish "clear field" from "don't touch", the frontend must omit
    // the key (don't touch) vs send null (currently also treated as don't touch).
    let req: UpdateItemRequest = serde_json::from_str(r#"{"deadline": null}"#).unwrap();
    assert!(req.deadline.is_none());
}

#[test]
fn update_item_value_field_is_some_some() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{"deadline": "2024-12-31"}"#).unwrap();
    assert!(matches!(req.deadline, Some(Some(ref d)) if d == "2024-12-31"));
}

// --- UpdateItemRequest description (Option<Option<String>>) ---

#[test]
fn update_item_description_absent_is_none() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{}"#).unwrap();
    assert!(req.description.is_none(), "absent = don't touch");
}

#[test]
fn update_item_description_null_is_none() {
    // serde collapses Option<Option<String>>: null == absent == None (outer).
    // This means sending {"description": null} CANNOT clear the description —
    // the backend sees None and skips the update entirely.
    // Actual clearing requires a custom deserializer or a sentinel value.
    let req: UpdateItemRequest = serde_json::from_str(r#"{"description": null}"#).unwrap();
    assert!(
        req.description.is_none(),
        "null also becomes None — cannot clear via null JSON"
    );
}

#[test]
fn update_item_description_value_is_some_some() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{"description": "hello"}"#).unwrap();
    assert!(matches!(req.description, Some(Some(ref d)) if d == "hello"));
}

// --- DateItem -> Item conversion ---

#[test]
fn date_item_to_item_conversion() {
    let di = DateItem {
        id: "i1".into(),
        list_id: "l1".into(),
        title: "Buy milk".into(),
        description: None,
        completed: false,
        position: 0,
        quantity: Some(2),
        actual_quantity: Some(0),
        unit: Some("szt".into()),
        start_date: None,
        start_time: None,
        deadline: Some("2024-12-31".into()),
        deadline_time: None,
        hard_deadline: None,
        created_at: "2024-01-01".into(),
        updated_at: "2024-01-01".into(),
        list_name: "Shopping".into(),
        list_type: ListType::Zakupy,
        date_type: Some("deadline".into()),
    };
    let item: Item = di.into();
    assert_eq!(item.id, "i1");
    assert_eq!(item.quantity, Some(2));
    assert_eq!(item.deadline, Some("2024-12-31".into()));
}

// --- Full Item deserialization with D1-style floats ---

#[test]
fn item_deserialize_d1_booleans() {
    let json = r#"{
        "id": "abc",
        "list_id": "l1",
        "title": "Test",
        "description": null,
        "completed": 1.0,
        "position": 0,
        "quantity": null,
        "actual_quantity": null,
        "unit": null,
        "start_date": null,
        "start_time": null,
        "deadline": null,
        "deadline_time": null,
        "hard_deadline": null,
        "created_at": "2024-01-01",
        "updated_at": "2024-01-01"
    }"#;
    let item: Item = serde_json::from_str(json).unwrap();
    assert!(item.completed);
}

// --- DaySummary with D1 floats ---

#[test]
fn day_summary_deserialize() {
    let json = r#"{"date": "2024-12-01", "total": 5.0, "completed": 3.0}"#;
    let ds: DaySummary = serde_json::from_str(json).unwrap();
    assert_eq!(ds.total, 5);
    assert_eq!(ds.completed, 3);
}
