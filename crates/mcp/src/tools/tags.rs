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

#[derive(Deserialize, JsonSchema)]
pub struct CreateTagsInput {
    pub name: String,
    pub color: Option<String>,
    /// Real UUID of an existing parent tag.
    pub parent_tag_id: Option<String>,
    /// client_ref of a tag created earlier in this same batch.
    pub parent_tag_ref: Option<String>,
    /// Label that other entries in this batch can reference via parent_tag_ref.
    pub client_ref: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateTagsParams {
    pub tags: Vec<CreateTagsInput>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetTagEntitiesParams {
    pub tag_id: String,
    /// Filter by entity type: "item" or "list". Omit to get both.
    pub entity_type: Option<String>,
}
