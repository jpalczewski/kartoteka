use crate::Locale;
use crate::models::{Container, Item, List, ListFeature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFieldError {
    pub field: String,
    pub code: String,
}

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
    pub locale: Locale,
}

/// Request body for PUT /api/preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferencesBody {
    pub locale: Locale,
}

/// Error response body returned by API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub fields: Vec<ValidationFieldError>,
}

/// A lightweight item preview used in container/list summary views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewItem {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub has_comments: bool,
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

// ── Location responses ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseLocationResponse {
    pub location_id: String,
    pub location: crate::models::Location,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HomeFilterResult {
    /// IDs of lists matching the active tag filter.
    pub matching_list_ids: Vec<String>,
    /// IDs of tags that co-occur with the selected tags on matching lists/items.
    pub related_tag_ids: Vec<String>,
}
