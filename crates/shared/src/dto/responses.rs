use crate::models::{Container, Item, List, ListFeature};
use serde::{Deserialize, Serialize};

/// Response from GET /api/me
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub is_admin: bool,
}

/// Response from GET /api/public/registration-mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationModeResponse {
    pub mode: String,
}

/// Response from POST /api/public/validate-invite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateInviteResponse {
    pub valid: bool,
    pub error: Option<String>,
}

/// Response from GET /api/lists/:list_id/items/:id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDetailResponse {
    #[serde(flatten)]
    pub item: Item,
    pub list_name: String,
    pub list_features: Vec<ListFeature>,
}

/// Response from GET /api/containers/:id/children
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerChildrenResponse {
    pub containers: Vec<Container>,
    pub lists: Vec<List>,
}

/// Response from GET /api/preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesResponse {
    pub locale: String,
}

/// Request body for PUT /api/preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferencesBody {
    pub locale: String,
}

/// Error response body returned by API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub status: u16,
}

/// Response from GET /api/home — matches actual API shape
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeData {
    #[serde(default)]
    pub pinned_lists: Vec<List>,
    #[serde(default)]
    pub pinned_containers: Vec<Container>,
    #[serde(default)]
    pub recent_lists: Vec<List>,
    #[serde(default)]
    pub recent_containers: Vec<Container>,
    #[serde(default)]
    pub root_containers: Vec<Container>,
    #[serde(default)]
    pub root_lists: Vec<List>,
}
