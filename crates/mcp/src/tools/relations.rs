use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct AddRelationParams {
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    /// "blocks" or "relates_to"
    pub relation_type: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct RemoveRelationParams {
    pub relation_id: String,
}
