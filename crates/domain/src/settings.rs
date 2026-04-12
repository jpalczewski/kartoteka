use crate::DomainError;
use kartoteka_db as db;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSetting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

fn row_to_setting(row: db::types::UserSettingRow) -> UserSetting {
    UserSetting {
        key: row.key,
        value: row.value,
        updated_at: row.updated_at,
    }
}

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<UserSetting>, DomainError> {
    Ok(db::settings::list_all(pool, user_id)
        .await?
        .into_iter()
        .map(row_to_setting)
        .collect())
}

#[tracing::instrument(skip(pool))]
pub async fn get(
    pool: &SqlitePool,
    user_id: &str,
    key: &str,
) -> Result<Option<UserSetting>, DomainError> {
    Ok(db::settings::get(pool, user_id, key)
        .await?
        .map(row_to_setting))
}

#[tracing::instrument(skip(pool))]
pub async fn set(
    pool: &SqlitePool,
    user_id: &str,
    key: &str,
    value: &str,
) -> Result<UserSetting, DomainError> {
    if key.is_empty() {
        return Err(DomainError::Validation("setting_key_empty"));
    }
    Ok(row_to_setting(
        db::settings::set(pool, user_id, key, value).await?,
    ))
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, key: &str) -> Result<bool, DomainError> {
    Ok(db::settings::delete(pool, user_id, key).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn set_and_list() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        set(&pool, &uid, "theme", "dark").await.unwrap();
        set(&pool, &uid, "density", "compact").await.unwrap();

        let settings = list_all(&pool, &uid).await.unwrap();
        assert_eq!(settings.len(), 2);
        assert_eq!(settings[0].key, "density");
        assert_eq!(settings[1].key, "theme");
    }

    #[tokio::test]
    async fn set_empty_key_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let result = set(&pool, &uid, "", "value").await;
        assert!(matches!(result.unwrap_err(), DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        assert!(get(&pool, &uid, "missing").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_returns_false() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        assert!(!delete(&pool, &uid, "missing").await.unwrap());
    }

    #[tokio::test]
    async fn set_and_delete() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        set(&pool, &uid, "key", "val").await.unwrap();
        assert!(delete(&pool, &uid, "key").await.unwrap());
        assert!(get(&pool, &uid, "key").await.unwrap().is_none());
    }
}
