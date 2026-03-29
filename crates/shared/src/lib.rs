use serde::{Deserialize, Deserializer, Serialize};

fn bool_from_number<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
        _ => Ok(false),
    }
}

// === Container types ===

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

/// Combined home data returned by GET /api/home
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeItem {
    pub kind: String, // "list" or "container"
    pub id: String,
    pub name: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
    // list-specific
    pub list_type: Option<String>,
    // container-specific
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeData {
    pub pinned: Vec<HomeItem>,
    pub recent: Vec<HomeItem>,
    pub root_containers: Vec<Container>,
    pub root_lists: Vec<List>,
}

// === Domain types ===

/// Known feature names
pub const FEATURE_QUANTITY: &str = "quantity";
pub const FEATURE_DEADLINES: &str = "deadlines";

/// Date type identifiers used in date badge/editor components
pub const DATE_TYPE_START: &str = "start";
pub const DATE_TYPE_DEADLINE: &str = "deadline";
pub const DATE_TYPE_HARD_DEADLINE: &str = "hard_deadline";

pub const SETTING_MCP_AUTO_ENABLE_FEATURES: &str = "mcp_auto_enable_features";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSetting {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertSettingRequest {
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListFeature {
    pub name: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

fn features_from_json<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<ListFeature>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::String(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom),
        serde_json::Value::Array(_) => serde_json::from_value(v).map_err(serde::de::Error::custom),
        serde_json::Value::Null => Ok(vec![]),
        _ => Ok(vec![]),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    Checklist,
    Zakupy,
    Pakowanie,
    Terminarz,
    Custom,
}

impl ListType {
    pub fn default_features(&self) -> Vec<ListFeature> {
        match self {
            Self::Zakupy | Self::Pakowanie => vec![ListFeature {
                name: FEATURE_QUANTITY.into(),
                config: serde_json::json!({"unit_default": "szt"}),
            }],
            Self::Terminarz => vec![ListFeature {
                name: FEATURE_DEADLINES.into(),
                config: serde_json::json!({"has_start_date": false, "has_deadline": true, "has_hard_deadline": false}),
            }],
            _ => vec![],
        }
    }
}

/// Date field types for cross-list queries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DateField {
    StartDate,
    Deadline,
    HardDeadline,
}

impl DateField {
    pub fn column_name(&self) -> &'static str {
        match self {
            Self::StartDate => "start_date",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }

    pub fn time_column_name(&self) -> Option<&'static str> {
        match self {
            Self::StartDate => Some("start_time"),
            Self::Deadline => Some("deadline_time"),
            Self::HardDeadline => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::StartDate => "start",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub list_type: ListType,
    pub parent_list_id: Option<String>,
    pub position: i32,
    #[serde(deserialize_with = "bool_from_number")]
    pub archived: bool,
    #[serde(default, deserialize_with = "features_from_json")]
    pub features: Vec<ListFeature>,
    #[serde(default)]
    pub container_id: Option<String>,
    #[serde(default, deserialize_with = "bool_from_number")]
    pub pinned: bool,
    #[serde(default)]
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl List {
    pub fn has_feature(&self, name: &str) -> bool {
        self.features.iter().any(|f| f.name == name)
    }
}

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

// === Request DTOs ===

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

fn default_config() -> serde_json::Value {
    serde_json::json!({})
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

/// Response from GET /api/lists/:list_id/items/:id — item + list context in one call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDetailResponse {
    #[serde(flatten)]
    pub item: Item,
    pub list_name: String,
    pub list_features: Vec<ListFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub parent_tag_id: Option<String>,
    pub created_at: String,
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
pub struct ItemTagLink {
    pub item_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTagLink {
    pub list_id: String,
    pub tag_id: String,
}

// === Cross-list query types ===

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

// === Calendar types ===

fn u32_from_number<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) as u32),
        _ => Ok(0),
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

#[cfg(test)]
mod tests;
