use crate::DbError;
use sqlx::SqlitePool;

/// Get a server config value by key. Returns None if key not found.
#[tracing::instrument(skip(pool))]
pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, DbError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM server_config WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await
            .map_err(DbError::Sqlx)?;
    Ok(row.map(|(v,)| v))
}

/// Upsert a server config key-value pair.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_pool;

    #[tokio::test]
    async fn set_and_get_value() {
        let pool = test_pool().await;
        set(&pool, "registration_enabled", "false").await.unwrap();
        assert_eq!(
            get(&pool, "registration_enabled").await.unwrap().as_deref(),
            Some("false")
        );
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let pool = test_pool().await;
        assert!(get(&pool, "nonexistent_key").await.unwrap().is_none());
    }
}
