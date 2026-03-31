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
    let req: UpdateItemRequest = serde_json::from_str(r#"{"deadline": null}"#).unwrap();
    assert!(matches!(req.deadline, Some(None)));
}

#[test]
fn update_item_value_field_is_some_some() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{"deadline": "2024-12-31"}"#).unwrap();
    assert!(matches!(req.deadline, Some(Some(ref d)) if d == "2024-12-31"));
}

// --- UpdateItemRequest description sentinel convention ---

#[test]
fn update_item_description_absent_is_none() {
    // None = don't touch description
    let req: UpdateItemRequest = serde_json::from_str(r#"{}"#).unwrap();
    assert!(req.description.is_none());
}

#[test]
fn update_item_description_null_clears_field() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{"description": null}"#).unwrap();
    assert!(matches!(req.description, Some(None)));
}

#[test]
fn update_item_description_value_is_some_string() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{"description": "hello"}"#).unwrap();
    assert!(matches!(req.description, Some(Some(ref d)) if d == "hello"));
}

#[test]
fn update_item_description_empty_string_is_preserved() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{"description": ""}"#).unwrap();
    assert!(matches!(req.description, Some(Some(ref d)) if d.is_empty()));
}

#[test]
fn update_item_unit_absent_is_none() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{}"#).unwrap();
    assert!(req.unit.is_none());
}

#[test]
fn update_item_unit_null_clears_field() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{"unit": null}"#).unwrap();
    assert!(matches!(req.unit, Some(None)));
}

#[test]
fn update_item_unit_value_is_some_some() {
    let req: UpdateItemRequest = serde_json::from_str(r#"{"unit": "kg"}"#).unwrap();
    assert!(matches!(req.unit, Some(Some(ref unit)) if unit == "kg"));
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

#[test]
fn date_item_deserializes_numeric_completed_and_missing_date_type() {
    let json = r#"{
        "id": "abc",
        "list_id": "l1",
        "title": "Test",
        "description": null,
        "completed": 1,
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
        "updated_at": "2024-01-01",
        "list_name": "Lista",
        "list_type": "checklist"
    }"#;
    let item: DateItem = serde_json::from_str(json).unwrap();
    assert!(item.completed);
    assert!(item.date_type.is_none());
}

#[test]
fn validate_business_date_accepts_valid_date() {
    let parsed = validate_business_date("2026-03-31").unwrap();
    assert_eq!(format_date(&parsed), "2026-03-31");
}

#[test]
fn validate_business_date_rejects_invalid_date() {
    assert_eq!(
        validate_business_date("2026-02-30"),
        Err(DateValidationError::Invalid)
    );
}

#[test]
fn validate_business_date_rejects_out_of_range_year() {
    assert_eq!(
        validate_business_date("1900-01-01"),
        Err(DateValidationError::OutOfRange)
    );
}

#[test]
fn validate_hhmm_time_rejects_seconds() {
    assert_eq!(
        validate_hhmm_time("14:30:00"),
        Err(TimeValidationError::Invalid)
    );
}

#[test]
fn validate_hhmm_time_rejects_invalid_hour() {
    assert_eq!(
        validate_hhmm_time("25:61"),
        Err(TimeValidationError::Invalid)
    );
}

#[test]
fn validate_hhmm_time_rejects_missing_zero_padding() {
    assert_eq!(validate_hhmm_time("9:5"), Err(TimeValidationError::Invalid));
}

#[test]
fn validate_hhmm_time_accepts_zero_padded_time() {
    assert!(validate_hhmm_time("09:05").is_ok());
}
