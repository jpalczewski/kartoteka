use crate::models::*;

#[test]
fn search_entity_result_deserializes_optional_booleans() {
    let json = r#"{
        "entity_type": "item",
        "id": "i1",
        "name": "Milk",
        "description": "desc",
        "updated_at": "2024-01-01",
        "list_id": "l1",
        "list_name": "Groceries",
        "list_type": "checklist",
        "archived": 0.0,
        "completed": 1.0,
        "container_id": null,
        "parent_list_id": null,
        "parent_container_id": null,
        "status": null
    }"#;

    let result: SearchEntityResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.entity_type, SearchEntityType::Item);
    assert_eq!(result.archived, Some(false));
    assert_eq!(result.completed, Some(true));
}
