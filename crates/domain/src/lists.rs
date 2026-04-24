use crate::{DomainError, rules};
use kartoteka_db as db;
use kartoteka_shared::types::ListFeature;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

// ── Public domain types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    Checklist,
    Shopping,
    Habits,
    Log,
}

impl ListType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ListType::Checklist => "checklist",
            ListType::Shopping => "shopping",
            ListType::Habits => "habits",
            ListType::Log => "log",
        }
    }
}

impl TryFrom<&str> for ListType {
    type Error = DomainError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "checklist" => Ok(ListType::Checklist),
            "shopping" => Ok(ListType::Shopping),
            "habits" => Ok(ListType::Habits),
            "log" => Ok(ListType::Log),
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
    pub description: Option<String>,
    pub list_type: String,
    pub parent_list_id: Option<String>,
    pub position: i64,
    pub archived: bool,
    pub container_id: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
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
    pub description: Option<String>,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
    pub features: Vec<String>, // feature_name strings
}

#[derive(Debug, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub icon: Option<Option<String>>, // Some(None) clears the field
    pub description: Option<Option<String>>,
    pub list_type: Option<String>,
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

fn parse_features_map(json: &str) -> Result<serde_json::Map<String, serde_json::Value>, DomainError> {
    serde_json::from_str(json).map_err(|e| DomainError::Internal(e.to_string()))
}

pub(crate) fn parse_features(json: &str) -> Result<Vec<ListFeature>, DomainError> {
    Ok(parse_features_map(json)?
        .into_iter()
        .map(|(feature_name, config)| ListFeature { feature_name, config })
        .collect())
}

/// Build a features JSON object from a list of names (each gets an empty config).
fn features_from_names(names: &[String]) -> serde_json::Value {
    let obj: serde_json::Map<String, serde_json::Value> = names
        .iter()
        .map(|n| (n.clone(), serde_json::json!({})))
        .collect();
    serde_json::Value::Object(obj)
}

pub(crate) fn row_to_list(row: db::lists::ListRow) -> Result<List, DomainError> {
    let features = parse_features(&row.features)?;
    Ok(List {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        icon: row.icon,
        description: row.description,
        list_type: row.list_type,
        parent_list_id: row.parent_list_id,
        position: row.position,
        archived: row.archived != 0,
        container_id: row.container_id,
        pinned: row.pinned != 0,
        last_opened_at: row.last_opened_at,
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

#[tracing::instrument(skip(pool))]
pub async fn sublists(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<List>, DomainError> {
    let rows = db::lists::sublists(pool, parent_id, user_id).await?;
    rows.into_iter().map(row_to_list).collect()
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateListRequest,
) -> Result<List, DomainError> {
    let list_type = req.list_type.as_deref().unwrap_or("checklist");

    // Phase 2: THINK
    rules::lists::validate_list_type_features(list_type, &req.features)?;

    // Phase 1: READ (position — before transaction, doesn't hold write lock)
    let position = db::lists::next_position(
        pool,
        user_id,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    )
    .await?;

    // Phase 3: WRITE
    let list_id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    db::lists::insert(
        &mut tx,
        &list_id,
        user_id,
        position,
        &req.name,
        req.icon.as_deref(),
        req.description.as_deref(),
        list_type,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    )
    .await?;
    if !req.features.is_empty() {
        db::lists::set_features(&mut tx, &list_id, &features_from_names(&req.features)).await?;
    }
    tx.commit().await.map_err(db::DbError::Sqlx)?;

    db::lists::get_one(pool, &list_id, user_id)
        .await?
        .map(row_to_list)
        .transpose()?
        .ok_or_else(|| DomainError::Internal("list disappeared after create".into()))
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateListRequest,
) -> Result<Option<List>, DomainError> {
    // Phase 1: READ — need current list to validate type change
    let current = match db::lists::get_one(pool, id, user_id).await? {
        Some(l) => l,
        None => return Ok(None),
    };

    // Phase 2: THINK — validate new list_type against existing features
    if let Some(new_type) = req.list_type.as_deref() {
        let features = parse_features(&current.features)?;
        let feature_names: Vec<String> = features.iter().map(|f| f.feature_name.clone()).collect();
        rules::lists::validate_list_type_features(new_type, &feature_names)?;
    }

    // Phase 3: WRITE
    let updated = db::lists::update(
        pool,
        id,
        user_id,
        req.name.as_deref(),
        req.icon.as_ref().map(|v| v.as_deref()),
        req.description.as_ref().map(|v| v.as_deref()),
        req.list_type.as_deref(),
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

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DomainError> {
    Ok(db::lists::delete(pool, id, user_id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_archive(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<List>, DomainError> {
    let toggled = db::lists::toggle_archived(pool, id, user_id).await?;
    if !toggled {
        return Ok(None);
    }
    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<List>, DomainError> {
    let toggled = db::lists::toggle_pinned(pool, id, user_id).await?;
    if !toggled {
        return Ok(None);
    }
    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
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

/// Replace all features for a list (validates against current list_type).
#[tracing::instrument(skip(pool))]
pub async fn set_features(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &SetFeaturesRequest,
) -> Result<Option<List>, DomainError> {
    // Phase 1: READ
    let current = match db::lists::get_one(pool, id, user_id).await? {
        Some(l) => l,
        None => return Ok(None),
    };

    // Phase 2: THINK
    rules::lists::validate_list_type_features(&current.list_type, &req.features)?;

    // Phase 3: WRITE
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    db::lists::set_features(&mut tx, id, &features_from_names(&req.features)).await?;
    tx.commit().await.map_err(db::DbError::Sqlx)?;

    db::lists::get_one(pool, id, user_id)
        .await?
        .map(row_to_list)
        .transpose()
}

/// Update the config of a single feature without affecting other features.
/// Returns `DomainError::NotFound("feature")` if the feature is not enabled on the list.
#[tracing::instrument(skip(pool))]
pub async fn update_feature_config(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    feature_name: &str,
    config: serde_json::Value,
) -> Result<(), DomainError> {
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
        }
    }

    #[tokio::test]
    async fn create_checklist_no_features() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Todo")).await.unwrap();

        assert_eq!(list.name, "Todo");
        assert_eq!(list.list_type, "checklist");
        assert!(list.features.is_empty());
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
    async fn create_shopping_requires_quantity() {
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
        };
        let err = create(&pool, &uid, &req).await.unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("shopping_lists_require_quantity")
        ));
    }

    #[tokio::test]
    async fn create_shopping_with_quantity_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Groceries".into(),
            list_type: Some("shopping".into()),
            features: vec!["quantity".into()],
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(list.list_type, "shopping");
        assert_eq!(list.features.len(), 1);
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
    async fn set_features_validates_against_list_type() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Shop".into(),
            list_type: Some("shopping".into()),
            features: vec!["quantity".into()],
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();

        // Removing quantity from a shopping list → invalid
        let err = set_features(
            &pool,
            &list.id,
            &uid,
            &SetFeaturesRequest {
                features: vec!["deadlines".into()], // no quantity!
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("shopping_lists_require_quantity")
        ));
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
}
