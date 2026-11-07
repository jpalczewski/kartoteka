use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct StartTimerParams {
    pub item_id: Option<String>,
    /// "stopwatch" (default) or "pomodoro". Breaks are tracked by the client;
    /// the server only records which mode the work session belongs to.
    pub mode: Option<String>,
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

#[derive(Deserialize, JsonSchema)]
pub struct GetTimeSummaryParams {
    /// "item", "list" or "container". With `entity_id`, returns the recursive
    /// tracked-time total for that entity; omit both to list recent entries.
    pub entity_type: Option<String>,
    /// id of the item/list/container named by `entity_type`.
    pub entity_id: Option<String>,
}
