pub mod auth_ctx;
pub mod constants;
pub mod date_utils;
pub mod locale;
pub mod redirect;
pub mod tag_utils;
pub use locale::Locale;
pub use redirect::safe_return_to;
pub use tag_utils::FilterMode;
#[allow(dead_code)]
pub(crate) mod deserializers;
pub mod dto;
pub use dto::*;
pub mod models;
pub use models::*;
pub mod types;
// Canonical item types live in `types` (they mirror the server's domain types and are what
// the frontend deserializes). Re-exported here so `kartoteka_shared::Item` keeps working.
pub use types::{
    Container, CreateContainerRequest, DateItem, HomeData, Item, MoveContainerRequest,
    UpdateContainerRequest,
};
pub(crate) mod validation;

use serde::{Deserialize, Deserializer, Serialize};

fn bool_from_number<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
        _ => Ok(false),
    }
}

// === Domain types ===

/// Known feature names
pub const FEATURE_QUANTITY: &str = "quantity";
pub const FEATURE_DEADLINES: &str = "deadlines";
pub const FEATURE_LOCATION: &str = "location";
pub const FEATURE_CHECKLIST: &str = "checklist";
pub const FEATURE_TIME_TRACKING: &str = "time_tracking";

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
    pub list_type: String,
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

// === Request DTOs ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: String,
    pub features: Option<Vec<ListFeature>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub list_type: Option<String>,
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

#[cfg(test)]
mod tests;
