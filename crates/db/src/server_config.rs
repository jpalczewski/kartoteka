use crate::{types::ServerConfigRow, DbError};
use sqlx::SqlitePool;

#[tracing::instrument(skip(pool))]
pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, DbError> {
    let row = sqlx::query_as::<_, ServerConfigRow>(
        "SELECT key, value FROM server_config WHERE key = ?",
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row.map(|r| r.value))
}

#[tracing::instrument(skip(pool))]
pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO server_config (key, value) VALUES (?, ?) \
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn is_registration_enabled(pool: &SqlitePool) -> Result<bool, DbError> {
    let value = get(pool, "registration_enabled").await?;
    Ok(value.as_deref() != Some("false"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_pool;

    #[tokio::test]
    async fn migration_sets_registration_enabled_true() {
        let pool = test_pool().await;
        assert!(is_registration_enabled(&pool).await.unwrap());
    }

    #[tokio::test]
    async fn set_and_get_value() {
        let pool = test_pool().await;
        set(&pool, "maintenance_mode", "on").await.unwrap();
        let val = get(&pool, "maintenance_mode").await.unwrap();
        assert_eq!(val.as_deref(), Some("on"));
    }

    #[tokio::test]
    async fn is_registration_enabled_false_when_set_false() {
        let pool = test_pool().await;
        set(&pool, "registration_enabled", "false").await.unwrap();
        assert!(!is_registration_enabled(&pool).await.unwrap());
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let pool = test_pool().await;
        let val = get(&pool, "nonexistent_key").await.unwrap();
        assert!(val.is_none());
    }
}
