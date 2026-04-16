use crate::DomainError;
use kartoteka_db as db;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

// ── Public domain type ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub content: String,
    pub author_type: String,
    pub author_name: Option<String>,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to_comment(row: db::types::CommentRow) -> Comment {
    Comment {
        id: row.id,
        entity_type: row.entity_type,
        entity_id: row.entity_id,
        content: row.content,
        author_type: row.author_type,
        author_name: row.author_name,
        user_id: row.user_id,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

// ── Ownership check ───────────────────────────────────────────────────────────

async fn verify_entity_ownership(
    pool: &SqlitePool,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), DomainError> {
    let exists = match entity_type {
        "item" => db::items::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        "list" => db::lists::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        "container" => db::containers::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        _ => return Err(DomainError::Validation("invalid_entity_type")),
    };
    if !exists {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

// ── Orchestration ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_for_entity(
    pool: &SqlitePool,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<Comment>, DomainError> {
    verify_entity_ownership(pool, user_id, entity_type, entity_id).await?;
    let rows = db::comments::list_for_entity(pool, entity_type, entity_id).await?;
    Ok(rows.into_iter().map(row_to_comment).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
    content: &str,
    author_type: &str,
    author_name: Option<&str>,
) -> Result<Comment, DomainError> {
    verify_entity_ownership(pool, user_id, entity_type, entity_id).await?;
    let id = Uuid::new_v4().to_string();
    let row = db::comments::insert(
        pool,
        db::comments::InsertCommentInput {
            id: &id,
            entity_type,
            entity_id,
            content,
            author_type,
            author_name,
            user_id,
        },
    )
    .await?;
    Ok(row_to_comment(row))
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, comment_id: &str) -> Result<(), DomainError> {
    db::comments::delete(pool, comment_id, user_id).await?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};
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
    async fn create_comment_on_owned_list() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;

        let comment = create(
            &pool,
            &user_id,
            "list",
            &list_id,
            "Great list!",
            "user",
            None,
        )
        .await
        .unwrap();

        assert_eq!(comment.content, "Great list!");
        assert_eq!(comment.entity_type, "list");
        assert_eq!(comment.entity_id, list_id);
        assert_eq!(comment.author_type, "user");
        assert!(!comment.id.is_empty());
    }

    #[tokio::test]
    async fn create_comment_forbidden_on_other_users_list() {
        let pool = test_pool().await;
        let owner_id = create_test_user(&pool).await;
        let attacker_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &owner_id).await;

        let result = create(
            &pool,
            &attacker_id,
            "list",
            &list_id,
            "injected!",
            "user",
            None,
        )
        .await;
        assert!(matches!(result, Err(DomainError::Forbidden)));
    }

    #[tokio::test]
    async fn list_comments_forbidden_on_other_users_list() {
        let pool = test_pool().await;
        let owner_id = create_test_user(&pool).await;
        let attacker_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &owner_id).await;

        let result = list_for_entity(&pool, &attacker_id, "list", &list_id).await;
        assert!(matches!(result, Err(DomainError::Forbidden)));
    }

    #[tokio::test]
    async fn list_comments_returns_all_for_entity() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;

        create(&pool, &user_id, "list", &list_id, "first", "user", None)
            .await
            .unwrap();
        create(&pool, &user_id, "list", &list_id, "second", "user", None)
            .await
            .unwrap();

        let comments = list_for_entity(&pool, &user_id, "list", &list_id)
            .await
            .unwrap();
        assert_eq!(comments.len(), 2);
    }

    #[tokio::test]
    async fn delete_own_comment() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;

        let comment = create(&pool, &user_id, "list", &list_id, "to delete", "user", None)
            .await
            .unwrap();

        delete(&pool, &user_id, &comment.id).await.unwrap();

        let comments = list_for_entity(&pool, &user_id, "list", &list_id)
            .await
            .unwrap();
        assert!(comments.is_empty());
    }

    #[tokio::test]
    async fn invalid_entity_type_returns_validation_error() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;

        let result = create(&pool, &user_id, "bogus", "any-id", "oops", "user", None).await;
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }
}
