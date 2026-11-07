use crate::DbError;
use sqlx::SqlitePool;

/// One row of the `pending_deletions` queue, as stored.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingDeletionRow {
    pub entity_type: String,
    pub entity_id: String,
    pub marked_at: String,
}

/// Mark an entity for deletion. Marking twice is a no-op (`INSERT OR IGNORE`).
///
/// NOTE: entity ownership is NOT checked here — callers must verify it first.
#[tracing::instrument(skip(pool))]
pub async fn insert(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: &str,
    user_id: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT OR IGNORE INTO pending_deletions (entity_type, entity_id, user_id) \
         VALUES (?, ?, ?)",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

/// Unmark an entity. Returns true when a row was removed.
#[tracing::instrument(skip(pool))]
pub async fn remove(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "DELETE FROM pending_deletions \
         WHERE entity_type = ? AND entity_id = ? AND user_id = ?",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn is_marked(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM pending_deletions \
         WHERE entity_type = ? AND entity_id = ? AND user_id = ?",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row.is_some())
}

/// The user's whole queue, newest first.
#[tracing::instrument(skip(pool))]
pub async fn list_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<PendingDeletionRow>, DbError> {
    // `marked_at` has second precision, so `rowid DESC` breaks ties and keeps a
    // freshly marked row on top.
    sqlx::query_as::<_, PendingDeletionRow>(
        "SELECT entity_type, entity_id, marked_at FROM pending_deletions \
         WHERE user_id = ? \
         ORDER BY marked_at DESC, rowid DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_item, create_test_list, create_test_user, test_pool};
    use sqlx::SqlitePool;
    use uuid::Uuid;

    async fn insert_container(pool: &SqlitePool, user_id: &str, name: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO containers (id, user_id, name) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(user_id)
            .bind(name)
            .execute(pool)
            .await
            .expect("insert_container");
        id
    }

    async fn insert_list_named(
        pool: &SqlitePool,
        user_id: &str,
        name: &str,
        parent_list_id: Option<&str>,
        container_id: Option<&str>,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO lists (id, user_id, name, parent_list_id, container_id) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(parent_list_id)
        .bind(container_id)
        .execute(pool)
        .await
        .expect("insert_list_named");
        id
    }

    async fn insert_comment(
        pool: &SqlitePool,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
        content: &str,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO comments (id, entity_type, entity_id, content, user_id) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(content)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_comment");
        id
    }

    async fn row_count(pool: &SqlitePool, user_id: &str) -> i64 {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM pending_deletions WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .unwrap();
        row.0
    }

    #[tokio::test]
    async fn insert_is_idempotent_and_listed_newest_first() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        insert(&pool, "item", &item_id, &uid).await.unwrap();
        insert(&pool, "item", &item_id, &uid).await.unwrap();
        insert(&pool, "list", &list_id, &uid).await.unwrap();

        assert!(is_marked(&pool, "item", &item_id, &uid).await.unwrap());
        let rows = list_for_user(&pool, &uid).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].entity_type, "list");
        assert_eq!(rows[0].entity_id, list_id);
        assert!(!rows[0].marked_at.is_empty());
        assert_eq!(rows[1].entity_type, "item");
    }

    #[tokio::test]
    async fn remove_returns_false_for_missing_row() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        insert(&pool, "item", &item_id, &uid).await.unwrap();
        assert!(remove(&pool, "item", &item_id, &uid).await.unwrap());
        assert!(!remove(&pool, "item", &item_id, &uid).await.unwrap());
        assert!(!is_marked(&pool, "item", &item_id, &uid).await.unwrap());
    }

    #[tokio::test]
    async fn list_for_user_is_scoped_to_the_owner() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let other_list = create_test_list(&pool, &other).await;

        insert(&pool, "list", &list_id, &uid).await.unwrap();
        insert(&pool, "list", &other_list, &other).await.unwrap();

        let rows = list_for_user(&pool, &uid).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].entity_id, list_id);
        assert!(!is_marked(&pool, "list", &list_id, &other).await.unwrap());
    }

    #[tokio::test]
    async fn deleting_an_item_clears_its_queue_row() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        insert(&pool, "item", &item_id, &uid).await.unwrap();
        sqlx::query("DELETE FROM items WHERE id = ?")
            .bind(&item_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(row_count(&pool, &uid).await, 0);
    }

    #[tokio::test]
    async fn deleting_a_list_cascades_through_items_and_comments() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root = insert_list_named(&pool, &uid, "Root", None, None).await;
        let child = insert_list_named(&pool, &uid, "Child", Some(&root), None).await;
        let item_id = create_test_item(&pool, &child).await;
        let item_comment = insert_comment(&pool, &uid, "item", &item_id, "on item").await;
        let list_comment = insert_comment(&pool, &uid, "list", &root, "on list").await;

        insert(&pool, "list", &root, &uid).await.unwrap();
        insert(&pool, "item", &item_id, &uid).await.unwrap();
        insert(&pool, "comment", &item_comment, &uid).await.unwrap();
        insert(&pool, "comment", &list_comment, &uid).await.unwrap();
        assert_eq!(row_count(&pool, &uid).await, 4);

        // The FK cascade drops the items, 017 drops the comments of both the item
        // and the list; every deleted row takes its queue row with it. Parent and
        // child list go in one statement because `parent_list_id` has no ON DELETE
        // action (the same reason `lists::delete_tree` uses one statement).
        sqlx::query("DELETE FROM lists WHERE id IN (?, ?)")
            .bind(&root)
            .bind(&child)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(row_count(&pool, &uid).await, 0);
    }

    #[tokio::test]
    async fn deleting_a_container_clears_rows_of_its_whole_tree() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container = insert_container(&pool, &uid, "Crate").await;
        let sub_container = {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO containers (id, user_id, name, parent_container_id) \
                 VALUES (?, ?, 'Inner', ?)",
            )
            .bind(&id)
            .bind(&uid)
            .bind(&container)
            .execute(&pool)
            .await
            .unwrap();
            id
        };
        let list_id = insert_list_named(&pool, &uid, "Boxed", None, Some(&container)).await;
        let item_id = create_test_item(&pool, &list_id).await;

        insert(&pool, "container", &container, &uid).await.unwrap();
        insert(&pool, "container", &sub_container, &uid)
            .await
            .unwrap();
        insert(&pool, "list", &list_id, &uid).await.unwrap();
        insert(&pool, "item", &item_id, &uid).await.unwrap();
        assert_eq!(row_count(&pool, &uid).await, 4);

        sqlx::query("DELETE FROM lists WHERE id = ?")
            .bind(&list_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM containers WHERE id IN (?, ?)")
            .bind(&sub_container)
            .bind(&container)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(row_count(&pool, &uid).await, 0);
    }

    #[tokio::test]
    async fn deleting_a_comment_clears_its_queue_row() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let comment_id = insert_comment(&pool, &uid, "list", &list_id, "bye").await;

        insert(&pool, "comment", &comment_id, &uid).await.unwrap();
        sqlx::query("DELETE FROM comments WHERE id = ?")
            .bind(&comment_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(row_count(&pool, &uid).await, 0);
        assert!(
            list_for_user(&pool, &uid).await.unwrap().is_empty(),
            "the queue should not point at a missing comment"
        );
    }

    #[tokio::test]
    async fn row_types_still_load_after_marking() {
        // Guards the queue against breaking the ordinary row queries.
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container = insert_container(&pool, &uid, "Crate").await;
        let list_id = insert_list_named(&pool, &uid, "Boxed", None, Some(&container)).await;
        let item_id = create_test_item(&pool, &list_id).await;
        let comment_id = insert_comment(&pool, &uid, "item", &item_id, "hi").await;

        insert(&pool, "container", &container, &uid).await.unwrap();
        insert(&pool, "list", &list_id, &uid).await.unwrap();
        insert(&pool, "item", &item_id, &uid).await.unwrap();
        insert(&pool, "comment", &comment_id, &uid).await.unwrap();

        assert!(
            crate::containers::get_one(&pool, &container, &uid)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            crate::lists::get_one(&pool, &list_id, &uid)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            crate::items::get_one(&pool, &item_id, &uid)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            crate::comments::get_owned(&pool, &comment_id, &uid)
                .await
                .unwrap()
                .is_some()
        );
    }
}
