use crate::{DbError, types::CommentRow};
use sqlx::SqlitePool;

// ── Read queries ──────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_for_entity(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<CommentRow>, DbError> {
    todo!("implement")
}

// ── Write queries ─────────────────────────────────────────────────────────────

pub struct InsertCommentInput<'a> {
    pub id: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub content: &'a str,
    pub author_type: &'a str,
    pub author_name: Option<&'a str>,
    pub user_id: &'a str,
}

#[tracing::instrument(skip(pool, input), fields(entity_type = %input.entity_type, entity_id = %input.entity_id))]
pub async fn insert(pool: &SqlitePool, input: InsertCommentInput<'_>) -> Result<CommentRow, DbError> {
    todo!("implement")
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, comment_id: &str, user_id: &str) -> Result<bool, DbError> {
    todo!("implement")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
    use uuid::Uuid;

    async fn insert_test_list(pool: &SqlitePool, user_id: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO lists (id, user_id, name, list_type) VALUES (?, ?, 'Test List', 'checklist')",
        )
        .bind(&id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_test_list");
        id
    }

    #[tokio::test]
    async fn insert_returns_comment() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;
        let comment_id = Uuid::new_v4().to_string();

        let row = insert(
            &pool,
            InsertCommentInput {
                id: &comment_id,
                entity_type: "list",
                entity_id: &list_id,
                content: "Hello, world!",
                author_type: "user",
                author_name: None,
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        assert_eq!(row.id, comment_id);
        assert_eq!(row.content, "Hello, world!");
        assert_eq!(row.entity_type, "list");
        assert_eq!(row.author_type, "user");
    }

    #[tokio::test]
    async fn list_for_entity_returns_in_asc_order() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;

        for i in 0..3u8 {
            insert(
                &pool,
                InsertCommentInput {
                    id: &Uuid::new_v4().to_string(),
                    entity_type: "list",
                    entity_id: &list_id,
                    content: &format!("comment {i}"),
                    author_type: "user",
                    author_name: None,
                    user_id: &user_id,
                },
            )
            .await
            .unwrap();
        }

        let rows = list_for_entity(&pool, "list", &list_id).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows[0].created_at <= rows[1].created_at);
        assert!(rows[1].created_at <= rows[2].created_at);
    }

    #[tokio::test]
    async fn delete_removes_comment() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;
        let comment_id = Uuid::new_v4().to_string();

        insert(
            &pool,
            InsertCommentInput {
                id: &comment_id,
                entity_type: "list",
                entity_id: &list_id,
                content: "to be deleted",
                author_type: "user",
                author_name: None,
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        let deleted = delete(&pool, &comment_id, &user_id).await.unwrap();
        assert!(deleted);

        let rows = list_for_entity(&pool, "list", &list_id).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn delete_wrong_user_returns_false() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let other_user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;
        let comment_id = Uuid::new_v4().to_string();

        insert(
            &pool,
            InsertCommentInput {
                id: &comment_id,
                entity_type: "list",
                entity_id: &list_id,
                content: "owner's comment",
                author_type: "user",
                author_name: None,
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        let deleted = delete(&pool, &comment_id, &other_user_id).await.unwrap();
        assert!(!deleted);

        let rows = list_for_entity(&pool, "list", &list_id).await.unwrap();
        assert_eq!(rows.len(), 1);
    }
}
