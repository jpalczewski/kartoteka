use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct AddCommentParams {
    /// "item", "list", or "container"
    pub entity_type: String,
    pub entity_id: String,
    pub content: String,
    /// Display name shown as comment author (e.g. "Claude"). Omit to attribute to the logged-in user.
    pub author_name: Option<String>,
}
