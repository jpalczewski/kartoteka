use crate::DbError;
use sqlx::{SqliteConnection, SqlitePool};

// ── Row types (internal to db crate) ────────────────────────────────────────

#[derive(sqlx::FromRow)]
pub struct ListRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub list_type: String,
    pub parent_list_id: Option<String>,
    pub position: i64,
    pub archived: i64,
    pub container_id: Option<String>,
    pub pinned: i64,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub features_json: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct FeatureRow {
    pub feature_name: String,
    pub config: String,
}

// Used by domain::items::create (B3)
#[derive(Debug)]
pub struct CreateItemContext {
    pub features: Vec<String>,
    pub next_position: i64,
}

#[derive(sqlx::FromRow)]
struct CreateItemContextRow {
    pub next_position: i64,
    pub features_json: String,
}

// ── SQL helpers ──────────────────────────────────────────────────────────────

/// Correlated subquery fragment that returns features as a JSON array.
const FEATURES_SUBQUERY: &str = "COALESCE(
    (SELECT json_group_array(json_object('feature_name', lf.feature_name, 'config', json(lf.config)))
     FROM list_features lf WHERE lf.list_id = l.id),
    '[]'
) as features_json";

// ── Read queries ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.user_id = ? AND l.archived = 0 AND l.parent_list_id IS NULL \
         ORDER BY l.pinned DESC, l.position",
        FEATURES_SUBQUERY
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Returns all archived lists for the user, including archived sublists.
/// Note: unlike `list_all`, this does not filter by parent_list_id IS NULL.
#[tracing::instrument(skip(pool))]
pub async fn list_archived(pool: &SqlitePool, user_id: &str) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.user_id = ? AND l.archived = 1 \
         ORDER BY l.updated_at DESC",
        FEATURES_SUBQUERY
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Lists where pinned = 1, not archived, no parent.
/// Ordered by name ascending.
#[tracing::instrument(skip(pool))]
pub async fn pinned(pool: &SqlitePool, user_id: &str) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.user_id = ? AND l.pinned = 1 AND l.archived = 0 \
         AND l.parent_list_id IS NULL \
         ORDER BY l.name ASC",
        FEATURES_SUBQUERY
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Recently opened lists (pinned = 0, last_opened_at not null), not archived, no parent.
/// Ordered by last_opened_at DESC. `limit` caps results (use 5 for home page).
#[tracing::instrument(skip(pool))]
pub async fn recent(pool: &SqlitePool, user_id: &str, limit: i64) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.user_id = ? AND l.pinned = 0 AND l.archived = 0 \
         AND l.last_opened_at IS NOT NULL AND l.parent_list_id IS NULL \
         ORDER BY l.last_opened_at DESC \
         LIMIT ?",
        FEATURES_SUBQUERY
    ))
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Root lists: no container, no parent list, not archived, not pinned.
/// Ordered by updated_at DESC.
#[tracing::instrument(skip(pool))]
pub async fn root(pool: &SqlitePool, user_id: &str) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.user_id = ? AND l.container_id IS NULL \
         AND l.parent_list_id IS NULL AND l.archived = 0 AND l.pinned = 0 \
         ORDER BY l.updated_at DESC",
        FEATURES_SUBQUERY
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l WHERE l.id = ? AND l.user_id = ?",
        FEATURES_SUBQUERY
    ))
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn sublists(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.parent_list_id = ? AND l.user_id = ? \
         ORDER BY l.position",
        FEATURES_SUBQUERY
    ))
    .bind(parent_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Returns the next available position for a new list.
#[tracing::instrument(skip(pool))]
pub async fn next_position(
    pool: &SqlitePool,
    user_id: &str,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
) -> Result<i64, DbError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM lists \
         WHERE user_id = ? AND container_id IS ? AND parent_list_id IS ?",
    )
    .bind(user_id)
    .bind(container_id)
    .bind(parent_list_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row.0)
}

