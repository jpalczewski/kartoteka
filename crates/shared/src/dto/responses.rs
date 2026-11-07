use crate::Locale;
use crate::types::{Container, Item, List, ListFeature};
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

// ── Delete preview ────────────────────────────────────────────────────────────

/// What a delete of a list/container would remove, used by the delete confirmation UI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletePreview {
    /// Descendant lists, excluding the list/container being deleted itself.
    pub lists: i64,
    /// Descendant containers, excluding the container being deleted itself.
    pub containers: i64,
    /// Items anywhere in the tree; a non-zero value blocks the deletion.
    pub items: i64,
}

// ── Location responses ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseLocationResponse {
    pub location_id: String,
    pub location: crate::models::Location,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HomeFilterResult {
    pub matching_list_ids: Vec<String>,
    pub matching_container_ids: Vec<String>,
    pub related_tag_ids: Vec<String>,
}

// ── Tag responses ─────────────────────────────────────────────────────────────

/// How many items and lists carry a tag — shown as the usage count in the tag tree
/// and summarised in the delete confirmation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagUsage {
    pub items: i64,
    pub lists: i64,
}

impl TagUsage {
    /// Items plus lists, i.e. the single number rendered next to a tag.
    #[must_use]
    pub const fn total(self) -> i64 {
        self.items + self.lists
    }
}
