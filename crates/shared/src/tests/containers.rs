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
