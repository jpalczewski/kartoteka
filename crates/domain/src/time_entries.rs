use crate::DomainError;
use kartoteka_db as db;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    pub item_id: Option<String>,
    pub user_id: String,
    pub description: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration: Option<i32>,
    pub source: String,
    pub mode: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTimeSummary {
    pub total_seconds: i64,
    pub entry_count: i64,
}

fn row_to_entry(row: db::types::TimeEntryRow) -> TimeEntry {
    TimeEntry {
        id: row.id,
        item_id: row.item_id,
        user_id: row.user_id,
        description: row.description,
        started_at: row.started_at,
        ended_at: row.ended_at,
        duration: row.duration,
        source: row.source,
        mode: row.mode,
        created_at: row.created_at,
    }
}

fn now_str() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

// ── Orchestration ─────────────────────────────────────────────────────────────

/// Start a stopwatch timer. Auto-stops any currently running timer for this user.
#[tracing::instrument(skip(pool))]
pub async fn start(
    pool: &SqlitePool,
    user_id: &str,
    item_id: Option<&str>,
) -> Result<TimeEntry, DomainError> {
    // Auto-stop any running timer
    if let Some(running) = db::time_entries::get_running(pool, user_id).await? {
        let now = chrono::Utc::now();
        let now_s = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let started =
            chrono::NaiveDateTime::parse_from_str(&running.started_at, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| now.naive_utc());
        let duration = (now.naive_utc() - started).num_seconds().max(0) as i32;
        db::time_entries::stop(pool, &running.id, user_id, &now_s, duration).await?;
    }
    let id = Uuid::new_v4().to_string();
    let started_at = now_str();
    let row = db::time_entries::insert(
        pool,
        db::time_entries::InsertTimeEntryInput {
            id: &id,
            item_id,
            user_id,
            description: None,
            started_at: &started_at,
            source: "timer",
            mode: Some("stopwatch"),
            ended_at: None,
            duration: None,
        },
    )
    .await?;
    Ok(row_to_entry(row))
}

/// Stop the running timer. Returns Ok(None) if no timer is running (not an error).
#[tracing::instrument(skip(pool))]
pub async fn stop(pool: &SqlitePool, user_id: &str) -> Result<Option<TimeEntry>, DomainError> {
    let entry = match db::time_entries::get_running(pool, user_id).await? {
        Some(e) => e,
        None => return Ok(None),
    };
    let now = chrono::Utc::now();
    let now_s = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let started = chrono::NaiveDateTime::parse_from_str(&entry.started_at, "%Y-%m-%d %H:%M:%S")
        .unwrap_or_else(|_| now.naive_utc());
    let duration = (now.naive_utc() - started).num_seconds().max(0) as i32;
    db::time_entries::stop(pool, &entry.id, user_id, &now_s, duration).await?;
    let updated = db::time_entries::get_one(pool, &entry.id, user_id)
        .await?
        .ok_or_else(|| DomainError::Internal("entry disappeared after stop".into()))?;
    Ok(Some(row_to_entry(updated)))
}

/// Log a manual entry. Validates: ended_at > started_at, duration ≤ 86400s.
#[tracing::instrument(skip(pool))]
pub async fn log_manual(
    pool: &SqlitePool,
    user_id: &str,
    item_id: Option<&str>,
    started_at: &str,
    ended_at: &str,
    description: Option<&str>,
) -> Result<TimeEntry, DomainError> {
    let started = chrono::NaiveDateTime::parse_from_str(started_at, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| DomainError::Validation("invalid_time_format"))?;
    let ended = chrono::NaiveDateTime::parse_from_str(ended_at, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| DomainError::Validation("invalid_time_format"))?;
    if ended <= started {
        return Err(DomainError::Validation("invalid_time_range"));
    }
    let duration_secs = (ended - started).num_seconds() as i32;
    if duration_secs > 86400 {
        return Err(DomainError::Validation("duration_too_long"));
    }
    let id = Uuid::new_v4().to_string();
    let row = db::time_entries::insert(
        pool,
        db::time_entries::InsertTimeEntryInput {
            id: &id,
            item_id,
            user_id,
            description,
            started_at,
            source: "manual",
            mode: None,
            ended_at: Some(ended_at),
            duration: Some(duration_secs),
        },
    )
    .await?;
    Ok(row_to_entry(row))
}

/// Assign an unassigned entry to an item. Checks item ownership.
#[tracing::instrument(skip(pool))]
pub async fn assign(
    pool: &SqlitePool,
    user_id: &str,
    entry_id: &str,
    item_id: &str,
) -> Result<TimeEntry, DomainError> {
    if db::items::get_one(pool, item_id, user_id).await?.is_none() {
        return Err(DomainError::Forbidden);
    }
    let found = db::time_entries::assign(pool, entry_id, item_id, user_id).await?;
    if !found {
        return Err(DomainError::NotFound("time_entry"));
    }
    let row = db::time_entries::get_one(pool, entry_id, user_id)
        .await?
        .ok_or_else(|| DomainError::Internal("entry disappeared after assign".into()))?;
    Ok(row_to_entry(row))
}

