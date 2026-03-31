use crate::*;

#[test]
fn container_status_serde() {
    assert_eq!(
        serde_json::to_string(&ContainerStatus::Active).unwrap(),
        r#""active""#
    );
    assert_eq!(
        serde_json::to_string(&ContainerStatus::Done).unwrap(),
        r#""done""#
    );
    assert_eq!(
        serde_json::to_string(&ContainerStatus::Paused).unwrap(),
        r#""paused""#
    );

    let cs: ContainerStatus = serde_json::from_str(r#""paused""#).unwrap();
    assert_eq!(cs, ContainerStatus::Paused);
}

#[test]
fn update_container_description_null_clears_field() {
    let req: UpdateContainerRequest = serde_json::from_str(r#"{"description": null}"#).unwrap();
    assert!(matches!(req.description, Some(None)));
}

#[test]
fn update_container_description_value_is_some_some() {
    let req: UpdateContainerRequest = serde_json::from_str(r#"{"description": "opis"}"#).unwrap();
    assert!(matches!(req.description, Some(Some(ref d)) if d == "opis"));
}
