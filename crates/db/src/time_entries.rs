use crate::{DbError, types::TimeEntryRow};
use sqlx::SqlitePool;

// ── Read ──────────────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(
        "SELECT id, item_id, user_id, description, started_at, ended_at, duration, source, mode, created_at \
         FROM time_entries WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_running(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(
        "SELECT id, item_id, user_id, description, started_at, ended_at, duration, source, mode, created_at \
         FROM time_entries WHERE user_id = ? AND ended_at IS NULL LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_for_item(
    pool: &SqlitePool,
    item_id: &str,
    user_id: &str,
) -> Result<Vec<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(
        "SELECT id, item_id, user_id, description, started_at, ended_at, duration, source, mode, created_at \
         FROM time_entries WHERE item_id = ? AND user_id = ? ORDER BY created_at DESC",
    )
    .bind(item_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_inbox(pool: &SqlitePool, user_id: &str) -> Result<Vec<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(
        "SELECT id, item_id, user_id, description, started_at, ended_at, duration, source, mode, created_at \
         FROM time_entries WHERE user_id = ? AND item_id IS NULL ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_all_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(
        "SELECT id, item_id, user_id, description, started_at, ended_at, duration, source, mode, created_at \
         FROM time_entries WHERE user_id = ? ORDER BY created_at DESC LIMIT 200",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Returns (total_seconds, entry_count) for completed entries on an item.
#[tracing::instrument(skip(pool))]
pub async fn summary_for_item(
    pool: &SqlitePool,
    item_id: &str,
    user_id: &str,
) -> Result<(i64, i64), DbError> {
    let row: (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(duration), 0), COUNT(*) FROM time_entries \
         WHERE item_id = ? AND user_id = ? AND ended_at IS NOT NULL",
    )
    .bind(item_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row)
}

// ── Write ─────────────────────────────────────────────────────────────────────

pub struct InsertTimeEntryInput<'a> {
    pub id: &'a str,
    pub item_id: Option<&'a str>,
    pub user_id: &'a str,
    pub description: Option<&'a str>,
    pub started_at: &'a str,
    pub source: &'a str,
    pub mode: Option<&'a str>,
    pub ended_at: Option<&'a str>,
    pub duration: Option<i32>,
}

