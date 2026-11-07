use crate::types::{ContainerRow, DeleteTreeOutcome, SubtreeSummary};
use crate::{DbError, SqlitePool};
use kartoteka_shared::types::{CreateContainerRequest, UpdateContainerRequest};
use sqlx::{Sqlite, SqliteConnection};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct ContainerProgressRow {
    pub total_lists: i64,
    pub total_items: i64,
    pub completed_items: i64,
}

/// Returns the next available position for a container under the given parent (or root).
#[tracing::instrument(skip(pool))]
pub async fn next_position(
    pool: &SqlitePool,
    user_id: &str,
    parent_id: Option<&str>,
) -> Result<i32, DbError> {
    let row: (i32,) = sqlx::query_as(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM containers WHERE user_id = ? AND parent_container_id IS ?",
    )
    .bind(user_id)
    .bind(parent_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// List all containers for a user, ordered by position.
#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    let rows = sqlx::query_as(
        "SELECT * FROM containers WHERE user_id = ? AND archived = 0 ORDER BY position ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get a single container by id and user_id.
#[tracing::instrument(skip(executor))]
pub async fn get_one<'e, E>(
    executor: E,
    id: &str,
    user_id: &str,
) -> Result<Option<ContainerRow>, DbError>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let row = sqlx::query_as("SELECT * FROM containers WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(executor)
        .await?;
    Ok(row)
}

/// Insert a new container and return the created row.
#[tracing::instrument(skip(pool))]
pub async fn insert(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateContainerRequest,
    position: i32,
) -> Result<ContainerRow, DbError> {
    let id = Uuid::new_v4().to_string();
    let mut conn = pool.acquire().await.map_err(DbError::Sqlx)?;
    insert_in_tx(&mut conn, &id, user_id, req, position).await
}

pub async fn find_owned_ids(
    pool: &SqlitePool,
    user_id: &str,
    ids: &[&str],
) -> Result<HashSet<String>, DbError> {
    crate::find_owned_ids_in(pool, "containers", user_id, ids).await
}

/// Insert a new container within a caller-owned transaction.
/// The caller is responsible for generating `id` and for commit/rollback.
#[tracing::instrument(skip(conn, req), fields(id = %id))]
pub async fn insert_in_tx(
    conn: &mut SqliteConnection,
    id: &str,
    user_id: &str,
    req: &CreateContainerRequest,
    position: i32,
) -> Result<ContainerRow, DbError> {
    let row = sqlx::query_as(
        r#"INSERT INTO containers (id, user_id, name, icon, color, description, status, parent_container_id, position)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
           RETURNING *"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(&req.name)
    .bind(&req.icon)
    .bind(&req.color)
    .bind(&req.description)
    .bind(&req.status)
    .bind(&req.parent_container_id)
    .bind(position)
    .fetch_one(&mut *conn)
    .await?;
    Ok(row)
}

/// Update a container using read-modify-write. Returns None if not found.
#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateContainerRequest,
) -> Result<Option<ContainerRow>, DbError> {
    let existing = match get_one(pool, id, user_id).await? {
        Some(row) => row,
        None => return Ok(None),
    };

    let name = req.name.as_deref().unwrap_or(&existing.name);
    let icon = req
        .icon
        .as_ref()
        .map_or(existing.icon.as_deref(), |v| v.as_deref());
    let color = req
        .color
        .as_ref()
        .map_or(existing.color.as_deref(), |v| v.as_deref());
    let description = req
        .description
        .as_ref()
        .map_or(existing.description.as_deref(), |v| v.as_deref());
    let status = req
        .status
        .as_ref()
        .map_or(existing.status.as_deref(), |v| v.as_deref());
    let location_id = req
        .location_id
        .as_ref()
        .map_or(existing.location_id.as_deref(), |v| v.as_deref());

    let row = sqlx::query_as(
        r#"UPDATE containers
           SET name=?, icon=?, color=?, description=?, status=?, location_id=?,
               updated_at=datetime('now')
           WHERE id=? AND user_id=?
           RETURNING *"#,
    )
    .bind(name)
    .bind(icon)
    .bind(color)
    .bind(description)
    .bind(status)
    .bind(location_id)
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ── Subtree inspection and tree deletion ────────────────────────────────────

/// Count lists, containers and items in the subtree rooted at `id`: the container
/// itself, all subcontainers, every list assigned to any of those containers and all
/// their sublists (recursively; archived rows included).
///
/// Returns `Ok(None)` when the container does not exist or belongs to another user.
#[tracing::instrument(skip(pool))]
pub async fn subtree_summary(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<SubtreeSummary>, DbError> {
    let mut conn = pool.acquire().await?;
    summary_in(&mut conn, id, user_id).await
}

async fn summary_in(
    conn: &mut SqliteConnection,
    id: &str,
    user_id: &str,
) -> Result<Option<SubtreeSummary>, DbError> {
    let exists: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM containers WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&mut *conn)
            .await?;
    if exists.is_none() {
        return Ok(None);
    }

    let summary = sqlx::query_as(
        "WITH RECURSIVE ctree(id) AS (
             SELECT id FROM containers WHERE id = ? AND user_id = ?
             UNION
             SELECT c.id FROM containers c
             JOIN ctree t ON c.parent_container_id = t.id
             WHERE c.user_id = ?
         ),
         ltree(id) AS (
             SELECT l.id FROM lists l
             WHERE l.container_id IN (SELECT id FROM ctree) AND l.user_id = ?
             UNION
             SELECT l.id FROM lists l
             JOIN ltree t ON l.parent_list_id = t.id
             WHERE l.user_id = ?
         )
         SELECT (SELECT COUNT(*) FROM ctree) - 1 AS containers,
                (SELECT COUNT(*) FROM ltree) AS lists,
                (SELECT COUNT(*) FROM items WHERE list_id IN (SELECT id FROM ltree)) AS items",
    )
    .bind(id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(Some(summary))
}

/// Delete a container and its whole subtree in a single transaction.
///
/// The summary is recomputed inside the transaction, so an item added after a
/// preview still blocks the deletion. Nothing is deleted when the tree holds items.
#[tracing::instrument(skip(pool))]
pub async fn delete_tree(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<DeleteTreeOutcome, DbError> {
    let mut tx = pool.begin().await?;

    let Some(summary) = summary_in(&mut tx, id, user_id).await? else {
        return Ok(DeleteTreeOutcome::NotFound);
    };
    if summary.items > 0 {
        return Ok(DeleteTreeOutcome::NotEmpty {
            items: summary.items,
        });
    }

    // Lists first: a list references its container. One statement per table is enough
    // because SQLite checks immediate foreign keys at the end of a statement, so the
    // missing ON DELETE actions on `lists.container_id` / `lists.parent_list_id` /
    // `containers.parent_container_id` never trip inside a subtree delete.
    sqlx::query(
        "WITH RECURSIVE ctree(id) AS (
             SELECT id FROM containers WHERE id = ? AND user_id = ?
             UNION
             SELECT c.id FROM containers c
             JOIN ctree t ON c.parent_container_id = t.id
             WHERE c.user_id = ?
         ),
         ltree(id) AS (
             SELECT l.id FROM lists l
             WHERE l.container_id IN (SELECT id FROM ctree) AND l.user_id = ?
             UNION
             SELECT l.id FROM lists l
             JOIN ltree t ON l.parent_list_id = t.id
             WHERE l.user_id = ?
         )
         DELETE FROM lists WHERE id IN (SELECT id FROM ltree) AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "WITH RECURSIVE ctree(id) AS (
             SELECT id FROM containers WHERE id = ? AND user_id = ?
             UNION
             SELECT c.id FROM containers c
             JOIN ctree t ON c.parent_container_id = t.id
             WHERE c.user_id = ?
         )
         DELETE FROM containers WHERE id IN (SELECT id FROM ctree) AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(DeleteTreeOutcome::Deleted)
}

/// Toggle archived state for a container and all its descendants + their lists.
/// Returns true if the root container was found and updated.
#[tracing::instrument(skip(pool))]
pub async fn toggle_archived(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let current = get_one(pool, id, user_id).await?;
    let Some(row) = current else {
        return Ok(false);
    };
    let new_val: i32 = if row.archived { 0 } else { 1 };

    let mut tx = pool.begin().await.map_err(DbError::Sqlx)?;

    sqlx::query(
        "WITH RECURSIVE subtree(id) AS (
            SELECT id FROM containers WHERE id = ? AND user_id = ?
            UNION ALL
            SELECT c.id FROM containers c JOIN subtree s ON c.parent_container_id = s.id
        )
        UPDATE containers SET archived = ?, updated_at = datetime('now')
        WHERE id IN (SELECT id FROM subtree) AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .bind(new_val)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(DbError::Sqlx)?;

    sqlx::query(
        "WITH RECURSIVE subtree(id) AS (
            SELECT id FROM containers WHERE id = ? AND user_id = ?
            UNION ALL
            SELECT c.id FROM containers c JOIN subtree s ON c.parent_container_id = s.id
        )
        UPDATE lists SET archived = ?, updated_at = datetime('now')
        WHERE container_id IN (SELECT id FROM subtree) AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .bind(new_val)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(DbError::Sqlx)?;

    tx.commit().await.map_err(DbError::Sqlx)?;
    Ok(true)
}

/// List archived containers for a user — root entries only
/// (containers whose parent is not also archived).
#[tracing::instrument(skip(pool))]
pub async fn list_archived(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    let rows = sqlx::query_as(
        "SELECT * FROM containers
         WHERE user_id = ? AND archived = 1
           AND (parent_container_id IS NULL
                OR parent_container_id NOT IN (
                    SELECT id FROM containers WHERE archived = 1 AND user_id = ?
                ))
         ORDER BY position ASC",
    )
    .bind(user_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List direct children of a container.
#[tracing::instrument(skip(pool))]
pub async fn children(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<ContainerRow>, DbError> {
    let rows = sqlx::query_as(
        "SELECT * FROM containers WHERE parent_container_id=? AND user_id=? AND archived = 0 ORDER BY position ASC",
    )
    .bind(parent_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List root-level containers (no parent) for a user.
#[tracing::instrument(skip(pool))]
pub async fn root(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    let rows = sqlx::query_as(
        "SELECT * FROM containers WHERE user_id=? AND parent_container_id IS NULL AND archived = 0 ORDER BY position ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List pinned containers for a user.
#[tracing::instrument(skip(pool))]
pub async fn pinned(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    let rows = sqlx::query_as(
        "SELECT * FROM containers WHERE user_id=? AND pinned=1 AND archived = 0 ORDER BY position ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List recently opened/updated containers for a user (top 10).
#[tracing::instrument(skip(pool))]
pub async fn recent(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    let rows = sqlx::query_as(
        "SELECT * FROM containers WHERE user_id=? AND archived = 0 ORDER BY COALESCE(last_opened_at, updated_at) DESC LIMIT 10",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Toggle the pinned state of a container. Returns None if not found.
#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<ContainerRow>, DbError> {
    let row = sqlx::query_as(
        r#"UPDATE containers
           SET pinned=CASE WHEN pinned=1 THEN 0 ELSE 1 END, updated_at=datetime('now')
           WHERE id=? AND user_id=?
           RETURNING *"#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Move a container to a new parent and position.
async fn move_container_row<'e, E>(
    executor: E,
    id: &str,
    user_id: &str,
    parent_id: Option<&str>,
    position: i32,
) -> Result<Option<ContainerRow>, DbError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let row = sqlx::query_as(
        r#"UPDATE containers
           SET parent_container_id=?, position=?, updated_at=datetime('now')
           WHERE id=? AND user_id=?
           RETURNING *"#,
    )
    .bind(parent_id)
    .bind(position)
    .bind(id)
    .bind(user_id)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

/// Move a container to a new parent and position.
#[tracing::instrument(skip(pool))]
pub async fn move_container(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    parent_id: Option<&str>,
    position: i32,
) -> Result<Option<ContainerRow>, DbError> {
    move_container_row(pool, id, user_id, parent_id, position).await
}

/// Same as [`move_container`] but runs on a caller-owned transaction connection.
#[tracing::instrument(skip(conn))]
pub async fn move_container_in_tx(
    conn: &mut SqliteConnection,
    id: &str,
    user_id: &str,
    parent_id: Option<&str>,
    position: i32,
) -> Result<Option<ContainerRow>, DbError> {
    move_container_row(&mut *conn, id, user_id, parent_id, position).await
}

/// Ids of all containers owned by `user_id` that are direct children of
/// `parent_id` (root when `None`), ordered by position. Used to validate a
/// reorder request against the real scope before writing.
pub async fn child_ids(
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
    user_id: &str,
    parent_id: Option<&str>,
) -> Result<Vec<String>, DbError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM containers \
         WHERE user_id = ? AND parent_container_id IS ? \
         ORDER BY position, id",
    )
    .bind(user_id)
    .bind(parent_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Check if `possible_descendant_id` is a descendant (or equal to) `ancestor_id`.
#[tracing::instrument(skip(pool))]
pub async fn is_descendant(
    pool: &SqlitePool,
    user_id: &str,
    ancestor_id: &str,
    possible_descendant_id: &str,
) -> Result<bool, DbError> {
    let row: (i64,) = sqlx::query_as(
        r#"WITH RECURSIVE subtree AS (
               SELECT id FROM containers WHERE id = ? AND user_id = ?
               UNION ALL
               SELECT c.id FROM containers c
               JOIN subtree s ON c.parent_container_id = s.id
               WHERE c.user_id = ?
           )
           SELECT COUNT(*) FROM subtree WHERE id = ?"#,
    )
    .bind(ancestor_id)
    .bind(user_id)
    .bind(user_id)
    .bind(possible_descendant_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0 > 0)
}

/// Get progress statistics for a container (lists and items).
///
/// **Recursive**: the counts cover the container itself, every sub-container at
/// any depth, every list in that subtree and those lists' sublists at any depth.
/// `total_items` counts all items in those lists; `completed_items` counts the
/// ones with `completed = 1`.
#[tracing::instrument(skip(pool))]
pub async fn progress(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<ContainerProgressRow, DbError> {
    let row = sqlx::query_as(
        r#"WITH RECURSIVE sub(id) AS (
               SELECT id FROM containers WHERE id = ? AND user_id = ?
               UNION
               SELECT c.id FROM containers c
               JOIN sub s ON c.parent_container_id = s.id
               WHERE c.user_id = ?
           ),
           sub_lists(id) AS (
               SELECT l.id FROM lists l WHERE l.user_id = ? AND l.container_id IN (SELECT id FROM sub)
               UNION
               SELECT l.id FROM lists l JOIN sub_lists sl ON l.parent_list_id = sl.id
               WHERE l.user_id = ?
           )
           SELECT (SELECT COUNT(*) FROM sub_lists) as total_lists,
                  COUNT(i.id) as total_items,
                  COALESCE(SUM(CASE WHEN i.completed=1 THEN 1 ELSE 0 END), 0) as completed_items
           FROM sub_lists s
           LEFT JOIN items i ON i.list_id = s.id"#,
    )
    .bind(id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Update last_opened_at for a container.
///
/// Only `last_opened_at` is written: opening a container is a read and must not
/// make it look freshly edited (`updated_at`).
#[tracing::instrument(skip(pool))]
pub async fn touch_last_opened(pool: &SqlitePool, id: &str, user_id: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE containers SET last_opened_at=datetime('now') WHERE id=? AND user_id=?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
async fn create_test_list(
    pool: &SqlitePool,
    user_id: &str,
    name: &str,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
) -> String {
    use crate::lists as db_lists;
    let id = uuid::Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.unwrap();
    db_lists::insert(
        &mut tx,
        &db_lists::InsertListInput {
            id: id.clone(),
            user_id: user_id.to_owned(),
            name: name.to_owned(),
            list_type: "checklist".to_owned(),
            container_id: container_id.map(str::to_owned),
            parent_list_id: parent_list_id.map(str::to_owned),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

#[cfg(test)]
async fn create_test_container(
    pool: &SqlitePool,
    user_id: &str,
    name: &str,
    parent_id: Option<&str>,
) -> ContainerRow {
    use kartoteka_shared::types::CreateContainerRequest;
    let req = CreateContainerRequest {
        name: name.to_string(),
        icon: None,
        description: None,
        status: None,
        parent_container_id: parent_id.map(str::to_string),
        color: None,
    };
    let pos = next_position(pool, user_id, parent_id).await.unwrap();
    insert(pool, user_id, &req, pos).await.unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
    use kartoteka_shared::types::CreateContainerRequest;

    fn make_req(name: &str) -> CreateContainerRequest {
        CreateContainerRequest {
            name: name.to_string(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
            color: None,
        }
    }

    #[tokio::test]
    async fn insert_and_get_one() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let req = make_req("My Container");
        let created = insert(&pool, &uid, &req, 0).await.unwrap();

        assert_eq!(created.name, "My Container");
        assert_eq!(created.user_id, uid);
        assert_eq!(created.position, 0);
        assert!(!created.pinned);

        let fetched = get_one(&pool, &created.id, &uid).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn get_one_wrong_user_returns_none() {
        let pool = test_pool().await;
        let uid_a = create_test_user(&pool).await;
        let uid_b = create_test_user(&pool).await;

        let req = make_req("A's Container");
        let created = insert(&pool, &uid_a, &req, 0).await.unwrap();

        let result = get_one(&pool, &created.id, &uid_b).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_all_returns_only_own() {
        let pool = test_pool().await;
        let uid_a = create_test_user(&pool).await;
        let uid_b = create_test_user(&pool).await;

        insert(&pool, &uid_a, &make_req("A's"), 0).await.unwrap();
        insert(&pool, &uid_b, &make_req("B's"), 0).await.unwrap();

        let list_a = list_all(&pool, &uid_a).await.unwrap();
        assert_eq!(list_a.len(), 1);
        assert_eq!(list_a[0].user_id, uid_a);
    }

    #[tokio::test]
    async fn update_patches_fields() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let created = insert(&pool, &uid, &make_req("Original"), 0).await.unwrap();

        let req = UpdateContainerRequest {
            name: Some("Updated".to_string()),
            icon: Some(Some("🗂️".to_string())),
            description: Some(Some("A description".to_string())),
            status: None,
            location_id: None,
            color: None,
        };

        let updated = update(&pool, &created.id, &uid, &req)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.icon.as_deref(), Some("🗂️"));
        assert_eq!(updated.description.as_deref(), Some("A description"));
    }

    #[tokio::test]
    async fn insert_and_update_color() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let mut req = make_req("Coloured");
        req.color = Some("#ff8800".to_string());
        let created = insert(&pool, &uid, &req, 0).await.unwrap();
        assert_eq!(created.color.as_deref(), Some("#ff8800"));

        let update_req = UpdateContainerRequest {
            name: None,
            icon: None,
            color: Some(Some("#0088ff".to_string())),
            description: None,
            status: None,
            location_id: None,
        };
        let updated = update(&pool, &created.id, &uid, &update_req)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.color.as_deref(), Some("#0088ff"));

        let cleared = UpdateContainerRequest {
            name: None,
            icon: None,
            color: Some(None),
            description: None,
            status: None,
            location_id: None,
        };
        let updated = update(&pool, &created.id, &uid, &cleared)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.color.is_none());
    }

    #[tokio::test]
    async fn update_clears_nullable_field() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let mut req = make_req("With Icon");
        req.icon = Some("📁".to_string());
        let created = insert(&pool, &uid, &req, 0).await.unwrap();
        assert!(created.icon.is_some());

        let update_req = UpdateContainerRequest {
            name: None,
            icon: Some(None), // clear icon
            description: None,
            status: None,
            location_id: None,
            color: None,
        };

        let updated = update(&pool, &created.id, &uid, &update_req)
            .await
            .unwrap()
            .unwrap();

        assert!(updated.icon.is_none());
    }

    #[tokio::test]
    async fn delete_tree_removes_empty_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let created = insert(&pool, &uid, &make_req("To Delete"), 0)
            .await
            .unwrap();

        let summary = subtree_summary(&pool, &created.id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            summary,
            SubtreeSummary {
                lists: 0,
                containers: 0,
                items: 0
            }
        );

        let outcome = delete_tree(&pool, &created.id, &uid).await.unwrap();
        assert_eq!(outcome, DeleteTreeOutcome::Deleted);

        let fetched = get_one(&pool, &created.id, &uid).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_tree_other_user_is_not_found() {
        let pool = test_pool().await;
        let uid_a = create_test_user(&pool).await;
        let uid_b = create_test_user(&pool).await;

        let created = insert(&pool, &uid_a, &make_req("A's"), 0).await.unwrap();

        assert!(
            subtree_summary(&pool, &created.id, &uid_b)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            delete_tree(&pool, &created.id, &uid_b).await.unwrap(),
            DeleteTreeOutcome::NotFound
        );
        assert!(get_one(&pool, &created.id, &uid_a).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn delete_tree_removes_container_with_nested_list_and_sublist() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let parent = create_test_container(&pool, &uid, "Parent", None).await;
        let child = create_test_container(&pool, &uid, "Child", Some(&parent.id)).await;
        let list = create_test_list(&pool, &uid, "List", Some(&child.id), None).await;
        let sublist = create_test_list(&pool, &uid, "Sublist", Some(&child.id), Some(&list)).await;

        let summary = subtree_summary(&pool, &parent.id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            summary,
            SubtreeSummary {
                lists: 2,
                containers: 1,
                items: 0
            }
        );

        let outcome = delete_tree(&pool, &parent.id, &uid).await.unwrap();
        assert_eq!(outcome, DeleteTreeOutcome::Deleted);

        for id in [&parent.id, &child.id] {
            assert!(
                get_one(&pool, id, &uid).await.unwrap().is_none(),
                "container {id} should be gone"
            );
        }
        for id in [&list, &sublist] {
            assert!(
                crate::lists::get_one(&pool, id, &uid)
                    .await
                    .unwrap()
                    .is_none(),
                "list {id} should be gone"
            );
        }
    }

    #[tokio::test]
    async fn delete_tree_removes_deep_sublist_chain_in_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let container = create_test_container(&pool, &uid, "Box", None).await;
        let a = create_test_list(&pool, &uid, "A", Some(&container.id), None).await;
        let b = create_test_list(&pool, &uid, "B", Some(&container.id), Some(&a)).await;
        let c = create_test_list(&pool, &uid, "C", Some(&container.id), Some(&b)).await;

        let outcome = delete_tree(&pool, &container.id, &uid).await.unwrap();
        assert_eq!(outcome, DeleteTreeOutcome::Deleted);

        assert!(get_one(&pool, &container.id, &uid).await.unwrap().is_none());
        for id in [&a, &b, &c] {
            assert!(
                crate::lists::get_one(&pool, id, &uid)
                    .await
                    .unwrap()
                    .is_none(),
                "list {id} should be gone"
            );
        }
    }

    #[tokio::test]
    async fn delete_tree_blocks_on_item_in_nested_list() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let parent = create_test_container(&pool, &uid, "Parent", None).await;
        let child = create_test_container(&pool, &uid, "Child", Some(&parent.id)).await;
        let list = create_test_list(&pool, &uid, "List", Some(&child.id), None).await;
        sqlx::query("INSERT INTO items (id, list_id, title, completed) VALUES (?, ?, 'Item', 0)")
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&list)
            .execute(&pool)
            .await
            .unwrap();

        let summary = subtree_summary(&pool, &parent.id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(summary.items, 1);
        assert_eq!(summary.lists, 1);
        assert_eq!(summary.containers, 1);

        let outcome = delete_tree(&pool, &parent.id, &uid).await.unwrap();
        assert_eq!(outcome, DeleteTreeOutcome::NotEmpty { items: 1 });

        // Nothing was deleted.
        assert!(get_one(&pool, &parent.id, &uid).await.unwrap().is_some());
        assert!(get_one(&pool, &child.id, &uid).await.unwrap().is_some());
        assert!(
            crate::lists::get_one(&pool, &list, &uid)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn children_returns_direct_children_only() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let parent = insert(&pool, &uid, &make_req("Parent"), 0).await.unwrap();

        let mut child_req = make_req("Child");
        child_req.parent_container_id = Some(parent.id.clone());
        let child = insert(&pool, &uid, &child_req, 0).await.unwrap();

        let mut grandchild_req = make_req("Grandchild");
        grandchild_req.parent_container_id = Some(child.id.clone());
        insert(&pool, &uid, &grandchild_req, 0).await.unwrap();

        let result = children(&pool, &parent.id, &uid).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, child.id);
    }

    #[tokio::test]
    async fn root_returns_top_level_only() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let root_c = insert(&pool, &uid, &make_req("Root"), 0).await.unwrap();

        let mut child_req = make_req("Child");
        child_req.parent_container_id = Some(root_c.id.clone());
        insert(&pool, &uid, &child_req, 0).await.unwrap();

        let result = root(&pool, &uid).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, root_c.id);
    }

    #[tokio::test]
    async fn pinned_returns_only_pinned() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let created = insert(&pool, &uid, &make_req("Unpinned"), 0).await.unwrap();

        let pinned_list = pinned(&pool, &uid).await.unwrap();
        assert!(pinned_list.is_empty());

        toggle_pin(&pool, &created.id, &uid).await.unwrap();

        let pinned_list = pinned(&pool, &uid).await.unwrap();
        assert_eq!(pinned_list.len(), 1);
        assert_eq!(pinned_list[0].id, created.id);
    }

    #[tokio::test]
    async fn toggle_pin_flips_state() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let created = insert(&pool, &uid, &make_req("Toggleable"), 0)
            .await
            .unwrap();
        assert!(!created.pinned);

        let after_first = toggle_pin(&pool, &created.id, &uid).await.unwrap().unwrap();
        assert!(after_first.pinned);

        let after_second = toggle_pin(&pool, &created.id, &uid).await.unwrap().unwrap();
        assert!(!after_second.pinned);
    }

    #[tokio::test]
    async fn move_container_updates_parent_and_position() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let parent = insert(&pool, &uid, &make_req("Parent"), 0).await.unwrap();
        let child = insert(&pool, &uid, &make_req("Child"), 0).await.unwrap();
        assert!(child.parent_container_id.is_none());

        let moved = move_container(&pool, &child.id, &uid, Some(&parent.id), 5)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            moved.parent_container_id.as_deref(),
            Some(parent.id.as_str())
        );
        assert_eq!(moved.position, 5);
    }

    #[tokio::test]
    async fn is_descendant_detects_cycle() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let a = insert(&pool, &uid, &make_req("A"), 0).await.unwrap();
        let mut b_req = make_req("B");
        b_req.parent_container_id = Some(a.id.clone());
        let b = insert(&pool, &uid, &b_req, 0).await.unwrap();

        // B is a descendant of A
        assert!(is_descendant(&pool, &uid, &a.id, &b.id).await.unwrap());
        // A is NOT a descendant of B
        assert!(!is_descendant(&pool, &uid, &b.id, &a.id).await.unwrap());
        // A is a "descendant" of itself (included in subtree)
        assert!(is_descendant(&pool, &uid, &a.id, &a.id).await.unwrap());
    }

    #[tokio::test]
    async fn progress_counts_lists_and_items() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let container = insert(&pool, &uid, &make_req("Project"), 0).await.unwrap();

        // Insert a list in the container
        let list_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO lists (id, user_id, name, container_id) VALUES (?, ?, 'Test List', ?)",
        )
        .bind(&list_id)
        .bind(&uid)
        .bind(&container.id)
        .execute(&pool)
        .await
        .unwrap();

        // Insert 2 items, 1 completed
        let item1_id = uuid::Uuid::new_v4().to_string();
        let item2_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO items (id, list_id, title, completed) VALUES (?, ?, 'Item 1', 1)")
            .bind(&item1_id)
            .bind(&list_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, completed) VALUES (?, ?, 'Item 2', 0)")
            .bind(&item2_id)
            .bind(&list_id)
            .execute(&pool)
            .await
            .unwrap();

        let prog = progress(&pool, &container.id, &uid).await.unwrap();
        assert_eq!(prog.total_lists, 1);
        assert_eq!(prog.total_items, 2);
        assert_eq!(prog.completed_items, 1);
    }

    #[tokio::test]
    async fn find_owned_ids_returns_only_owned() {
        let pool = test_pool().await;
        let uid_a = create_test_user(&pool).await;
        let uid_b = create_test_user(&pool).await;

        let a = insert(&pool, &uid_a, &make_req("A"), 0).await.unwrap();
        let b = insert(&pool, &uid_b, &make_req("B"), 0).await.unwrap();

        let found = find_owned_ids(&pool, &uid_a, &[a.id.as_str(), b.id.as_str()])
            .await
            .unwrap();
        assert!(found.contains(&a.id));
        assert!(!found.contains(&b.id));
    }

    #[tokio::test]
    async fn find_owned_ids_empty_input_returns_empty() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let found = find_owned_ids(&pool, &uid, &[]).await.unwrap();
        assert!(found.is_empty());
    }

    #[tokio::test]
    async fn next_position_increments() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        // Empty: first position should be 0
        let pos0 = next_position(&pool, &uid, None).await.unwrap();
        assert_eq!(pos0, 0);

        // Insert one container at position 0
        insert(&pool, &uid, &make_req("First"), 0).await.unwrap();

        // Next position should be 1
        let pos1 = next_position(&pool, &uid, None).await.unwrap();
        assert_eq!(pos1, 1);
    }

    #[tokio::test]
    async fn toggle_archived_flips_and_cascades() {
        use crate::lists as db_lists;

        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        // parent → child hierarchy
        let parent = create_test_container(&pool, &uid, "Parent", None).await;
        let child = create_test_container(&pool, &uid, "Child", Some(&parent.id)).await;

        // list attached to the child container
        let list_id = uuid::Uuid::new_v4().to_string();
        let mut tx = pool.begin().await.unwrap();
        db_lists::insert(
            &mut tx,
            &db_lists::InsertListInput {
                id: list_id.clone(),
                user_id: uid.clone(),
                name: "MyList".into(),
                list_type: "checklist".into(),
                container_id: Some(child.id.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        // archive parent — should cascade to child and list
        let affected = toggle_archived(&pool, &parent.id, &uid).await.unwrap();
        assert!(affected);

        let after_parent = get_one(&pool, &parent.id, &uid).await.unwrap().unwrap();
        assert!(after_parent.archived);

        let after_child = get_one(&pool, &child.id, &uid).await.unwrap().unwrap();
        assert!(after_child.archived, "child should be archived by cascade");

        let after_list = db_lists::get_one(&pool, &list_id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert!(
            after_list.archived != 0,
            "list should be archived by cascade"
        );

        // restore parent — cascade back
        toggle_archived(&pool, &parent.id, &uid).await.unwrap();
        let restored_child = get_one(&pool, &child.id, &uid).await.unwrap().unwrap();
        assert!(
            !restored_child.archived,
            "child should be restored by cascade"
        );
    }

    #[tokio::test]
    async fn list_archived_returns_root_only_not_cascaded_children() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let parent = create_test_container(&pool, &uid, "Parent", None).await;
        let _child = create_test_container(&pool, &uid, "Child", Some(&parent.id)).await;

        // archive parent — cascades to child
        toggle_archived(&pool, &parent.id, &uid).await.unwrap();

        // list_archived should return ONLY the parent (root of archived subtree)
        let archived = list_archived(&pool, &uid).await.unwrap();
        assert_eq!(
            archived.len(),
            1,
            "should return only root of archived subtree, not cascaded children"
        );
        assert_eq!(archived[0].id, parent.id);
    }

    #[tokio::test]
    async fn touch_last_opened_sets_only_last_opened_at() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let created = create_test_container(&pool, &uid, "Touched", None).await;
        assert!(created.last_opened_at.is_none());

        touch_last_opened(&pool, &created.id, &uid).await.unwrap();

        let after = get_one(&pool, &created.id, &uid).await.unwrap().unwrap();
        assert!(after.last_opened_at.is_some());
        assert_eq!(
            after.updated_at, created.updated_at,
            "opening a container is a read and must not bump updated_at"
        );
    }

    #[tokio::test]
    async fn touch_last_opened_is_scoped_by_user() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let created = create_test_container(&pool, &owner, "Owned", None).await;

        touch_last_opened(&pool, &created.id, &other).await.unwrap();

        let row = get_one(&pool, &created.id, &owner).await.unwrap().unwrap();
        assert!(row.last_opened_at.is_none());
    }
}
