use crate::{DomainError, rules};
use kartoteka_db as db;
use kartoteka_db::types::DeleteTreeOutcome;
use kartoteka_shared::constants::{
    FEATURE_CHECKLIST, FEATURE_DEADLINES, FEATURE_LOCATION, FEATURE_QUANTITY, FEATURE_TIME_TRACKING,
};
use kartoteka_shared::types::ListFeature;
use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

// ── Public domain types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    Checklist,
    Shopping,
    Schedule,
    Log,
    Notes,
}

impl ListType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ListType::Checklist => "checklist",
            ListType::Shopping => "shopping",
            ListType::Schedule => "schedule",
            ListType::Log => "log",
            ListType::Notes => "notes",
        }
    }

    pub fn default_feature_names(&self) -> Vec<&'static str> {
        match self {
            ListType::Checklist => vec!["checklist"],
            ListType::Shopping => vec!["checklist", "quantity"],
            ListType::Schedule => vec!["checklist", "deadlines"],
            ListType::Log => vec!["time_tracking"],
            ListType::Notes => vec![],
        }
    }
}

impl std::str::FromStr for ListType {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "checklist" => Ok(ListType::Checklist),
            "shopping" => Ok(ListType::Shopping),
            "schedule" | "habits" => Ok(ListType::Schedule),
            "log" => Ok(ListType::Log),
            "notes" => Ok(ListType::Notes),
            _ => Err(DomainError::Validation("unknown_list_type")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    /// Accent colour as `#rrggbb`; None = neutral.
    pub color: Option<String>,
    pub description: Option<String>,
    pub list_type: String,
    pub parent_list_id: Option<String>,
    pub position: i64,
    pub archived: bool,
    pub container_id: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub location_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub features: Vec<ListFeature>,
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: Option<String>, // defaults to "checklist"
    pub icon: Option<String>,
    /// Accent colour as `#rrggbb`; None = neutral.
    pub color: Option<String>,
    pub description: Option<String>,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
    pub features: Vec<String>, // feature_name strings
}

#[derive(Debug, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub icon: Option<Option<String>>,  // Some(None) clears the field
    pub color: Option<Option<String>>, // Accent colour: Some(None) = neutral
    pub description: Option<Option<String>>,
    pub list_type: Option<String>,
    pub location_id: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct MoveListRequest {
    pub position: i64,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetFeaturesRequest {
    pub features: Vec<String>,
}

// ── Conversion from db row ────────────────────────────────────────────────────

fn parse_features_map(
    json: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, DomainError> {
    serde_json::from_str(json).map_err(|e| DomainError::Internal(e.to_string()))
}

pub(crate) fn parse_features(json: &str) -> Result<Vec<ListFeature>, DomainError> {
    Ok(parse_features_map(json)?
        .into_iter()
        .map(|(feature_name, config)| ListFeature {
            feature_name,
            config,
        })
        .collect())
}

/// Build a features JSON object from a list of names (each gets an empty config).
pub fn features_from_names(names: &[String]) -> serde_json::Value {
    let obj: serde_json::Map<String, serde_json::Value> = names
        .iter()
        .map(|n| (n.clone(), serde_json::json!({})))
        .collect();
    serde_json::Value::Object(obj)
}

/// Every feature name a list may enable.
pub const KNOWN_FEATURES: [&str; 5] = [
    FEATURE_CHECKLIST,
    FEATURE_DEADLINES,
    FEATURE_LOCATION,
    FEATURE_QUANTITY,
    FEATURE_TIME_TRACKING,
];

/// Reject feature names outside [`KNOWN_FEATURES`].
pub fn validate_feature_names(names: &[String]) -> Result<(), DomainError> {
    if names.iter().any(|n| !KNOWN_FEATURES.contains(&n.as_str())) {
        return Err(DomainError::Validation("unknown_feature"));
    }
    Ok(())
}

/// Build the feature map for a whole-set replacement, keeping the stored config
/// of every feature that stays enabled. A newly enabled feature gets `{}`.
pub fn features_preserving_configs(
    existing: &serde_json::Map<String, serde_json::Value>,
    names: &[String],
) -> serde_json::Value {
    let obj: serde_json::Map<String, serde_json::Value> = names
        .iter()
        .map(|n| {
            let config = existing
                .get(n)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            (n.clone(), config)
        })
        .collect();
    serde_json::Value::Object(obj)
}

/// Parse a caller-supplied list type and resolve the feature names to persist.
///
/// `list_type` defaults to `checklist`; the type's default features apply only
/// when the caller supplied no explicit feature list (`None`). An explicit
/// empty list means "no features" and is preserved.
pub fn resolve_list_type(
    list_type: Option<&str>,
    features: Option<&[String]>,
) -> Result<(ListType, Vec<String>), DomainError> {
    let list_type: ListType = list_type.unwrap_or("checklist").parse()?;
    let feature_names: Vec<String> = match features {
        Some(names) => names.to_vec(),
        None => list_type
            .default_feature_names()
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };
    Ok((list_type, feature_names))
}

/// Insert a list row plus its feature rows inside a caller-owned transaction.
pub async fn insert_list_in(
    conn: &mut SqliteConnection,
    input: &db::lists::InsertListInput,
    feature_names: &[String],
) -> Result<(), DomainError> {
    db::lists::insert(conn, input).await?;
    if !feature_names.is_empty() {
        db::lists::set_features(conn, &input.id, &features_from_names(feature_names)).await?;
    }
    Ok(())
}

pub(crate) fn row_to_list(row: db::lists::ListRow) -> Result<List, DomainError> {
    let features = parse_features(&row.features)?;
    Ok(List {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        icon: row.icon,
        color: row.color,
        description: row.description,
        list_type: row.list_type,
        parent_list_id: row.parent_list_id,
        position: row.position,
        archived: row.archived != 0,
        container_id: row.container_id,
        pinned: row.pinned != 0,
        last_opened_at: row.last_opened_at,
        location_id: row.location_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
        features,
    })
}

// ── Orchestration ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<List>, DomainError> {
    let rows = db::lists::list_all(pool, user_id).await?;
    rows.into_iter().map(row_to_list).collect()
}

#[tracing::instrument(skip(pool))]
pub async fn list_archived(pool: &SqlitePool, user_id: &str) -> Result<Vec<List>, DomainError> {
    let rows = db::lists::list_archived(pool, user_id).await?;
    rows.into_iter().map(row_to_list).collect()
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<List>, DomainError> {
    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
}

/// Record that the user opened a list (powers the home "recent" section).
/// Only `last_opened_at` is written; `updated_at` is left untouched.
#[tracing::instrument(skip(pool))]
pub async fn touch_last_opened(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    db::lists::touch_last_opened(pool, id, user_id).await?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn sublists(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<List>, DomainError> {
    let rows = db::lists::sublists(pool, parent_id, user_id).await?;
    rows.into_iter().map(row_to_list).collect()
}

/// Verify that an optional `parent_list_id` / `container_id` pair refers to
/// entities owned by `user_id`.
///
/// `create` and `move_list` persist these ids verbatim, so without this check a
/// caller could nest their list under another user's list or container.
#[tracing::instrument(skip(pool))]
pub async fn ensure_placement_owned(
    pool: &SqlitePool,
    user_id: &str,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
) -> Result<(), DomainError> {
    if let Some(parent_id) = parent_list_id {
        db::lists::get_one(pool, parent_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("list"))?;
    }
    if let Some(container_id) = container_id {
        db::containers::get_one(pool, container_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("container"))?;
    }
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateListRequest,
) -> Result<List, DomainError> {
    // Phase 1: READ / THINK — the parent list and container must belong to the caller.
    ensure_placement_owned(
        pool,
        user_id,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    )
    .await?;

    DomainError::ensure_unique(
        db::lists::find_id_by_name_in_scope(
            pool,
            user_id,
            &req.name,
            req.container_id.as_deref(),
            req.parent_list_id.as_deref(),
            None,
        )
        .await?,
        "list",
    )?;

    let list_type_str = req.list_type.as_deref().unwrap_or("checklist");
    let (list_type, feature_names) = resolve_list_type(
        Some(list_type_str),
        if req.features.is_empty() {
            None
        } else {
            Some(req.features.as_slice())
        },
    )?;

    let icon = rules::appearance::normalize_icon(req.icon.as_deref())?;
    let color = rules::appearance::normalize_color(req.color.as_deref())?;

    let position = db::lists::next_position(
        pool,
        user_id,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    )
    .await?;

    let list_id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    insert_list_in(
        &mut tx,
        &db::lists::InsertListInput {
            id: list_id.clone(),
            user_id: user_id.to_owned(),
            position,
            name: req.name.clone(),
            icon,
            color,
            description: req.description.clone(),
            list_type: list_type.as_str().to_owned(),
            container_id: req.container_id.clone(),
            parent_list_id: req.parent_list_id.clone(),
        },
        &feature_names,
    )
    .await?;
    tx.commit().await.map_err(db::DbError::Sqlx)?;

    db::lists::get_one(pool, &list_id, user_id)
        .await?
        .map(row_to_list)
        .transpose()?
        .ok_or_else(|| DomainError::Internal("list disappeared after create".into()))
}

/// Set the icon and/or accent colour of a list.
///
/// Both arguments use the `Option<Option<_>>` patch convention: `None` keeps the
/// stored value, `Some(None)` clears it, `Some(Some(v))` sets it. Colours must
/// be `#rrggbb`; icons are trimmed and capped (see [`rules::appearance`]).
#[tracing::instrument(skip(pool, icon, color), fields(list_id = %id))]
pub async fn set_appearance(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    icon: Option<Option<String>>,
    color: Option<Option<String>>,
) -> Result<Option<List>, DomainError> {
    if icon.is_none() && color.is_none() {
        return get_one(pool, id, user_id).await;
    }
    let req = UpdateListRequest {
        name: None,
        icon,
        color,
        description: None,
        list_type: None,
        location_id: None,
    };
    update(pool, id, user_id, &req).await
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateListRequest,
) -> Result<Option<List>, DomainError> {
    // Phase 1: READ — need current list to check it exists before update
    let current = match db::lists::get_one(pool, id, user_id).await? {
        Some(c) => c,
        None => return Ok(None),
    };

    // Phase 2: THINK — duplicate name check
    if let Some(ref new_name) = req.name {
        DomainError::ensure_unique(
            db::lists::find_id_by_name_in_scope(
                pool,
                user_id,
                new_name,
                current.container_id.as_deref(),
                current.parent_list_id.as_deref(),
                Some(id),
            )
            .await?,
            "list",
        )?;
    }

    crate::locations::ensure_owned(pool, &req.location_id, user_id).await?;
    let icon = req
        .icon
        .as_ref()
        .map(|v| rules::appearance::normalize_icon(v.as_deref()))
        .transpose()?;
    let color = req
        .color
        .as_ref()
        .map(|v| rules::appearance::normalize_color(v.as_deref()))
        .transpose()?;

    // Phase 3: WRITE
    let updated = db::lists::update(
        pool,
        id,
        user_id,
        &db::lists::UpdateListInput {
            name: req.name.clone(),
            icon,
            color,
            description: req.description.clone(),
            list_type: req.list_type.clone(),
            location_id: req.location_id.clone(),
        },
    )
    .await?;

    if !updated {
        return Ok(None);
    }

    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
}

/// Preview what deleting this list would remove.
#[tracing::instrument(skip(pool))]
pub async fn delete_preview(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<kartoteka_shared::DeletePreview, DomainError> {
    let summary = db::lists::subtree_summary(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("list"))?;
    Ok(crate::delete_preview(summary))
}

/// Delete a list together with its whole subtree.
///
/// Deletion is refused while any list in the subtree still holds items. A subtree that
/// only has descendants (sublists) requires `force`; forcing never bypasses the item
/// check. The item check is repeated inside the delete transaction, so a concurrent
/// insert cannot slip through.
#[tracing::instrument(skip(pool))]
pub async fn delete(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    force: bool,
) -> Result<(), DomainError> {
    let summary = db::lists::subtree_summary(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("list"))?;

    if summary.items > 0 {
        return Err(DomainError::NotEmpty {
            kind: "list",
            items: summary.items,
        });
    }
    if !force && summary.lists > 0 {
        return Err(DomainError::HasChildren {
            kind: "list",
            lists: summary.lists,
            containers: summary.containers,
        });
    }

    match db::lists::delete_tree(pool, id, user_id, false).await? {
        DeleteTreeOutcome::Deleted => Ok(()),
        DeleteTreeOutcome::NotFound => Err(DomainError::NotFound("list")),
        DeleteTreeOutcome::NotEmpty { items } => Err(DomainError::NotEmpty {
            kind: "list",
            items,
        }),
    }
}

/// Archive or unarchive a list to an explicit state.
///
/// Idempotent: calling it twice with the same `archived` leaves one state. The
/// `Option` is `None` when the list does not belong to `user_id`.
#[tracing::instrument(skip(pool), fields(list_id = %id, archived))]
pub async fn set_archived(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    archived: bool,
) -> Result<Option<List>, DomainError> {
    if !db::lists::set_archived(pool, id, user_id, archived).await? {
        return Ok(None);
    }
    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
}

/// Pin or unpin a list to an explicit state (idempotent, see [`set_archived`]).
#[tracing::instrument(skip(pool), fields(list_id = %id, pinned))]
pub async fn set_pinned(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    pinned: bool,
) -> Result<Option<List>, DomainError> {
    if !db::lists::set_pinned(pool, id, user_id, pinned).await? {
        return Ok(None);
    }
    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_archive(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<List>, DomainError> {
    let Some(current) = db::lists::get_one(pool, id, user_id).await? else {
        return Ok(None);
    };
    set_archived(pool, id, user_id, current.archived == 0).await
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<List>, DomainError> {
    let Some(current) = db::lists::get_one(pool, id, user_id).await? else {
        return Ok(None);
    };
    set_pinned(pool, id, user_id, current.pinned == 0).await
}

/// Mark all items in the list as not completed (keep items and list intact).
#[tracing::instrument(skip(pool))]
pub async fn reset(pool: &SqlitePool, id: &str, user_id: &str) -> Result<u64, DomainError> {
    db::lists::get_one(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("list"))?;
    Ok(db::lists::uncheck_items(pool, id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn move_list(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &MoveListRequest,
) -> Result<Option<List>, DomainError> {
    let current = match db::lists::get_one(pool, id, user_id).await? {
        Some(c) => c,
        None => return Ok(None),
    };
    // The destination container / parent list must belong to the caller.
    ensure_placement_owned(
        pool,
        user_id,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    )
    .await?;
    DomainError::ensure_unique(
        db::lists::find_id_by_name_in_scope(
            pool,
            user_id,
            &current.name,
            req.container_id.as_deref(),
            req.parent_list_id.as_deref(),
            Some(id),
        )
        .await?,
        "list",
    )?;
    let moved = db::lists::move_list(
        pool,
        id,
        user_id,
        req.position,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    )
    .await?;
    if !moved {
        return Ok(None);
    }
    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
}

/// Rewrite list positions inside one scope, atomically.
///
/// The scope is given by `container_id` and/or `parent_list_id` (both `None` =
/// root) and must belong to `user_id`. Every id in `ordered_ids` must be a list
/// owned by `user_id` that already sits in that scope. A list can never be made
/// its own parent and no cycle may be created, otherwise nothing is written.
#[tracing::instrument(skip(pool, ordered_ids), fields(container_id = ?container_id, parent_list_id = ?parent_list_id, count = ordered_ids.len()))]
pub async fn reorder(
    pool: &SqlitePool,
    user_id: &str,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
    ordered_ids: &[String],
) -> Result<(), DomainError> {
    // Phase 1 READ / THINK: scope ownership, self-parent, cycles
    ensure_placement_owned(pool, user_id, container_id, parent_list_id).await?;
    if let Some(parent_id) = parent_list_id {
        for id in ordered_ids {
            rules::lists::validate_move(id, Some(parent_id))?;
            if db::lists::is_descendant(pool, user_id, id, parent_id).await? {
                return Err(DomainError::Validation("circular_list_move"));
            }
        }
    }
    let scope = db::lists::ids_in_scope(pool, user_id, container_id, parent_list_id).await?;
    DomainError::ensure_ids_in_scope(&scope, ordered_ids, "list")?;

    // Phase 3 WRITE: one transaction, all or nothing
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    for (pos, id) in ordered_ids.iter().enumerate() {
        let moved = db::lists::move_list_in_tx(
            &mut tx,
            id,
            user_id,
            pos as i64,
            container_id,
            parent_list_id,
        )
        .await?;
        if !moved {
            return Err(DomainError::NotFound("list"));
        }
    }
    tx.commit().await.map_err(db::DbError::Sqlx)?;
    Ok(())
}

/// Replace all features for a list.
///
/// The config of a feature that stays enabled is kept, so enabling `location`
/// does not wipe the `deadlines` config; a newly enabled feature starts with
/// `{}`. Names outside [`KNOWN_FEATURES`] are rejected.
#[tracing::instrument(skip(pool))]
pub async fn set_features(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &SetFeaturesRequest,
) -> Result<Option<List>, DomainError> {
    // Phase 2: THINK — unknown names are rejected before anything is written
    validate_feature_names(&req.features)?;

    // Phase 1: READ — need the current features to keep their configs
    let Some(current) = db::lists::get_one(pool, id, user_id).await? else {
        return Ok(None);
    };
    let features =
        features_preserving_configs(&parse_features_map(&current.features)?, &req.features);

    // Phase 3: WRITE
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    db::lists::set_features(&mut tx, id, &features).await?;
    tx.commit().await.map_err(db::DbError::Sqlx)?;

    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
}

/// Update the config of a single feature without affecting other features.
///
/// The config must be a JSON object; anything else is
/// `DomainError::Validation("invalid_feature_config")`. Returns
/// `DomainError::NotFound("feature")` if the feature is not enabled on the list.
#[tracing::instrument(skip(pool, config))]
pub async fn update_feature_config(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    feature_name: &str,
    config: serde_json::Value,
) -> Result<(), DomainError> {
    // Phase 2: THINK — the config is an object keyed by feature options
    if !config.is_object() {
        return Err(DomainError::Validation("invalid_feature_config"));
    }

    // Phase 1: READ
    let current = db::lists::get_one(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("list"))?;

    // Phase 2: THINK
    let mut features = parse_features_map(&current.features)?;
    if !features.contains_key(feature_name) {
        return Err(DomainError::NotFound("feature"));
    }
    features.insert(feature_name.to_string(), config);

    // Phase 3: WRITE
    let new_features = serde_json::Value::Object(features);
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    db::lists::set_features(&mut tx, id, &new_features).await?;
    tx.commit().await.map_err(db::DbError::Sqlx)?;
    Ok(())
}

// Re-export CreateItemContext so server/mcp don't import db directly
pub use db::lists::CreateItemContext;

pub async fn get_create_item_context(
    pool: &SqlitePool,
    list_id: &str,
    user_id: &str,
) -> Result<Option<CreateItemContext>, DomainError> {
    Ok(db::lists::get_create_item_context(pool, list_id, user_id).await?)
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    fn checklist_req(name: &str) -> CreateListRequest {
        CreateListRequest {
            name: name.to_string(),
            list_type: Some("checklist".into()),
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            features: vec![],
            color: None,
        }
    }

    #[tokio::test]
    async fn create_checklist_gets_default_checklist_feature() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Todo")).await.unwrap();

        assert_eq!(list.name, "Todo");
        assert_eq!(list.list_type, "checklist");
        let names: Vec<&str> = list
            .features
            .iter()
            .map(|f| f.feature_name.as_str())
            .collect();
        assert!(names.contains(&"checklist"));
        assert!(!list.archived);
        assert!(!list.pinned);
    }

    #[tokio::test]
    async fn create_with_features_roundtrip() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Deadlined".into(),
            list_type: Some("checklist".into()),
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            features: vec!["deadlines".into(), "quantity".into()],
            color: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(list.features.len(), 2);
        let names: Vec<&str> = list
            .features
            .iter()
            .map(|f| f.feature_name.as_str())
            .collect();
        assert!(names.contains(&"deadlines"));
        assert!(names.contains(&"quantity"));
    }

    #[tokio::test]
    async fn create_shopping_no_explicit_features_gets_checklist_and_quantity() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Groceries".into(),
            list_type: Some("shopping".into()),
            features: vec![],
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            color: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();
        let names: Vec<&str> = list
            .features
            .iter()
            .map(|f| f.feature_name.as_str())
            .collect();
        assert!(names.contains(&"checklist"));
        assert!(names.contains(&"quantity"));
    }

    #[tokio::test]
    async fn create_notes_has_no_features() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "My Notes".into(),
            list_type: Some("notes".into()),
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            features: vec![],
            color: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(list.list_type, "notes");
        assert!(list.features.is_empty());
    }

    #[tokio::test]
    async fn create_schedule_gets_checklist_and_deadlines() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "My Schedule".into(),
            list_type: Some("schedule".into()),
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            features: vec![],
            color: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(list.list_type, "schedule");
        let names: Vec<&str> = list
            .features
            .iter()
            .map(|f| f.feature_name.as_str())
            .collect();
        assert!(names.contains(&"checklist"));
        assert!(names.contains(&"deadlines"));
    }

    #[tokio::test]
    async fn habits_alias_resolves_to_schedule() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Old Habits".into(),
            list_type: Some("habits".into()),
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            features: vec![],
            color: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(list.list_type, "schedule");
    }

    #[tokio::test]
    async fn toggle_archive_flips_and_returns_updated() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Archivable"))
            .await
            .unwrap();

        let updated = toggle_archive(&pool, &list.id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.archived);

        let updated2 = toggle_archive(&pool, &list.id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert!(!updated2.archived);
    }

    #[tokio::test]
    async fn reset_marks_items_incomplete_not_deleted() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Resettable"))
            .await
            .unwrap();

        // Insert 2 items, one already completed
        sqlx::query(
            "INSERT INTO items (id, list_id, title, completed) VALUES ('i1', ?, 'A', 1), ('i2', ?, 'B', 0)",
        )
        .bind(&list.id)
        .bind(&list.id)
        .execute(&pool)
        .await
        .unwrap();

        let affected = reset(&pool, &list.id, &uid).await.unwrap();
        // Only the completed item needed updating, but the exact count depends on DB behaviour;
        // what matters is ≥ 0 and no error.
        let _ = affected;

        // List still exists
        let found = get_one(&pool, &list.id, &uid).await.unwrap();
        assert!(found.is_some());

        // Items still exist
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE list_id = ?")
            .bind(&list.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 2);

        // All items are not completed
        let completed_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM items WHERE list_id = ? AND completed = 1")
                .bind(&list.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(completed_count.0, 0);
    }

    #[tokio::test]
    async fn reset_wrong_user_returns_not_found() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Mine")).await.unwrap();

        let err = reset(&pool, &list.id, &other).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound("list")));
    }

    #[tokio::test]
    async fn list_all_excludes_archived() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let l1 = create(&pool, &uid, &checklist_req("Active")).await.unwrap();
        create(&pool, &uid, &checklist_req("Active2"))
            .await
            .unwrap();
        toggle_archive(&pool, &l1.id, &uid).await.unwrap();

        let lists = list_all(&pool, &uid).await.unwrap();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].name, "Active2");
    }

