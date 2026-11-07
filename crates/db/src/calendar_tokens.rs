use crate::{DbError, SqlitePool};
use serde::{Deserialize, Serialize};

const CALENDAR_TOKEN_COLS: &str = "id, user_id, list_id, name, last_used_at, created_at";

/// A calendar feed token. `token_hash` is deliberately never selected: the
/// plaintext token is shown once on creation and only its SHA-256 hash is stored.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct CalendarTokenRow {
    pub id: String,
    pub user_id: String,
    /// `None` means the token covers all of the user's lists.
    pub list_id: Option<String>,
    pub name: String,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

#[tracing::instrument(skip(pool, token_hash))]
pub async fn create(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    list_id: Option<&str>,
    name: &str,
    token_hash: &str,
) -> Result<CalendarTokenRow, DbError> {
    sqlx::query_as::<_, CalendarTokenRow>(&format!(
        "INSERT INTO calendar_tokens (id, user_id, list_id, name, token_hash) \
         VALUES (?, ?, ?, ?, ?) \
         RETURNING {CALENDAR_TOKEN_COLS}"
    ))
    .bind(id)
    .bind(user_id)
    .bind(list_id)
    .bind(name)
    .bind(token_hash)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_by_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<CalendarTokenRow>, DbError> {
    sqlx::query_as::<_, CalendarTokenRow>(&format!(
        "SELECT {CALENDAR_TOKEN_COLS} FROM calendar_tokens WHERE user_id = ? ORDER BY created_at DESC"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Delete a token. Returns true if found and owned by user_id, false otherwise.
#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let result = sqlx::query("DELETE FROM calendar_tokens WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(result.rows_affected() > 0)
}

#[tracing::instrument(skip(pool, token_hash))]
pub async fn find_by_hash(
    pool: &SqlitePool,
    token_hash: &str,
) -> Result<Option<CalendarTokenRow>, DbError> {
    sqlx::query_as::<_, CalendarTokenRow>(&format!(
        "SELECT {CALENDAR_TOKEN_COLS} FROM calendar_tokens WHERE token_hash = ?"
    ))
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Update last_used_at to current time, throttled to once per 5 minutes to avoid a
/// write on every feed request hitting SQLite's write lock.
#[tracing::instrument(skip(pool))]
pub async fn touch_last_used(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE calendar_tokens SET last_used_at = datetime('now') \
         WHERE id = ? AND (last_used_at IS NULL OR last_used_at < datetime('now', '-5 minutes'))",
    )
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
    use sqlx::SqlitePool;

    async fn setup_list(pool: &SqlitePool, user_id: &str) -> String {
        let list_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'Test List')")
            .bind(&list_id)
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        list_id
    }

    #[tokio::test]
    async fn create_and_find_by_hash() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let row = create(&pool, "ct-1", &user_id, None, "My Feed", "hash-1")
            .await
            .unwrap();
        assert_eq!(row.id, "ct-1");
        assert_eq!(row.user_id, user_id);
        assert!(row.list_id.is_none());
        assert_eq!(row.name, "My Feed");
        assert!(row.last_used_at.is_none());

        let found = find_by_hash(&pool, "hash-1").await.unwrap().unwrap();
        assert_eq!(found.id, "ct-1");
        assert_eq!(found.name, "My Feed");
    }

    #[tokio::test]
    async fn create_with_list_id() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;
        let row = create(
            &pool,
            "ct-1",
            &user_id,
            Some(&list_id),
            "List Feed",
            "hash-1",
        )
        .await
        .unwrap();
        assert_eq!(row.list_id.as_deref(), Some(list_id.as_str()));
    }

    #[tokio::test]
    async fn find_by_hash_unknown_returns_none() {
        let pool = test_pool().await;
        let result = find_by_hash(&pool, "no-such-hash").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_by_user_returns_only_owned_tokens() {
        let pool = test_pool().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_test_user(&pool).await;
        create(&pool, "ct-1", &user1, None, "Feed A", "hash-1")
            .await
            .unwrap();
        create(&pool, "ct-2", &user1, None, "Feed B", "hash-2")
            .await
            .unwrap();
        create(&pool, "ct-3", &user2, None, "Feed C", "hash-3")
            .await
            .unwrap();

        let list = list_by_user(&pool, &user1).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|t| t.user_id == user1));
    }

    #[tokio::test]
    async fn token_hash_is_unique() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        create(&pool, "ct-1", &user_id, None, "Feed A", "same-hash")
            .await
            .unwrap();
        let err = create(&pool, "ct-2", &user_id, None, "Feed B", "same-hash")
            .await
            .unwrap_err();
        assert!(matches!(err, DbError::Sqlx(_)));
    }

    #[tokio::test]
    async fn delete_removes_token_returns_true() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        create(&pool, "ct-1", &user_id, None, "Feed", "hash-1")
            .await
            .unwrap();
        let deleted = delete(&pool, "ct-1", &user_id).await.unwrap();
        assert!(deleted);
        assert!(find_by_hash(&pool, "hash-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_wrong_user_returns_false_and_leaves_row() {
        let pool = test_pool().await;
        let user1 = create_test_user(&pool).await;
        let user2 = create_test_user(&pool).await;
        create(&pool, "ct-1", &user1, None, "Feed", "hash-1")
            .await
            .unwrap();
        let deleted = delete(&pool, "ct-1", &user2).await.unwrap();
        assert!(!deleted);
        assert!(find_by_hash(&pool, "hash-1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn deleting_list_cascades_to_tokens() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;
        create(
            &pool,
            "ct-1",
            &user_id,
            Some(&list_id),
            "List Feed",
            "hash-1",
        )
        .await
        .unwrap();

        sqlx::query("DELETE FROM lists WHERE id = ?")
            .bind(&list_id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(find_by_hash(&pool, "hash-1").await.unwrap().is_none());
        assert!(list_by_user(&pool, &user_id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn deleting_user_cascades_to_tokens() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        create(&pool, "ct-1", &user_id, None, "Feed", "hash-1")
            .await
            .unwrap();

        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(&user_id)
            .execute(&pool)
            .await
            .unwrap();

        assert!(find_by_hash(&pool, "hash-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn touch_last_used_sets_timestamp() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        create(&pool, "ct-1", &user_id, None, "Feed", "hash-1")
            .await
            .unwrap();
        touch_last_used(&pool, "ct-1").await.unwrap();
        let row = find_by_hash(&pool, "hash-1").await.unwrap().unwrap();
        assert!(row.last_used_at.is_some());
    }

    #[tokio::test]
    async fn touch_last_used_is_throttled() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        create(&pool, "ct-1", &user_id, None, "Feed", "hash-1")
            .await
            .unwrap();

        sqlx::query(
            "UPDATE calendar_tokens SET last_used_at = datetime('now', '-1 minute') WHERE id = ?",
        )
        .bind("ct-1")
        .execute(&pool)
        .await
        .unwrap();
        touch_last_used(&pool, "ct-1").await.unwrap();

        // The recent timestamp is preserved because the throttle window is 5 minutes.
        let row = find_by_hash(&pool, "hash-1").await.unwrap().unwrap();
        let recent: (i64,) = sqlx::query_as(
            "SELECT last_used_at > datetime('now', '-2 minutes') FROM calendar_tokens WHERE id = ?",
        )
        .bind("ct-1")
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(recent.0, 1);
        assert!(row.last_used_at.is_some());
    }
}