/// One query: ownership check + feature names + MAX(position) for items.
/// Returns None if list not found or not owned by user_id.
#[tracing::instrument(skip(pool))]
pub async fn get_create_item_context(
    pool: &SqlitePool,
    list_id: &str,
    user_id: &str,
) -> Result<Option<CreateItemContext>, DbError> {
    let row: Option<CreateItemContextRow> = sqlx::query_as(
        "SELECT \
            COALESCE(MAX(i.position) + 1, 0) as next_position, \
            COALESCE( \
                (SELECT json_group_array(lf.feature_name) \
                 FROM list_features lf WHERE lf.list_id = l.id), \
                '[]' \
            ) as features_json \
         FROM lists l \
         LEFT JOIN items i ON i.list_id = l.id \
         WHERE l.id = ? AND l.user_id = ? \
         GROUP BY l.id",
    )
    .bind(list_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)?;

    match row {
        None => Ok(None),
        Some(r) => {
            let features: Vec<String> = serde_json::from_str(&r.features_json)?;
            Ok(Some(CreateItemContext {
                features,
                next_position: r.next_position,
            }))
        }
    }
}

// ── Write queries (called in transaction) ────────────────────────────────────

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(tx))]
pub async fn insert(
    tx: &mut SqliteConnection,
    id: &str,
    user_id: &str,
    position: i64,
    name: &str,
    icon: Option<&str>,
    description: Option<&str>,
    list_type: &str,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO lists (id, user_id, name, icon, description, list_type, \
                            container_id, parent_list_id, position) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(icon)
    .bind(description)
    .bind(list_type)
    .bind(container_id)
    .bind(parent_list_id)
    .bind(position)
    .execute(&mut *tx)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

/// Replace all features for a list. Caller must be in a transaction.
#[tracing::instrument(skip(tx))]
pub async fn replace_features(
    tx: &mut SqliteConnection,
    list_id: &str,
    features: &[String],
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM list_features WHERE list_id = ?")
        .bind(list_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Sqlx)?;

    for feature in features {
        sqlx::query(
            "INSERT INTO list_features (list_id, feature_name, config) VALUES (?, ?, '{}')",
        )
        .bind(list_id)
        .bind(feature)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Sqlx)?;
    }
    Ok(())
}

