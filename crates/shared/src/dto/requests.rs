use crate::deserializers::{default_config, double_option};
use crate::models::{ContainerStatus, ListFeature, ListType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContainerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub status: Option<Option<ContainerStatus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveListRequest {
    pub container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetListPlacementRequest {
    pub list_ids: Vec<String>,
    pub parent_list_id: Option<String>,
    pub container_id: Option<String>,
}

impl SetListPlacementRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.list_ids.is_empty() {
            return Err("list_ids must not be empty");
        }
        if self.parent_list_id.is_some() && self.container_id.is_some() {
            return Err("parent_list_id and container_id are mutually exclusive");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveContainerRequest {
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderContainersRequest {
    pub container_ids: Vec<String>,
    pub parent_container_id: Option<String>,
}

impl ReorderContainersRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.container_ids.is_empty() {
            return Err("container_ids must not be empty");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: ListType,
    pub features: Option<Vec<ListFeature>>,
    pub parent_list_id: Option<String>,
    pub container_id: Option<String>,
}

impl CreateListRequest {
    pub fn validate_placement(&self) -> Result<(), &'static str> {
        if self.parent_list_id.is_some() && self.container_id.is_some() {
            return Err("parent_list_id and container_id are mutually exclusive");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_type: Option<ListType>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderItemsRequest {
    pub item_ids: Vec<String>,
}

impl ReorderItemsRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.item_ids.is_empty() {
            return Err("item_ids must not be empty");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_quantity: Option<i32>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub unit: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub start_date: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub start_time: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub deadline: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub deadline_time: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub hard_deadline: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(
        default,
        deserialize_with = "double_option",
        skip_serializing_if = "Option::is_none"
    )]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TagLinkAction {
    Assign,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetTagLinksRequest {
    pub action: TagLinkAction,
    pub tag_ids: Vec<String>,
    pub item_ids: Option<Vec<String>>,
    pub list_ids: Option<Vec<String>>,
}

impl SetTagLinksRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.tag_ids.is_empty() {
            return Err("tag_ids must not be empty");
        }
        let has_item_ids = self.item_ids.as_ref().is_some_and(|ids| !ids.is_empty());
        let has_list_ids = self.list_ids.as_ref().is_some_and(|ids| !ids.is_empty());
        match (has_item_ids, has_list_ids) {
            (true, false) | (false, true) => Ok(()),
            (false, false) => Err("provide item_ids or list_ids"),
            (true, true) => Err("item_ids and list_ids are mutually exclusive"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertSettingRequest {
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvitationCodeRequest {
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateInviteRequest {
    pub code: String,
    pub email: String,
}
