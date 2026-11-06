use crate::deserializers::u32_from_number;
use crate::types::Container;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContainerStatus {
    Active,
    Done,
    Paused,
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
