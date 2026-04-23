use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct CreateItemParams {
    /// Target list ID.
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub deadline: Option<String>,
    pub hard_deadline: Option<String>,
    pub start_time: Option<String>,
    pub deadline_time: Option<String>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub estimated_duration: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateItemParams {
    pub item_id: String,
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub completed: Option<bool>,
    pub start_date: Option<Option<String>>,
    pub deadline: Option<Option<String>>,
    pub hard_deadline: Option<Option<String>>,
    pub start_time: Option<Option<String>>,
    pub deadline_time: Option<Option<String>>,
    pub quantity: Option<Option<i32>>,
    pub actual_quantity: Option<Option<i32>>,
    pub unit: Option<Option<String>>,
    pub estimated_duration: Option<Option<i32>>,
}
