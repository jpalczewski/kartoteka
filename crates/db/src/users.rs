use crate::{types::UserRow, DbError};
use sqlx::SqlitePool;

#[tracing::instrument(skip(pool))]
pub async fn find_by_email(pool: &SqlitePool, email: &str) -> Result<Option<UserRow>, DbError> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, email, name, avatar_url, role, created_at, updated_at \
         FROM users WHERE email = ?",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<UserRow>, DbError> {
    sqlx::query_as::<_, UserRow>(
        "SELECT id, email, name, avatar_url, role, created_at, updated_at \
         FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn count(pool: &SqlitePool) -> Result<i64, DbError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(row.0)
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    id: &str,
    email: &str,
    name: Option<&str>,
    role: &str,
) -> Result<UserRow, DbError> {
    sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (id, email, name, role) \
         VALUES (?, ?, ?, ?) \
         RETURNING id, email, name, avatar_url, role, created_at, updated_at",
    )
    .bind(id)
    .bind(email)
    .bind(name)
    .bind(role)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_pool;
    use uuid::Uuid;

    #[tokio::test]
    async fn create_and_find_by_email() {
        let pool = test_pool().await;
        let id = Uuid::new_v4().to_string();
        let email = "alice@example.com";

        let created = create(&pool, &id, email, Some("Alice"), "user")
            .await
            .unwrap();
        assert_eq!(created.id, id);
        assert_eq!(created.email, email);
        assert_eq!(created.name.as_deref(), Some("Alice"));
        assert_eq!(created.role, "user");

        let found = find_by_email(&pool, email).await.unwrap().unwrap();
        assert_eq!(found.id, id);
        assert_eq!(found.email, email);
    }

    #[tokio::test]
    async fn find_by_email_missing_returns_none() {
        let pool = test_pool().await;
        let result = find_by_email(&pool, "nobody@example.com").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn find_by_id_returns_user() {
        let pool = test_pool().await;
        let id = Uuid::new_v4().to_string();

        create(&pool, &id, "bob@example.com", None, "user")
            .await
            .unwrap();

        let found = find_by_id(&pool, &id).await.unwrap().unwrap();
        assert_eq!(found.id, id);
        assert_eq!(found.email, "bob@example.com");
        assert!(found.name.is_none());
    }

    #[tokio::test]
    async fn find_by_id_missing_returns_none() {
        let pool = test_pool().await;
        let result = find_by_id(&pool, "nonexistent-id").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn count_empty_db() {
        let pool = test_pool().await;
        let n = count(&pool).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn count_increments_after_create() {
        let pool = test_pool().await;

        create(
            &pool,
            &Uuid::new_v4().to_string(),
            "user1@example.com",
            None,
            "user",
        )
        .await
        .unwrap();
        create(
            &pool,
            &Uuid::new_v4().to_string(),
            "user2@example.com",
            None,
            "user",
        )
        .await
        .unwrap();

        let n = count(&pool).await.unwrap();
        assert_eq!(n, 2);
    }
}
