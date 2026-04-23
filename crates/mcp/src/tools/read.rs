use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetListParams {
    pub list_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListItemsParams {
    pub list_id: String,
    /// Opaque cursor from previous response; omit for first page
    pub cursor: Option<String>,
    /// Max items to return (1–500, default 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetContainerParams {
    pub container_id: String,
}
