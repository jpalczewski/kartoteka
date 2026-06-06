use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveListToContainerParams {
    pub list_id: String,
    /// ID of the container to move the list into. Pass null to detach the list from its current container.
    pub container_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveContainerToParentParams {
    pub container_id: String,
    /// ID of the parent container to nest this container under. Pass null to move the container to root level.
    pub parent_container_id: Option<String>,
}
