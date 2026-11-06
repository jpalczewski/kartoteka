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
    Container, CreateContainerRequest, CreateListRequest, DateItem, HomeData, Item, ItemTagLink,
    List, ListFeature, ListTagLink, MoveContainerRequest, Tag, UpdateContainerRequest,
};
pub(crate) mod validation;

use serde::{Deserialize, Deserializer, Serialize};

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