    #[tokio::test]
    async fn positions_increment_on_create() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let l1 = create(&pool, &uid, &checklist_req("First")).await.unwrap();
        let l2 = create(&pool, &uid, &checklist_req("Second")).await.unwrap();
        assert_eq!(l1.position, 0);
        assert_eq!(l2.position, 1);
    }

    #[tokio::test]
    async fn create_duplicate_name_returns_already_exists_with_id() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let first = create(&pool, &uid, &checklist_req("Zakupy")).await.unwrap();
        let err = create(&pool, &uid, &checklist_req("zakupy"))
            .await
            .unwrap_err();
        match err {
            DomainError::AlreadyExists { kind, id } => {
                assert_eq!(kind, "list");
                assert_eq!(id, first.id);
            }
            _ => panic!("expected AlreadyExists, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn create_same_name_different_user_ok() {
        let pool = test_pool().await;
        let uid1 = create_test_user(&pool).await;
        let uid2 = create_test_user(&pool).await;
        create(&pool, &uid1, &checklist_req("Praca")).await.unwrap();
        assert!(create(&pool, &uid2, &checklist_req("Praca")).await.is_ok());
    }

    #[tokio::test]
    async fn update_rename_to_existing_name_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let first = create(&pool, &uid, &checklist_req("Zakupy")).await.unwrap();
        let second = create(&pool, &uid, &checklist_req("Praca")).await.unwrap();
        let err = update(
            &pool,
            &second.id,
            &uid,
            &UpdateListRequest {
                name: Some("zakupy".into()),
                icon: None,
                description: None,
                list_type: None,
                location_id: None,
                color: None,
            },
        )
        .await
        .unwrap_err();
        match err {
            DomainError::AlreadyExists { id, .. } => assert_eq!(id, first.id),
            _ => panic!("expected AlreadyExists, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn update_same_name_as_self_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Zakupy")).await.unwrap();
        assert!(
            update(
                &pool,
                &list.id,
                &uid,
                &UpdateListRequest {
                    name: Some("Zakupy".into()),
                    icon: None,
                    description: None,
                    list_type: None,
                    location_id: None,
                    color: None,
                },
            )
            .await
            .is_ok()
        );
    }

    async fn make_container(pool: &SqlitePool, user_id: &str, name: &str) -> String {
        crate::containers::create(
            pool,
            user_id,
            &kartoteka_shared::types::CreateContainerRequest {
                name: name.to_string(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: None,
                color: None,
            },
        )
        .await
        .unwrap()
        .id
    }

    fn sublist_req(name: &str, parent_list_id: &str) -> CreateListRequest {
        CreateListRequest {
            name: name.to_string(),
            list_type: Some("checklist".into()),
            icon: None,
            color: None,
            description: None,
            container_id: None,
            parent_list_id: Some(parent_list_id.to_string()),
            features: vec![],
        }
    }

    async fn make_item(pool: &SqlitePool, list_id: &str, title: &str) {
        sqlx::query("INSERT INTO items (id, list_id, title, completed) VALUES (?, ?, ?, 0)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(list_id)
            .bind(title)
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_sublist_under_own_list_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = create(&pool, &uid, &checklist_req("Parent")).await.unwrap();

        let mut req = checklist_req("Child");
        req.parent_list_id = Some(parent.id.clone());
        let child = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(child.parent_list_id.as_deref(), Some(parent.id.as_str()));
    }

    #[tokio::test]
    async fn create_sublist_under_other_users_list_is_rejected() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let parent = create(&pool, &owner, &checklist_req("Parent"))
            .await
            .unwrap();

        let mut req = checklist_req("Child");
        req.parent_list_id = Some(parent.id.clone());
        let err = create(&pool, &intruder, &req).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound("list")), "got {err:?}");

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM lists WHERE user_id = ?")
            .bind(&intruder)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn create_in_own_container_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container_id = make_container(&pool, &uid, "Box").await;

        let mut req = checklist_req("Inside");
        req.container_id = Some(container_id.clone());
        let list = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(list.container_id.as_deref(), Some(container_id.as_str()));
    }

    #[tokio::test]
    async fn create_in_other_users_container_is_rejected() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let container_id = make_container(&pool, &owner, "Box").await;

        let mut req = checklist_req("Inside");
        req.container_id = Some(container_id);
        let err = create(&pool, &intruder, &req).await.unwrap_err();
        assert!(
            matches!(err, DomainError::NotFound("container")),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn move_list_under_own_list_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = create(&pool, &uid, &checklist_req("Parent")).await.unwrap();
        let child = create(&pool, &uid, &checklist_req("Child")).await.unwrap();

        let moved = move_list(
            &pool,
            &child.id,
            &uid,
            &MoveListRequest {
                position: 0,
                container_id: None,
                parent_list_id: Some(parent.id.clone()),
            },
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(moved.parent_list_id.as_deref(), Some(parent.id.as_str()));
    }

    #[tokio::test]
    async fn move_list_under_other_users_list_is_rejected() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let parent = create(&pool, &owner, &checklist_req("Parent"))
            .await
            .unwrap();
        let list = create(&pool, &intruder, &checklist_req("Mine"))
            .await
            .unwrap();

        let err = move_list(
            &pool,
            &list.id,
            &intruder,
            &MoveListRequest {
                position: 0,
                container_id: None,
                parent_list_id: Some(parent.id.clone()),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::NotFound("list")), "got {err:?}");

        let unchanged = get_one(&pool, &list.id, &intruder).await.unwrap().unwrap();
        assert!(unchanged.parent_list_id.is_none());
    }

    #[tokio::test]
    async fn move_list_into_other_users_container_is_rejected() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let container_id = make_container(&pool, &owner, "Box").await;
        let list = create(&pool, &intruder, &checklist_req("Mine"))
            .await
            .unwrap();

        let err = move_list(
            &pool,
            &list.id,
            &intruder,
            &MoveListRequest {
                position: 0,
                container_id: Some(container_id),
                parent_list_id: None,
            },
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainError::NotFound("container")),
            "got {err:?}"
        );
    }

    // ── reorder (atomic, authorization-checked) ─────────────────────────────

    async fn stored_placement(
        pool: &SqlitePool,
        user_id: &str,
        id: &str,
    ) -> (Option<String>, Option<String>, i64) {
        let row = db::lists::get_one(pool, id, user_id)
            .await
            .unwrap()
            .expect("list row");
        (row.container_id, row.parent_list_id, row.position)
    }

    #[tokio::test]
    async fn reorder_root_lists_assigns_positions_in_given_order() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let a = create(&pool, &uid, &checklist_req("A")).await.unwrap();
        let b = create(&pool, &uid, &checklist_req("B")).await.unwrap();
        let c = create(&pool, &uid, &checklist_req("C")).await.unwrap();

        reorder(
            &pool,
            &uid,
            None,
            None,
            &[c.id.clone(), b.id.clone(), a.id.clone()],
        )
        .await
        .unwrap();

        let ordered = db::lists::ids_in_scope(&pool, &uid, None, None)
            .await
            .unwrap();
        assert_eq!(ordered, vec![c.id.clone(), b.id.clone(), a.id.clone()]);
        for (expected, id) in [&c.id, &b.id, &a.id].iter().enumerate() {
            assert_eq!(
                stored_placement(&pool, &uid, id).await,
                (None, None, expected as i64)
            );
        }
    }

    #[tokio::test]
    async fn reorder_lists_inside_container_keeps_scope() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container_id = make_container(&pool, &uid, "Box").await;
        let a = create(
            &pool,
            &uid,
            &CreateListRequest {
                container_id: Some(container_id.clone()),
                color: None,
                ..checklist_req("A")
            },
        )
        .await
        .unwrap();
        let b = create(
            &pool,
            &uid,
            &CreateListRequest {
                container_id: Some(container_id.clone()),
                color: None,
                ..checklist_req("B")
            },
        )
        .await
        .unwrap();

        reorder(
            &pool,
            &uid,
            Some(&container_id),
            None,
            &[b.id.clone(), a.id.clone()],
        )
        .await
        .unwrap();

        assert_eq!(
            stored_placement(&pool, &uid, &b.id).await,
            (Some(container_id.clone()), None, 0)
        );
        assert_eq!(
            stored_placement(&pool, &uid, &a.id).await,
            (Some(container_id), None, 1)
        );
    }

    #[tokio::test]
    async fn reorder_making_list_its_own_parent_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Self")).await.unwrap();

        let err = reorder(
            &pool,
            &uid,
            None,
            Some(&list.id),
            std::slice::from_ref(&list.id),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainError::Validation("cannot_move_to_self")),
            "got {err:?}"
        );
        assert_eq!(
            stored_placement(&pool, &uid, &list.id).await,
            (None, None, 0)
        );
    }

    #[tokio::test]
    async fn reorder_creating_list_cycle_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let a = create(&pool, &uid, &checklist_req("A")).await.unwrap();
        let b = create(
            &pool,
            &uid,
            &CreateListRequest {
                parent_list_id: Some(a.id.clone()),
                color: None,
                ..checklist_req("B")
            },
        )
        .await
        .unwrap();

        // `a` is the parent of `b`, so nesting `a` under `b` would be a cycle.
        let err = reorder(&pool, &uid, None, Some(&b.id), std::slice::from_ref(&a.id))
            .await
            .unwrap_err();
        assert!(
            matches!(err, DomainError::Validation("circular_list_move")),
            "got {err:?}"
        );
        assert_eq!(stored_placement(&pool, &uid, &a.id).await, (None, None, 0));
    }

    #[tokio::test]
    async fn reorder_with_foreign_id_writes_nothing() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let a = create(&pool, &uid, &checklist_req("A")).await.unwrap();
        let b = create(&pool, &uid, &checklist_req("B")).await.unwrap();
        let foreign = create(&pool, &intruder, &checklist_req("X")).await.unwrap();

        // `b` first: a partial write would already have changed its position.
        let err = reorder(
            &pool,
            &uid,
            None,
            None,
            &[b.id.clone(), foreign.id.clone(), a.id.clone()],
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::NotFound("list")), "got {err:?}");
        assert_eq!(stored_placement(&pool, &uid, &b.id).await, (None, None, 1));
        assert_eq!(stored_placement(&pool, &uid, &a.id).await, (None, None, 0));
    }

    #[tokio::test]
    async fn reorder_with_id_from_other_scope_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container_id = make_container(&pool, &uid, "Box").await;
        let inside = create(
            &pool,
            &uid,
            &CreateListRequest {
                container_id: Some(container_id.clone()),
                color: None,
                ..checklist_req("Inside")
            },
        )
        .await
        .unwrap();
        let root_list = create(&pool, &uid, &checklist_req("Root")).await.unwrap();

        let err = reorder(
            &pool,
            &uid,
            Some(&container_id),
            None,
            &[inside.id.clone(), root_list.id.clone()],
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::NotFound("list")), "got {err:?}");
        assert_eq!(
            stored_placement(&pool, &uid, &inside.id).await,
            (Some(container_id), None, 0)
        );
        assert_eq!(
            stored_placement(&pool, &uid, &root_list.id).await,
            (None, None, 0)
        );
    }

    #[tokio::test]
    async fn reorder_with_foreign_scope_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let foreign_container = make_container(&pool, &intruder, "Foreign").await;
        let list = create(&pool, &uid, &checklist_req("Mine")).await.unwrap();

        let err = reorder(
            &pool,
            &uid,
            Some(&foreign_container),
            None,
            std::slice::from_ref(&list.id),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, DomainError::NotFound("container")),
            "got {err:?}"
        );
        assert_eq!(
            stored_placement(&pool, &uid, &list.id).await,
            (None, None, 0)
        );
    }

    // ── delete matrix ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_empty_leaf_list_ok_without_force() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Doomed")).await.unwrap();

        delete(&pool, &list.id, &uid, false).await.unwrap();

        assert!(
            get_one(&pool, &list.id, &uid).await.unwrap().is_none(),
            "list should be gone"
        );
    }

    #[tokio::test]
    async fn delete_list_with_empty_sublists_requires_force() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root = create(&pool, &uid, &checklist_req("Root")).await.unwrap();
        let child = create(&pool, &uid, &sublist_req("Child", &root.id))
            .await
            .unwrap();

        let err = delete(&pool, &root.id, &uid, false).await.unwrap_err();
        assert!(
            matches!(
                err,
                DomainError::HasChildren {
                    kind: "list",
                    lists: 1,
                    containers: 0
                }
            ),
            "got {err:?}"
        );
        assert!(
            get_one(&pool, &root.id, &uid).await.unwrap().is_some(),
            "nothing should be deleted on HasChildren"
        );

        delete(&pool, &root.id, &uid, true).await.unwrap();

        assert!(get_one(&pool, &root.id, &uid).await.unwrap().is_none());
        assert!(get_one(&pool, &child.id, &uid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_list_with_items_is_rejected_even_with_force() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root = create(&pool, &uid, &checklist_req("Root")).await.unwrap();
        let child = create(&pool, &uid, &sublist_req("Child", &root.id))
            .await
            .unwrap();
        make_item(&pool, &child.id, "Deep item").await;

        let err = delete(&pool, &root.id, &uid, true).await.unwrap_err();
        assert!(
            matches!(
                err,
                DomainError::NotEmpty {
                    kind: "list",
                    items: 1
                }
            ),
            "got {err:?}"
        );
        assert!(get_one(&pool, &root.id, &uid).await.unwrap().is_some());
        assert!(get_one(&pool, &child.id, &uid).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn delete_item_in_root_list_blocks_deletion() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root = create(&pool, &uid, &checklist_req("Root")).await.unwrap();
        make_item(&pool, &root.id, "Top item").await;

        let err = delete(&pool, &root.id, &uid, false).await.unwrap_err();
        assert!(
            matches!(
                err,
                DomainError::NotEmpty {
                    kind: "list",
                    items: 1
                }
            ),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn delete_list_of_other_user_is_not_found() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let list = create(&pool, &owner, &checklist_req("Mine")).await.unwrap();

        let err = delete(&pool, &list.id, &intruder, true).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound("list")), "got {err:?}");
        assert!(get_one(&pool, &list.id, &owner).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn delete_preview_reports_subtree_counts() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root = create(&pool, &uid, &checklist_req("Root")).await.unwrap();
        let child = create(&pool, &uid, &sublist_req("Child", &root.id))
            .await
            .unwrap();
        make_item(&pool, &child.id, "Item").await;

        let preview = delete_preview(&pool, &root.id, &uid).await.unwrap();
        assert_eq!(
            preview,
            kartoteka_shared::DeletePreview {
                lists: 1,
                containers: 0,
                items: 1
            }
        );

        let intruder = create_test_user(&pool).await;
        assert!(matches!(
            delete_preview(&pool, &root.id, &intruder)
                .await
                .unwrap_err(),
            DomainError::NotFound("list")
        ));
    }

    #[tokio::test]
    async fn set_appearance_sets_icon_and_color() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Styled")).await.unwrap();
        assert!(list.color.is_none());

        let updated = set_appearance(
            &pool,
            &list.id,
            &uid,
            Some(Some("lucide:star".into())),
            Some(Some("#ff8800".into())),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(updated.icon.as_deref(), Some("lucide:star"));
        assert_eq!(updated.color.as_deref(), Some("#ff8800"));

        // clearing only the colour keeps the icon
        let cleared = set_appearance(&pool, &list.id, &uid, None, Some(None))
            .await
            .unwrap()
            .unwrap();
        assert!(cleared.color.is_none());
        assert_eq!(cleared.icon.as_deref(), Some("lucide:star"));
    }

    #[tokio::test]
    async fn set_appearance_rejects_bad_color() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Bad")).await.unwrap();

        let err = set_appearance(&pool, &list.id, &uid, None, Some(Some("red".into())))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("color_invalid_format")
        ));
        assert!(
            get_one(&pool, &list.id, &uid)
                .await
                .unwrap()
                .unwrap()
                .color
                .is_none()
        );
    }

    #[tokio::test]
    async fn create_rejects_bad_color() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let err = create(
            &pool,
            &uid,
            &CreateListRequest {
                color: Some("#abc".into()),
                ..checklist_req("Bad colour")
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("color_invalid_format")
        ));
    }

    #[tokio::test]
    async fn set_archived_is_idempotent() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Archivable"))
            .await
            .unwrap();

        for _ in 0..2 {
            let archived = set_archived(&pool, &list.id, &uid, true)
                .await
                .unwrap()
                .unwrap();
            assert!(archived.archived);
        }
        for _ in 0..2 {
            let restored = set_archived(&pool, &list.id, &uid, false)
                .await
                .unwrap()
                .unwrap();
            assert!(!restored.archived);
        }

        // the row really holds one state, not a toggled one
        assert!(
            !get_one(&pool, &list.id, &uid)
                .await
                .unwrap()
                .unwrap()
                .archived
        );
    }

    #[tokio::test]
    async fn set_pinned_is_idempotent() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Pinnable"))
            .await
            .unwrap();

        for _ in 0..2 {
            let pinned = set_pinned(&pool, &list.id, &uid, true)
                .await
                .unwrap()
                .unwrap();
            assert!(pinned.pinned);
        }
        let unpinned = set_pinned(&pool, &list.id, &uid, false)
            .await
            .unwrap()
            .unwrap();
        assert!(!unpinned.pinned);
    }

    #[tokio::test]
    async fn set_archived_and_set_pinned_of_other_user_are_none() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Mine")).await.unwrap();

        assert!(
            set_archived(&pool, &list.id, &intruder, true)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            set_pinned(&pool, &list.id, &intruder, true)
                .await
                .unwrap()
                .is_none()
        );
        let row = get_one(&pool, &list.id, &uid).await.unwrap().unwrap();
        assert!(!row.archived);
        assert!(!row.pinned);
    }

    #[tokio::test]
    async fn set_features_keeps_the_config_of_a_feature_that_stays_enabled() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(
            &pool,
            &uid,
            &CreateListRequest {
                features: vec!["deadlines".into()],
                ..checklist_req("Configured")
            },
        )
        .await
        .unwrap();
        update_feature_config(
            &pool,
            &list.id,
            &uid,
            "deadlines",
            serde_json::json!({ "warn_days": 3, "show_time": true }),
        )
        .await
        .unwrap();

        let updated = set_features(
            &pool,
            &list.id,
            &uid,
            &SetFeaturesRequest {
                features: vec!["deadlines".into(), "location".into()],
            },
        )
        .await
        .unwrap()
        .unwrap();

        let deadlines = updated
            .features
            .iter()
            .find(|f| f.feature_name == "deadlines")
            .expect("deadlines stays enabled");
        assert_eq!(
            deadlines.config,
            serde_json::json!({ "warn_days": 3, "show_time": true })
        );
        let location = updated
            .features
            .iter()
            .find(|f| f.feature_name == "location")
            .expect("location is enabled");
        assert_eq!(location.config, serde_json::json!({}));
    }

    #[tokio::test]
    async fn set_features_drops_the_config_of_a_disabled_feature() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(
            &pool,
            &uid,
            &CreateListRequest {
                features: vec!["deadlines".into()],
                ..checklist_req("Configured")
            },
        )
        .await
        .unwrap();
        update_feature_config(
            &pool,
            &list.id,
            &uid,
            "deadlines",
            serde_json::json!({ "warn_days": 3 }),
        )
        .await
        .unwrap();

        let updated = set_features(
            &pool,
            &list.id,
            &uid,
            &SetFeaturesRequest {
                features: vec!["checklist".into()],
            },
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(updated.features.len(), 1);
        assert_eq!(updated.features[0].feature_name, "checklist");

        // enabling it again starts from an empty config
        let reenabled = set_features(
            &pool,
            &list.id,
            &uid,
            &SetFeaturesRequest {
                features: vec!["checklist".into(), "deadlines".into()],
            },
        )
        .await
        .unwrap()
        .unwrap();
        let deadlines = reenabled
            .features
            .iter()
            .find(|f| f.feature_name == "deadlines")
            .unwrap();
        assert_eq!(deadlines.config, serde_json::json!({}));
    }

    #[tokio::test]
    async fn set_features_rejects_unknown_name() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Plain")).await.unwrap();

        let err = set_features(
            &pool,
            &list.id,
            &uid,
            &SetFeaturesRequest {
                features: vec!["checklist".into(), "teleport".into()],
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::Validation("unknown_feature")));

        // nothing was written
        let row = get_one(&pool, &list.id, &uid).await.unwrap().unwrap();
        assert_eq!(row.features.len(), 1);
        assert_eq!(row.features[0].feature_name, "checklist");
    }

    #[test]
    fn features_preserving_configs_keeps_stored_config_and_defaults_new_ones() {
        let existing = serde_json::json!({
            "deadlines": { "warn_days": 5 },
            "quantity": {},
            "time_tracking": { "pomodoro": 25 },
        });
        let existing = existing.as_object().unwrap().clone();

        let features =
            features_preserving_configs(&existing, &["deadlines".into(), "location".into()]);
        let obj = features.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj["deadlines"], serde_json::json!({ "warn_days": 5 }));
        assert_eq!(obj["location"], serde_json::json!({}));
    }

    #[test]
    fn validate_feature_names_accepts_every_known_feature() {
        let names: Vec<String> = KNOWN_FEATURES.iter().map(|n| (*n).to_string()).collect();
        assert!(validate_feature_names(&names).is_ok());
        assert!(validate_feature_names(&[]).is_ok());
        assert!(matches!(
            validate_feature_names(&["nope".to_string()]),
            Err(DomainError::Validation("unknown_feature"))
        ));
    }

    #[tokio::test]
    async fn update_feature_config_rejects_a_non_object_config() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Configured"))
            .await
            .unwrap();

        let err = update_feature_config(
            &pool,
            &list.id,
            &uid,
            "checklist",
            serde_json::json!("nope"),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("invalid_feature_config")
        ));
    }

    #[tokio::test]
    async fn update_feature_config_for_a_disabled_feature_is_not_found() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Plain")).await.unwrap();

        let err = update_feature_config(&pool, &list.id, &uid, "deadlines", serde_json::json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound("feature")));
    }
}
