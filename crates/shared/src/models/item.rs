use crate::deserializers::{bool_from_number, u32_from_number};
use crate::types::DateItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummary {
    pub date: String,
    #[serde(deserialize_with = "u32_from_number")]
    pub total: u32,
    #[serde(deserialize_with = "u32_from_number")]
    pub completed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayItems {
    pub date: String,
    pub items: Vec<DateItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItemResult {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "bool_from_number")]
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub location_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub list_name: String,
    pub list_type: String,
    #[serde(deserialize_with = "bool_from_number")]
    pub list_archived: bool,
    #[serde(default)]
    pub tag_ids: Vec<String>,
}
