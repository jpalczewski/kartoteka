use crate::dto::DeletePreview;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// An entity type that can be marked for deletion (the `/trash` queue).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletableEntity {
    Item,
    List,
    Container,
    Comment,
}

impl DeletableEntity {
    /// Every kind, in declaration order.
    pub const ALL: [DeletableEntity; 4] = [
        DeletableEntity::Item,
        DeletableEntity::List,
        DeletableEntity::Container,
        DeletableEntity::Comment,
    ];

    /// The value stored in `pending_deletions.entity_type` and used as the
    /// `NotFound(kind)` label.
    pub fn as_str(self) -> &'static str {
        match self {
            DeletableEntity::Item => "item",
            DeletableEntity::List => "list",
            DeletableEntity::Container => "container",
            DeletableEntity::Comment => "comment",
        }
    }
}

impl FromStr for DeletableEntity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "item" => Ok(DeletableEntity::Item),
            "list" => Ok(DeletableEntity::List),
            "container" => Ok(DeletableEntity::Container),
            "comment" => Ok(DeletableEntity::Comment),
            other => Err(format!("unknown deletable entity: {other}")),
        }
    }
}

impl fmt::Display for DeletableEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Identifies one queued row: `(entity_type, entity_id)` is the table's primary key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingDeletionKey {
    pub entity_type: DeletableEntity,
    pub entity_id: String,
}

/// One row of `/trash`, resolved for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDeletionItem {
    pub entity_type: DeletableEntity,
    pub entity_id: String,
    /// item title / list or container name / comment content cut to 120 chars
    /// (on char boundaries, ends with `…`).
    pub label: String,
    /// item -> list name; list -> container or parent list name; container -> parent
    /// container name; comment -> label of the commented entity. `None` at the root.
    pub context: Option<String>,
    /// Link to the entity in the UI: `/lists/{list_id}/items/{id}`, `/lists/{id}`,
    /// `/containers/{id}`; comment -> link to its entity.
    pub href: Option<String>,
    pub marked_at: String,
    /// list/container only: what goes along with it (descendants + items); `None`
    /// for item/comment.
    pub contains: Option<DeletePreview>,
}
