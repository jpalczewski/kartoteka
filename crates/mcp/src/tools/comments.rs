use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct AddCommentParams {
    /// "item", "list", or "container"
    pub entity_type: String,
    pub entity_id: String,
    pub content: String,
}
