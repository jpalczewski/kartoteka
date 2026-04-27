use crate::deserializers::optional_bool_from_number;
use serde::{Deserialize, Serialize};

use super::ContainerStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchEntityType {
    Item,
    List,
    Container,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEntityResult {
    pub entity_type: SearchEntityType,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub updated_at: String,
    #[serde(default)]
    pub list_id: Option<String>,
    #[serde(default)]
    pub list_name: Option<String>,
    #[serde(default)]
    pub list_type: Option<String>,
    #[serde(default, deserialize_with = "optional_bool_from_number")]
    pub archived: Option<bool>,
    #[serde(default, deserialize_with = "optional_bool_from_number")]
    pub completed: Option<bool>,
    #[serde(default)]
    pub container_id: Option<String>,
    #[serde(default)]
    pub parent_list_id: Option<String>,
    #[serde(default)]
    pub parent_container_id: Option<String>,
    #[serde(default)]
    pub status: Option<ContainerStatus>,
}
