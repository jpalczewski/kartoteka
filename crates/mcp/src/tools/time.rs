use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct StartTimerParams {
    pub item_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct LogTimeParams {
    pub item_id: Option<String>,
    /// Format: "YYYY-MM-DD HH:MM:SS"
    pub started_at: String,
    /// Format: "YYYY-MM-DD HH:MM:SS"
    pub ended_at: String,
    pub description: Option<String>,
}
