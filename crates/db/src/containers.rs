use crate::types::ContainerRow;
use crate::{DbError, SqlitePool};
use kartoteka_shared::types::{CreateContainerRequest, UpdateContainerRequest};
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
    let rows = sqlx::query_as("SELECT * FROM containers WHERE user_id = ? ORDER BY position ASC")
        .bind(user_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Get a single container by id and user_id.
#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<ContainerRow>, DbError> {
    let row = sqlx::query_as("SELECT * FROM containers WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
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
    let row = sqlx::query_as(
        r#"INSERT INTO containers (id, user_id, name, icon, description, status, parent_container_id, position)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)
           RETURNING *"#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(&req.name)
    .bind(&req.icon)
    .bind(&req.description)
    .bind(&req.status)
    .bind(&req.parent_container_id)
    .bind(position)
    .fetch_one(pool)
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
    let description = req
        .description
        .as_ref()
        .map_or(existing.description.as_deref(), |v| v.as_deref());
    let status = req
        .status
        .as_ref()
        .map_or(existing.status.as_deref(), |v| v.as_deref());

    let row = sqlx::query_as(
        r#"UPDATE containers
           SET name=?, icon=?, description=?, status=?, updated_at=datetime('now')
           WHERE id=? AND user_id=?
           RETURNING *"#,
    )
    .bind(name)
    .bind(icon)
    .bind(description)
    .bind(status)
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Delete a container. Returns true if a row was deleted.
#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let result = sqlx::query("DELETE FROM containers WHERE id=? AND user_id=?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// List direct children of a container.
#[tracing::instrument(skip(pool))]
pub async fn children(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<ContainerRow>, DbError> {
    let rows = sqlx::query_as(
        "SELECT * FROM containers WHERE parent_container_id=? AND user_id=? ORDER BY position ASC",
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
        "SELECT * FROM containers WHERE user_id=? AND parent_container_id IS NULL ORDER BY position ASC",
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
        "SELECT * FROM containers WHERE user_id=? AND pinned=1 ORDER BY position ASC",
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
        "SELECT * FROM containers WHERE user_id=? ORDER BY COALESCE(last_opened_at, updated_at) DESC LIMIT 10",
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
#[tracing::instrument(skip(pool))]
pub async fn move_container(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    parent_id: Option<&str>,
    position: i32,
) -> Result<Option<ContainerRow>, DbError> {
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
    .fetch_optional(pool)
    .await?;
    Ok(row)
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
#[tracing::instrument(skip(pool))]
pub async fn progress(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<ContainerProgressRow, DbError> {
    let row = sqlx::query_as(
        r#"SELECT COUNT(DISTINCT l.id) as total_lists,
                  COUNT(i.id) as total_items,
                  COALESCE(SUM(CASE WHEN i.completed=1 THEN 1 ELSE 0 END), 0) as completed_items
           FROM lists l
           LEFT JOIN items i ON i.list_id = l.id
           WHERE l.container_id = ? AND l.user_id = ?"#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Update last_opened_at for a container.
#[tracing::instrument(skip(pool))]
pub async fn touch_last_opened(pool: &SqlitePool, id: &str, user_id: &str) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE containers SET last_opened_at=datetime('now'), updated_at=datetime('now') WHERE id=? AND user_id=?",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
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
        };

        let updated = update(&pool, &created.id, &uid, &update_req)
            .await
            .unwrap()
            .unwrap();

        assert!(updated.icon.is_none());
    }

    #[tokio::test]
    async fn delete_removes_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let created = insert(&pool, &uid, &make_req("To Delete"), 0)
            .await
            .unwrap();
        let deleted = delete(&pool, &created.id, &uid).await.unwrap();
        assert!(deleted);

        let fetched = get_one(&pool, &created.id, &uid).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_wrong_user_returns_false() {
        let pool = test_pool().await;
        let uid_a = create_test_user(&pool).await;
        let uid_b = create_test_user(&pool).await;

        let created = insert(&pool, &uid_a, &make_req("A's"), 0).await.unwrap();
        let result = delete(&pool, &created.id, &uid_b).await.unwrap();
        assert!(!result);
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
}