#[tracing::instrument(skip(pool, input), fields(item_id = ?input.item_id, source = %input.source))]
pub async fn insert(
    pool: &SqlitePool,
    input: InsertTimeEntryInput<'_>,
) -> Result<TimeEntryRow, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(
        "INSERT INTO time_entries \
           (id, item_id, user_id, description, started_at, source, mode, ended_at, duration) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
         RETURNING id, item_id, user_id, description, started_at, ended_at, duration, source, mode, created_at",
    )
    .bind(input.id)
    .bind(input.item_id)
    .bind(input.user_id)
    .bind(input.description)
    .bind(input.started_at)
    .bind(input.source)
    .bind(input.mode)
    .bind(input.ended_at)
    .bind(input.duration)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn stop(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    ended_at: &str,
    duration: i32,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE time_entries SET ended_at = ?, duration = ? WHERE id = ? AND user_id = ?",
    )
    .bind(ended_at)
    .bind(duration)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn assign(
    pool: &SqlitePool,
    id: &str,
    item_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query("UPDATE time_entries SET item_id = ? WHERE id = ? AND user_id = ?")
        .bind(item_id)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM time_entries WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
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

    async fn insert_test_item(pool: &SqlitePool, list_id: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'Test Item')")
            .bind(&id)
            .bind(list_id)
            .execute(pool)
            .await
            .expect("insert_test_item");
        id
    }

    async fn make_entry(
        pool: &SqlitePool,
        user_id: &str,
        item_id: Option<&str>,
        ended_at: Option<&str>,
        duration: Option<i32>,
    ) -> TimeEntryRow {
        insert(
            pool,
            InsertTimeEntryInput {
                id: &Uuid::new_v4().to_string(),
                item_id,
                user_id,
                description: None,
                started_at: "2026-01-01 10:00:00",
                source: "timer",
                mode: Some("stopwatch"),
                ended_at,
                duration,
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn insert_returns_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_id = insert_test_item(&pool, &list_id).await;

        let entry = make_entry(&pool, &uid, Some(&item_id), None, None).await;
        assert_eq!(entry.user_id, uid);
        assert_eq!(entry.item_id.as_deref(), Some(item_id.as_str()));
        assert_eq!(entry.source, "timer");
        assert!(entry.ended_at.is_none());
        assert!(entry.duration.is_none());
    }

    #[tokio::test]
    async fn get_running_returns_active_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        // No running entry yet
        assert!(get_running(&pool, &uid).await.unwrap().is_none());

        // Insert a running entry (ended_at = None)
        let entry = make_entry(&pool, &uid, None, None, None).await;
        let running = get_running(&pool, &uid).await.unwrap();
        assert!(running.is_some());
        assert_eq!(running.unwrap().id, entry.id);

        // Insert a completed entry — should not change result
        make_entry(&pool, &uid, None, Some("2026-01-01 11:00:00"), Some(3600)).await;
        let running = get_running(&pool, &uid).await.unwrap();
        assert!(running.is_some()); // still the first running one
    }

    #[tokio::test]
    async fn stop_sets_ended_at_and_duration() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let entry = make_entry(&pool, &uid, None, None, None).await;

        let affected = stop(&pool, &entry.id, &uid, "2026-01-01 11:00:00", 3600)
            .await
            .unwrap();
        assert!(affected);

        let updated = get_one(&pool, &entry.id, &uid).await.unwrap().unwrap();
        assert_eq!(updated.ended_at.as_deref(), Some("2026-01-01 11:00:00"));
        assert_eq!(updated.duration, Some(3600));

        // Running timer should now be gone
        assert!(get_running(&pool, &uid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_inbox_returns_only_unassigned() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_id = insert_test_item(&pool, &list_id).await;

        make_entry(&pool, &uid, Some(&item_id), None, None).await; // assigned
        make_entry(&pool, &uid, None, None, None).await; // unassigned

        let inbox = list_inbox(&pool, &uid).await.unwrap();
        assert_eq!(inbox.len(), 1);
        assert!(inbox[0].item_id.is_none());
    }

    #[tokio::test]
    async fn list_for_item_returns_only_that_item() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_a = insert_test_item(&pool, &list_id).await;
        let item_b = insert_test_item(&pool, &list_id).await;

        make_entry(&pool, &uid, Some(&item_a), None, None).await;
        make_entry(&pool, &uid, Some(&item_b), None, None).await;

        let rows = list_for_item(&pool, &item_a, &uid).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].item_id.as_deref(), Some(item_a.as_str()));
    }

    #[tokio::test]
    async fn assign_changes_item_id() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_id = insert_test_item(&pool, &list_id).await;

        let entry = make_entry(&pool, &uid, None, None, None).await;
        assert!(entry.item_id.is_none());

        let affected = assign(&pool, &entry.id, &item_id, &uid).await.unwrap();
        assert!(affected);

        let updated = get_one(&pool, &entry.id, &uid).await.unwrap().unwrap();
        assert_eq!(updated.item_id.as_deref(), Some(item_id.as_str()));
    }

    #[tokio::test]
    async fn delete_removes_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let entry = make_entry(&pool, &uid, None, None, None).await;

        let deleted = delete(&pool, &entry.id, &uid).await.unwrap();
        assert!(deleted);
        assert!(get_one(&pool, &entry.id, &uid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_wrong_user_returns_false() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let entry = make_entry(&pool, &uid, None, None, None).await;

        let deleted = delete(&pool, &entry.id, &other).await.unwrap();
        assert!(!deleted);
        assert!(get_one(&pool, &entry.id, &uid).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn summary_for_item_sums_completed_entries() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_id = insert_test_item(&pool, &list_id).await;

        // Two completed entries (60s + 120s = 180s)
        make_entry(
            &pool,
            &uid,
            Some(&item_id),
            Some("2026-01-01 10:01:00"),
            Some(60),
        )
        .await;
        make_entry(
            &pool,
            &uid,
            Some(&item_id),
            Some("2026-01-01 10:03:00"),
            Some(120),
        )
        .await;
        // One running entry (no duration) — should NOT be counted
        make_entry(&pool, &uid, Some(&item_id), None, None).await;

        let (total, count) = summary_for_item(&pool, &item_id, &uid).await.unwrap();
        assert_eq!(total, 180);
        assert_eq!(count, 2);
    }
}