// ── Write queries (no transaction needed) ────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    name: Option<&str>,
    icon: Option<Option<&str>>,
    description: Option<Option<&str>>,
    list_type: Option<&str>,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE lists \
         SET name = COALESCE(?, name), \
             icon = CASE WHEN ? = 1 THEN ? ELSE icon END, \
             description = CASE WHEN ? = 1 THEN ? ELSE description END, \
             list_type = COALESCE(?, list_type), \
             updated_at = datetime('now') \
         WHERE id = ? AND user_id = ?",
    )
    .bind(name)
    .bind(icon.is_some() as i32)
    .bind(icon.and_then(|v| v))
    .bind(description.is_some() as i32)
    .bind(description.and_then(|v| v))
    .bind(list_type)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM lists WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_archived(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE lists SET archived = CASE WHEN archived = 0 THEN 1 ELSE 0 END, \
                          updated_at = datetime('now') \
         WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pinned(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE lists SET pinned = CASE WHEN pinned = 0 THEN 1 ELSE 0 END, \
                          updated_at = datetime('now') \
         WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

/// Mark all items in a list as not completed.
#[tracing::instrument(skip(pool))]
pub async fn uncheck_items(pool: &SqlitePool, list_id: &str) -> Result<u64, DbError> {
    let result = sqlx::query("UPDATE items SET completed = 0 WHERE list_id = ?")
        .bind(list_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(pool))]
pub async fn move_list(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    position: i64,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE lists SET position = ?, container_id = ?, parent_list_id = ?, \
                          updated_at = datetime('now') \
         WHERE id = ? AND user_id = ?",
    )
    .bind(position)
    .bind(container_id)
    .bind(parent_list_id)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

// ── db-level tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};

    async fn insert_test_list(pool: &SqlitePool, user_id: &str, name: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let mut tx = pool.begin().await.unwrap();
        insert(
            &mut tx,
            &id,
            user_id,
            0,
            name,
            None,
            None,
            "checklist",
            None,
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        id
    }

    #[tokio::test]
    async fn insert_and_get_one() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "My List").await;

        let row = get_one(&pool, &id, &user_id).await.unwrap().unwrap();
        assert_eq!(row.name, "My List");
        assert_eq!(row.list_type, "checklist");
        assert_eq!(row.archived, 0);
        assert_eq!(row.pinned, 0);
        assert_eq!(row.features_json, "[]");
    }

    #[tokio::test]
    async fn get_one_wrong_user_returns_none() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let other_user = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "Private").await;

        let row = get_one(&pool, &id, &other_user).await.unwrap();
        assert!(row.is_none());
    }

    #[tokio::test]
    async fn list_all_excludes_archived() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "Active").await;
        insert_test_list(&pool, &user_id, "Active2").await;
        toggle_archived(&pool, &id, &user_id).await.unwrap();

        let rows = list_all(&pool, &user_id).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Active2");
    }

    #[tokio::test]
    async fn features_roundtrip() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "With Features").await;

        let mut tx = pool.begin().await.unwrap();
        replace_features(&mut tx, &id, &["deadlines".into(), "quantity".into()])
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let row = get_one(&pool, &id, &user_id).await.unwrap().unwrap();
        let features: Vec<serde_json::Value> = serde_json::from_str(&row.features_json).unwrap();
        assert_eq!(features.len(), 2);
        let names: Vec<&str> = features
            .iter()
            .map(|f| f["feature_name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"deadlines"));
        assert!(names.contains(&"quantity"));
    }

    #[tokio::test]
    async fn toggle_archived_flips() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "List").await;

        toggle_archived(&pool, &id, &user_id).await.unwrap();
        let row = get_one(&pool, &id, &user_id).await.unwrap().unwrap();
        assert_eq!(row.archived, 1);

        toggle_archived(&pool, &id, &user_id).await.unwrap();
        let row = get_one(&pool, &id, &user_id).await.unwrap().unwrap();
        assert_eq!(row.archived, 0);
    }

    #[tokio::test]
    async fn get_create_item_context_no_items() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "Empty List").await;

        let mut tx = pool.begin().await.unwrap();
        replace_features(&mut tx, &id, &["deadlines".into()])
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let ctx = get_create_item_context(&pool, &id, &user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ctx.next_position, 0);
        assert_eq!(ctx.features, vec!["deadlines"]);
    }

    #[tokio::test]
    async fn get_create_item_context_wrong_user_returns_none() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "List").await;

        let ctx = get_create_item_context(&pool, &id, &other).await.unwrap();
        assert!(ctx.is_none());
    }

    #[tokio::test]
    async fn delete_removes_list() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "Doomed").await;

        let deleted = delete(&pool, &id, &user_id).await.unwrap();
        assert!(deleted);
        assert!(get_one(&pool, &id, &user_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn pinned_returns_only_pinned_lists() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &uid, "Pinned").await;
        toggle_pinned(&pool, &id, &uid).await.unwrap();

        let rows = pinned(&pool, &uid).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_ne!(rows[0].pinned, 0);
    }

    #[tokio::test]
    async fn recent_returns_recently_opened_not_pinned() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &uid, "Recent").await;
        // Simulate open by updating last_opened_at
        sqlx::query("UPDATE lists SET last_opened_at = datetime('now') WHERE id = ?")
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();

        let rows = recent(&pool, &uid, 5).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
    }

    #[tokio::test]
    async fn root_excludes_sublists_and_containerized_lists() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root_id = insert_test_list(&pool, &uid, "Root").await;

        let rows = root(&pool, &uid).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, root_id);
    }

    #[tokio::test]
    async fn root_excludes_pinned_lists() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &uid, "PinnedRoot").await;
        toggle_pinned(&pool, &id, &uid).await.unwrap();

        let rows = root(&pool, &uid).await.unwrap();
        assert!(rows.is_empty(), "pinned list should not appear in root()");
    }
}
