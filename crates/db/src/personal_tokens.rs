use crate::{DbError, SqlitePool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct PersonalTokenRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub scope: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

pub async fn create(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    name: &str,
    scope: &str,
    expires_at: Option<&str>,
) -> Result<PersonalTokenRow, DbError> {
    sqlx::query_as::<_, PersonalTokenRow>(
        "INSERT INTO personal_tokens (id, user_id, name, scope, expires_at) \
         VALUES (?, ?, ?, ?, ?) \
         RETURNING id, user_id, name, scope, last_used_at, expires_at, created_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(scope)
    .bind(expires_at)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

pub async fn list_by_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<PersonalTokenRow>, DbError> {
    sqlx::query_as::<_, PersonalTokenRow>(
        "SELECT id, user_id, name, scope, last_used_at, expires_at, created_at \
         FROM personal_tokens WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Delete a token. Returns true if found and owned by user_id, false otherwise.
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let result = sqlx::query("DELETE FROM personal_tokens WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(result.rows_affected() > 0)
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<PersonalTokenRow>, DbError> {
    sqlx::query_as::<_, PersonalTokenRow>(
        "SELECT id, user_id, name, scope, last_used_at, expires_at, created_at \
         FROM personal_tokens WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Update last_used_at to current time.
pub async fn touch_last_used(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE personal_tokens SET last_used_at = datetime('now') WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn create_and_find_token() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let row = create(&pool, "tok-1", &user_id, "My Token", "full", None)
            .await
            .unwrap();
        assert_eq!(row.id, "tok-1");
        assert_eq!(row.scope, "full");
        assert!(row.last_used_at.is_none());
        assert!(row.expires_at.is_none());

        let found = find_by_id(&pool, "tok-1").await.unwrap().unwrap();
        assert_eq!(found.name, "My Token");
        assert_eq!(found.user_id, user_id);
    }

    #[tokio::test]
    async fn create_with_expires_at() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let row = create(
            &pool,
            "tok-2",
            &user_id,
            "Expiring",
            "calendar",
            Some("2030-01-01T00:00:00Z"),
        )
        .await
        .unwrap();
        assert_eq!(row.expires_at.unwrap(), "2030-01-01T00:00:00Z");
    }

    #[tokio::test]
    async fn find_by_id_unknown_returns_none() {
        let pool = test_pool().await;
        let result = find_by_id(&pool, "no-such-id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_by_user_returns_owned_tokens() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        create(&pool, "t1", &user_id, "Token A", "full", None)
            .await
            .unwrap();
        create(&pool, "t2", &user_id, "Token B", "calendar", None)
            .await
            .unwrap();
        let list = list_by_user(&pool, &user_id).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn list_by_user_excludes_other_users() {
        let pool = test_pool().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_test_user(&pool).await;
        create(&pool, "t1", &user1, "Token", "full", None)
            .await
            .unwrap();
        let list = list_by_user(&pool, &user2).await.unwrap();
        assert_eq!(list.len(), 0);
    }

    #[tokio::test]
    async fn delete_removes_token_returns_true() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        create(&pool, "tok-1", &user_id, "Token", "full", None)
            .await
            .unwrap();
        let deleted = delete(&pool, "tok-1", &user_id).await.unwrap();
        assert!(deleted);
        assert!(find_by_id(&pool, "tok-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_wrong_user_returns_false_and_leaves_row() {
        let pool = test_pool().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_test_user(&pool).await;
        create(&pool, "tok-1", &user1, "Token", "full", None)
            .await
            .unwrap();
        let deleted = delete(&pool, "tok-1", &user2).await.unwrap();
        assert!(!deleted);
        assert!(find_by_id(&pool, "tok-1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn touch_last_used_sets_timestamp() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        create(&pool, "tok-1", &user_id, "Token", "full", None)
            .await
            .unwrap();
        touch_last_used(&pool, "tok-1").await.unwrap();
        let row = find_by_id(&pool, "tok-1").await.unwrap().unwrap();
        assert!(row.last_used_at.is_some());
    }
}
