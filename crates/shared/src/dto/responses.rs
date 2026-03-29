use serde::{Deserialize, Serialize};
use crate::models::{Item, ListFeature, Container, List};

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
