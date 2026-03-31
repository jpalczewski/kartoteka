use crate::*;

// --- ListType serde + default_features ---

#[test]
fn list_type_serde_snake_case() {
    assert_eq!(
        serde_json::to_string(&ListType::Checklist).unwrap(),
        r#""checklist""#
    );
    assert_eq!(
        serde_json::to_string(&ListType::Zakupy).unwrap(),
        r#""zakupy""#
    );
    assert_eq!(
        serde_json::to_string(&ListType::Custom).unwrap(),
        r#""custom""#
    );

    let lt: ListType = serde_json::from_str(r#""terminarz""#).unwrap();
    assert_eq!(lt, ListType::Terminarz);
}

#[test]
fn default_features_zakupy() {
    let features = ListType::Zakupy.default_features();
    assert_eq!(features.len(), 1);
    assert_eq!(features[0].name, FEATURE_QUANTITY);
}

#[test]
fn default_features_pakowanie() {
    let features = ListType::Pakowanie.default_features();
    assert_eq!(features.len(), 1);
    assert_eq!(features[0].name, FEATURE_QUANTITY);
}

#[test]
fn default_features_terminarz() {
    let features = ListType::Terminarz.default_features();
    assert_eq!(features.len(), 1);
    assert_eq!(features[0].name, FEATURE_DEADLINES);
}

#[test]
fn default_features_checklist_empty() {
    assert!(ListType::Checklist.default_features().is_empty());
}

#[test]
fn default_features_custom_empty() {
    assert!(ListType::Custom.default_features().is_empty());
}

// --- List::has_feature ---

#[test]
fn list_has_feature() {
    let list = List {
        id: "1".into(),
        user_id: "u1".into(),
        name: "test".into(),
        description: None,
        list_type: ListType::Zakupy,
        parent_list_id: None,
        position: 0,
        archived: false,
        features: vec![ListFeature {
            name: FEATURE_QUANTITY.into(),
            config: serde_json::json!({}),
        }],
        container_id: None,
        pinned: false,
        last_opened_at: None,
        created_at: "2024-01-01".into(),
        updated_at: "2024-01-01".into(),
    };
    assert!(list.has_feature(FEATURE_QUANTITY));
    assert!(!list.has_feature(FEATURE_DEADLINES));
}

// --- FeatureConfigRequest ---

#[test]
fn feature_config_request_default() {
    let req: FeatureConfigRequest = serde_json::from_str(r#"{}"#).unwrap();
    assert!(req.config.is_object());
    assert_eq!(req.config, serde_json::json!({}));
}

#[test]
fn feature_config_request_with_config() {
    let req: FeatureConfigRequest =
        serde_json::from_str(r#"{"config": {"unit_default": "kg"}}"#).unwrap();
    assert_eq!(req.config["unit_default"], "kg");
}

// --- MoveListRequest ---

#[test]
fn move_list_request_with_container() {
    let req: MoveListRequest = serde_json::from_str(r#"{"container_id": "c1"}"#).unwrap();
    assert_eq!(req.container_id, Some("c1".into()));
}

#[test]
fn move_list_request_remove_from_container() {
    let req: MoveListRequest = serde_json::from_str(r#"{"container_id": null}"#).unwrap();
    assert!(req.container_id.is_none());
}

#[test]
fn update_list_description_null_clears_field() {
    let req: UpdateListRequest = serde_json::from_str(r#"{"description": null}"#).unwrap();
    assert!(matches!(req.description, Some(None)));
}

#[test]
fn update_list_description_value_is_some_some() {
    let req: UpdateListRequest = serde_json::from_str(r#"{"description": "opis"}"#).unwrap();
    assert!(matches!(req.description, Some(Some(ref d)) if d == "opis"));
}

#[test]
fn set_list_placement_request_validates_non_empty_ids() {
    let req = SetListPlacementRequest {
        list_ids: vec![],
        parent_list_id: None,
        container_id: None,
    };
    assert_eq!(req.validate(), Err("list_ids must not be empty"));
}

#[test]
fn set_list_placement_request_rejects_two_targets() {
    let req = SetListPlacementRequest {
        list_ids: vec!["l1".into()],
        parent_list_id: Some("parent".into()),
        container_id: Some("container".into()),
    };
    assert_eq!(
        req.validate(),
        Err("parent_list_id and container_id are mutually exclusive")
    );
}

#[test]
fn set_list_placement_request_allows_root_target() {
    let req = SetListPlacementRequest {
        list_ids: vec!["l1".into(), "l2".into()],
        parent_list_id: None,
        container_id: None,
    };
    assert!(req.validate().is_ok());
}

#[test]
fn create_list_request_rejects_two_targets() {
    let req = CreateListRequest {
        name: "Test".into(),
        list_type: ListType::Checklist,
        features: None,
        parent_list_id: Some("parent".into()),
        container_id: Some("container".into()),
    };
    assert_eq!(
        req.validate_placement(),
        Err("parent_list_id and container_id are mutually exclusive")
    );
}

#[test]
fn create_list_request_allows_parent_list_id() {
    let req = CreateListRequest {
        name: "Sublist".into(),
        list_type: ListType::Custom,
        features: None,
        parent_list_id: Some("parent".into()),
        container_id: None,
    };
    assert!(req.validate_placement().is_ok());
}

#[test]
fn set_tag_links_request_requires_exactly_one_target_kind() {
    let req = SetTagLinksRequest {
        action: TagLinkAction::Assign,
        tag_ids: vec!["t1".into()],
        item_ids: Some(vec!["i1".into()]),
        list_ids: Some(vec!["l1".into()]),
    };
    assert_eq!(
        req.validate(),
        Err("item_ids and list_ids are mutually exclusive")
    );
}

#[test]
fn set_tag_links_request_requires_tags() {
    let req = SetTagLinksRequest {
        action: TagLinkAction::Assign,
        tag_ids: vec![],
        item_ids: Some(vec!["i1".into()]),
        list_ids: None,
    };
    assert_eq!(req.validate(), Err("tag_ids must not be empty"));
}

#[test]
fn set_tag_links_request_accepts_list_targets() {
    let req = SetTagLinksRequest {
        action: TagLinkAction::Remove,
        tag_ids: vec!["t1".into(), "t2".into()],
        item_ids: None,
        list_ids: Some(vec!["l1".into()]),
    };
    assert!(req.validate().is_ok());
}

// --- DateField ---

#[test]
fn date_field_column_names() {
    assert_eq!(DateField::StartDate.column_name(), "start_date");
    assert_eq!(DateField::Deadline.column_name(), "deadline");
    assert_eq!(DateField::HardDeadline.column_name(), "hard_deadline");
}

#[test]
fn date_field_time_columns() {
    assert_eq!(DateField::StartDate.time_column_name(), Some("start_time"));
    assert_eq!(
        DateField::Deadline.time_column_name(),
        Some("deadline_time")
    );
    assert_eq!(DateField::HardDeadline.time_column_name(), None);
}

#[test]
fn date_field_labels() {
    assert_eq!(DateField::StartDate.label(), "start_date");
    assert_eq!(DateField::Deadline.label(), "deadline");
    assert_eq!(DateField::HardDeadline.label(), "hard_deadline");
}

#[test]
fn date_field_serde() {
    let df: DateField = serde_json::from_str(r#""start_date""#).unwrap();
    assert_eq!(df, DateField::StartDate);
    assert_eq!(
        serde_json::to_string(&DateField::HardDeadline).unwrap(),
        r#""hard_deadline""#
    );
}
