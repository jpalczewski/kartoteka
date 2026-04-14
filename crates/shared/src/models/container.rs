use crate::deserializers::{bool_from_number, u32_from_number};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContainerStatus {
    Active,
    Done,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
    pub position: i32,
    #[serde(deserialize_with = "bool_from_number")]
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDetail {
    #[serde(flatten)]
    pub container: Container,
    #[serde(deserialize_with = "u32_from_number", default)]
    pub completed_items: u32,
    #[serde(deserialize_with = "u32_from_number", default)]
    pub total_items: u32,
    #[serde(deserialize_with = "u32_from_number", default)]
    pub completed_lists: u32,
    #[serde(deserialize_with = "u32_from_number", default)]
    pub total_lists: u32,
}
