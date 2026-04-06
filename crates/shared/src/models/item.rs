use super::list::ListType;
use crate::deserializers::{bool_from_number, u32_from_number};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
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
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateItem {
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
    pub created_at: String,
    pub updated_at: String,
    pub list_name: String,
    pub list_type: ListType,
    #[serde(default)]
    pub date_type: Option<String>,
}

impl From<DateItem> for Item {
    fn from(di: DateItem) -> Self {
        Item {
            id: di.id,
            list_id: di.list_id,
            title: di.title,
            description: di.description,
            completed: di.completed,
            position: di.position,
            quantity: di.quantity,
            actual_quantity: di.actual_quantity,
            unit: di.unit,
            start_date: di.start_date,
            start_time: di.start_time,
            deadline: di.deadline,
            deadline_time: di.deadline_time,
            hard_deadline: di.hard_deadline,
            created_at: di.created_at,
            updated_at: di.updated_at,
        }
    }
}

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
    pub created_at: String,
    pub updated_at: String,
    pub list_name: String,
    pub list_type: ListType,
    #[serde(deserialize_with = "bool_from_number")]
    pub list_archived: bool,
    #[serde(default)]
    pub tag_ids: Vec<String>,
}
