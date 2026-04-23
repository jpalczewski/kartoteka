use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct CreateListFromTemplateParams {
    pub template_id: String,
    pub list_name: String,
    /// e.g. "checklist", "custom"
    pub list_type: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SaveAsTemplateParams {
    pub list_id: String,
    pub template_name: String,
}