/// Delete a time entry. Returns Forbidden if not owned by user or not found.
#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, entry_id: &str) -> Result<(), DomainError> {
    let deleted = db::time_entries::delete(pool, entry_id, user_id).await?;
    if !deleted {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

pub async fn list_for_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
) -> Result<Vec<TimeEntry>, DomainError> {
    if db::items::get_one(pool, item_id, user_id).await?.is_none() {
        return Err(DomainError::Forbidden);
    }
    let rows = db::time_entries::list_for_item(pool, item_id, user_id).await?;
    Ok(rows.into_iter().map(row_to_entry).collect())
}

pub async fn list_inbox(pool: &SqlitePool, user_id: &str) -> Result<Vec<TimeEntry>, DomainError> {
    let rows = db::time_entries::list_inbox(pool, user_id).await?;
    Ok(rows.into_iter().map(row_to_entry).collect())
}

pub async fn list_all_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<TimeEntry>, DomainError> {
    let rows = db::time_entries::list_all_for_user(pool, user_id).await?;
    Ok(rows.into_iter().map(row_to_entry).collect())
}

pub async fn summary_for_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
) -> Result<ItemTimeSummary, DomainError> {
    let (total_seconds, entry_count) =
        db::time_entries::summary_for_item(pool, item_id, user_id).await?;
    Ok(ItemTimeSummary {
        total_seconds,
        entry_count,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};
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

    #[tokio::test]
    async fn start_creates_running_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let entry = start(&pool, &uid, None).await.unwrap();
        assert_eq!(entry.source, "timer");
        assert_eq!(entry.mode.as_deref(), Some("stopwatch"));
        assert!(entry.ended_at.is_none());
        assert!(entry.item_id.is_none());
    }

    #[tokio::test]
    async fn start_with_item_assigns_item_id() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_id = insert_test_item(&pool, &list_id).await;

        let entry = start(&pool, &uid, Some(&item_id)).await.unwrap();
        assert_eq!(entry.item_id.as_deref(), Some(item_id.as_str()));
    }

    #[tokio::test]
    async fn start_auto_stops_previous_timer() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let first = start(&pool, &uid, None).await.unwrap();
        let _second = start(&pool, &uid, None).await.unwrap();

        // First entry should now be stopped
        let stopped = kartoteka_db::time_entries::get_one(&pool, &first.id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert!(stopped.ended_at.is_some());
        assert!(stopped.duration.is_some());

        // Only one running entry should exist
        let running = kartoteka_db::time_entries::get_running(&pool, &uid)
            .await
            .unwrap();
        assert!(running.is_some());
    }

    #[tokio::test]
    async fn stop_returns_none_when_no_timer() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let result = stop(&pool, &uid).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn stop_completes_running_timer() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        start(&pool, &uid, None).await.unwrap();
        let stopped = stop(&pool, &uid).await.unwrap();
        assert!(stopped.is_some());
        let stopped = stopped.unwrap();
        assert!(stopped.ended_at.is_some());
        assert!(stopped.duration.unwrap() >= 0);

        // No more running timer
        assert!(
            kartoteka_db::time_entries::get_running(&pool, &uid)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn log_manual_rejects_invalid_range() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let result = log_manual(
            &pool,
            &uid,
            None,
            "2026-01-01 12:00:00", // started_at
            "2026-01-01 11:00:00", // ended_at BEFORE started_at
            None,
        )
        .await;
        assert!(matches!(
            result,
            Err(DomainError::Validation("invalid_time_range"))
        ));
    }

    #[tokio::test]
    async fn log_manual_rejects_too_long() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let result = log_manual(
            &pool,
            &uid,
            None,
            "2026-01-01 00:00:00",
            "2026-01-02 01:00:00", // 25h — too long
            None,
        )
        .await;
        assert!(matches!(
            result,
            Err(DomainError::Validation("duration_too_long"))
        ));
    }

    #[tokio::test]
    async fn log_manual_valid_entry_has_source_manual() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let entry = log_manual(
            &pool,
            &uid,
            None,
            "2026-01-01 10:00:00",
            "2026-01-01 11:00:00", // 3600s
            Some("test session"),
        )
        .await
        .unwrap();

        assert_eq!(entry.source, "manual");
        assert_eq!(entry.duration, Some(3600));
        assert_eq!(entry.description.as_deref(), Some("test session"));
    }

    #[tokio::test]
    async fn assign_forbidden_for_other_users_item() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let attacker = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &owner).await;
        let item_id = insert_test_item(&pool, &list_id).await;

        // attacker creates their own entry
        let entry = start(&pool, &attacker, None).await.unwrap();

        // attacker tries to assign to owner's item
        let result = assign(&pool, &attacker, &entry.id, &item_id).await;
        assert!(matches!(result, Err(DomainError::Forbidden)));
    }

    #[tokio::test]
    async fn summary_for_item_sums_completed_entries() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_id = insert_test_item(&pool, &list_id).await;

        log_manual(
            &pool,
            &uid,
            Some(&item_id),
            "2026-01-01 10:00:00",
            "2026-01-01 10:01:00",
            None,
        )
        .await
        .unwrap(); // 60s
        log_manual(
            &pool,
            &uid,
            Some(&item_id),
            "2026-01-01 11:00:00",
            "2026-01-01 11:02:00",
            None,
        )
        .await
        .unwrap(); // 120s
        // Running entry — should NOT be counted
        start(&pool, &uid, Some(&item_id)).await.unwrap();

        let summary = summary_for_item(&pool, &uid, &item_id).await.unwrap();
        assert_eq!(summary.total_seconds, 180);
        assert_eq!(summary.entry_count, 2);
    }
}
