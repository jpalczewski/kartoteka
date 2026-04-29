use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct CreateTagParams {
    pub name: String,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct AssignTagParams {
    /// "item", "list", or "container"
    pub entity_type: String,
    pub entity_id: String,
    pub tag_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct UnassignTagParams {
    /// "item", "list", or "container"
    pub entity_type: String,
    pub entity_id: String,
    pub tag_id: String,
}
