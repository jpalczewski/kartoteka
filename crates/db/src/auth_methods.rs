use crate::{DbError, types::AuthMethodRow};
use sqlx::SqlitePool;

#[tracing::instrument(skip(pool))]
pub async fn find_by_user_and_provider(
    pool: &SqlitePool,
    user_id: &str,
    provider: &str,
) -> Result<Option<AuthMethodRow>, DbError> {
    sqlx::query_as::<_, AuthMethodRow>(
        "SELECT id, user_id, provider, provider_id, credential, created_at \
         FROM auth_methods WHERE user_id = ? AND provider = ?",
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    provider: &str,
    provider_id: &str,
    credential: Option<&str>,
) -> Result<AuthMethodRow, DbError> {
    sqlx::query_as::<_, AuthMethodRow>(
        "INSERT INTO auth_methods (id, user_id, provider, provider_id, credential) \
         VALUES (?, ?, ?, ?, ?) \
         RETURNING id, user_id, provider, provider_id, credential, created_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(provider)
    .bind(provider_id)
    .bind(credential)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
    use uuid::Uuid;

    #[tokio::test]
    async fn create_and_find_password_method() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = Uuid::new_v4().to_string();

        let created = create(
            &pool,
            &id,
            &user_id,
            "password",
            &user_id,
            Some("hashed_secret"),
        )
        .await
        .unwrap();

        assert_eq!(created.id, id);
        assert_eq!(created.user_id, user_id);
        assert_eq!(created.provider, "password");
        assert_eq!(created.credential.as_deref(), Some("hashed_secret"));

        let found = find_by_user_and_provider(&pool, &user_id, "password")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, id);
        assert_eq!(found.provider, "password");
    }

    #[tokio::test]
    async fn find_missing_provider_returns_none() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;

        let result = find_by_user_and_provider(&pool, &user_id, "github")
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
