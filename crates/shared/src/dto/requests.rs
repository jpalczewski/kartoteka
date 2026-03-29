use serde::{Deserialize, Serialize};
use crate::models::{ContainerStatus, ListType, ListFeature};
use crate::deserializers::default_config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContainerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<Option<ContainerStatus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveListRequest {
    pub container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveContainerRequest {
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: ListType,
    pub features: Option<Vec<ListFeature>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub list_type: Option<ListType>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfigRequest {
    #[serde(default = "default_config")]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateItemRequest {
    pub title: String,
    pub description: Option<String>,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateItemRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
    pub position: Option<i32>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<Option<String>>,
    pub start_time: Option<Option<String>>,
    pub deadline: Option<Option<String>>,
    pub deadline_time: Option<Option<String>>,
    pub hard_deadline: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: String,
    pub parent_tag_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergeTagRequest {
    pub target_tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagAssignment {
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertSettingRequest {
    pub value: serde_json::Value,
}
