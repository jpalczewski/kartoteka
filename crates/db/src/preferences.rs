use crate::{DbError, settings};
use sqlx::SqlitePool;

#[tracing::instrument(skip(pool))]
pub async fn get_timezone(pool: &SqlitePool, user_id: &str) -> Result<String, DbError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = 'timezone'")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(DbError::Sqlx)?;
    Ok(row.map(|(v,)| v).unwrap_or_else(|| "UTC".to_string()))
}

#[tracing::instrument(skip(pool))]
pub async fn set_timezone(pool: &SqlitePool, user_id: &str, timezone: &str) -> Result<(), DbError> {
    settings::set(pool, user_id, "timezone", timezone).await?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn get_locale(pool: &SqlitePool, user_id: &str) -> Result<String, DbError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = 'locale'")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(DbError::Sqlx)?;
    Ok(row.map(|(v,)| v).unwrap_or_else(|| "en".to_string()))
}

#[tracing::instrument(skip(pool))]
pub async fn set_locale(pool: &SqlitePool, user_id: &str, locale: &str) -> Result<(), DbError> {
    settings::set(pool, user_id, "locale", locale).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn get_timezone_defaults_to_utc() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        assert_eq!(get_timezone(&pool, &uid).await.unwrap(), "UTC");
    }

    #[tokio::test]
    async fn set_and_get_timezone() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        set_timezone(&pool, &uid, "Europe/Warsaw").await.unwrap();
        assert_eq!(get_timezone(&pool, &uid).await.unwrap(), "Europe/Warsaw");
    }

    #[tokio::test]
    async fn get_locale_defaults_to_en() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        assert_eq!(get_locale(&pool, &uid).await.unwrap(), "en");
    }

    #[tokio::test]
    async fn set_and_get_locale() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        set_locale(&pool, &uid, "pl").await.unwrap();
        assert_eq!(get_locale(&pool, &uid).await.unwrap(), "pl");
    }
}
