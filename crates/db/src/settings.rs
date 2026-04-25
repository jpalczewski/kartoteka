use crate::{DbError, types::UserSettingRow};
use sqlx::SqlitePool;

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<UserSettingRow>, DbError> {
    sqlx::query_as::<_, UserSettingRow>(
        "SELECT user_id, key, value, updated_at \
         FROM user_settings WHERE user_id = ? ORDER BY key",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get(
    pool: &SqlitePool,
    user_id: &str,
    key: &str,
) -> Result<Option<UserSettingRow>, DbError> {
    sqlx::query_as::<_, UserSettingRow>(
        "SELECT user_id, key, value, updated_at \
         FROM user_settings WHERE user_id = ? AND key = ?",
    )
    .bind(user_id)
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Upsert: insert or overwrite. Returns the stored row.
#[tracing::instrument(skip(pool))]
pub async fn set(
    pool: &SqlitePool,
    user_id: &str,
    key: &str,
    value: &str,
) -> Result<UserSettingRow, DbError> {
    sqlx::query_as::<_, UserSettingRow>(
        "INSERT INTO user_settings (user_id, key, value, updated_at) \
         VALUES (?, ?, ?, datetime('now')) \
         ON CONFLICT (user_id, key) \
         DO UPDATE SET value = excluded.value, updated_at = datetime('now') \
         RETURNING user_id, key, value, updated_at",
    )
    .bind(user_id)
    .bind(key)
    .bind(value)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, key: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM user_settings WHERE user_id = ? AND key = ?")
        .bind(user_id)
        .bind(key)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn set_and_get() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        set(&pool, &uid, "theme", "dark").await.unwrap();
        let row = get(&pool, &uid, "theme").await.unwrap().unwrap();
        assert_eq!(row.key, "theme");
        assert_eq!(row.value, "dark");
    }

    #[tokio::test]
    async fn set_overwrites_existing_value() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        set(&pool, &uid, "theme", "dark").await.unwrap();
        set(&pool, &uid, "theme", "light").await.unwrap();
        let row = get(&pool, &uid, "theme").await.unwrap().unwrap();
        assert_eq!(row.value, "light");
    }

    #[tokio::test]
    async fn list_all_returns_all_keys_sorted() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        set(&pool, &uid, "zoom", "1.5").await.unwrap();
        set(&pool, &uid, "density", "compact").await.unwrap();

        let rows = list_all(&pool, &uid).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].key, "density");
        assert_eq!(rows[1].key, "zoom");
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let row = get(&pool, &uid, "nonexistent").await.unwrap();
        assert!(row.is_none());
    }

    #[tokio::test]
    async fn delete_removes_key() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        set(&pool, &uid, "theme", "dark").await.unwrap();
        assert!(delete(&pool, &uid, "theme").await.unwrap());
        assert!(get(&pool, &uid, "theme").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_key_returns_false() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        assert!(!delete(&pool, &uid, "nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn list_all_isolation_between_users() {
        let pool = test_pool().await;
        let uid1 = create_test_user(&pool).await;
        let uid2 = create_test_user(&pool).await;

        set(&pool, &uid1, "key", "val1").await.unwrap();
        set(&pool, &uid2, "key", "val2").await.unwrap();

        let rows1 = list_all(&pool, &uid1).await.unwrap();
        assert_eq!(rows1.len(), 1);
        assert_eq!(rows1[0].value, "val1");
    }
}
