use crate::DomainError;
use kartoteka_db as db;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
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

/// Tracked time for one entity (item, list subtree or container subtree).
///
/// `seconds` includes the live elapsed time of a running entry, so a project
/// total is already correct before the timer is stopped; `running_since` is the
/// `started_at` of that entry when there is one.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeTotal {
    pub seconds: i64,
    /// `started_at` of the running entry counted into `seconds`, if any.
    pub running_since: Option<String>,
}

/// How a running timer was started. Breaks/pomodoro cycles are frontend-only:
/// the backend just records which mode a work session belongs to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerMode {
    #[default]
    Stopwatch,
    Pomodoro,
}

impl TimerMode {
    /// Value stored in `time_entries.mode`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stopwatch => "stopwatch",
            Self::Pomodoro => "pomodoro",
        }
    }

    /// Parse a caller-supplied mode (case-insensitive).
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "stopwatch" => Ok(Self::Stopwatch),
            "pomodoro" => Ok(Self::Pomodoro),
            _ => Err(DomainError::Validation("invalid_timer_mode")),
        }
    }
}

/// Format a duration as `h:mm` (e.g. `125` seconds -> `0:02`, `7500` -> `2:05`).
pub fn format_h_mm(seconds: i64) -> String {
    let secs = seconds.max(0);
    format!("{}:{:02}", secs / 3600, (secs % 3600) / 60)
}

fn total_to_domain(row: db::time_entries::TimeTotalRow) -> TimeTotal {
    TimeTotal {
        seconds: row.seconds,
        running_since: row.running_since,
    }
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
    start_with_mode(pool, user_id, item_id, TimerMode::default()).await
}

/// Start a timer in the given mode (`stopwatch` or `pomodoro`).
/// Auto-stops any currently running timer for this user.
#[tracing::instrument(skip(pool))]
pub async fn start_with_mode(
    pool: &SqlitePool,
    user_id: &str,
    item_id: Option<&str>,
    mode: TimerMode,
) -> Result<TimeEntry, DomainError> {
    if let Some(iid) = item_id {
        let row = db::items::get_one(pool, iid, user_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        let features = db::lists::get_feature_names(pool, &row.list_id).await?;
        if !features.iter().any(|f| f == "time_tracking") {
            return Err(DomainError::FeatureRequired("time_tracking"));
        }
    }
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
            mode: Some(mode.as_str()),
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
    if let Some(iid) = item_id {
        let row = db::items::get_one(pool, iid, user_id)
            .await?
            .ok_or(DomainError::Forbidden)?;
        let features = db::lists::get_feature_names(pool, &row.list_id).await?;
        if !features.iter().any(|f| f == "time_tracking") {
            return Err(DomainError::FeatureRequired("time_tracking"));
        }
    }
    let started = chrono::NaiveDateTime::parse_from_str(started_at, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| DomainError::Validation("invalid_time_format"))?;
    let ended = chrono::NaiveDateTime::parse_from_str(ended_at, "%Y-%m-%d %H:%M:%S")
        .map_err(|_| DomainError::Validation("invalid_time_format"))?;
    if ended <= started {
        return Err(DomainError::Validation("invalid_time_range"));
    }
    let duration_secs_i64 = (ended - started).num_seconds();
    if duration_secs_i64 > 86400 {
        return Err(DomainError::Validation("duration_too_long"));
    }
    let duration_secs = duration_secs_i64 as i32;
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

/// Assign an unassigned entry to an item. Checks item ownership and that the
/// item's list has the `time_tracking` feature, exactly like [`start`]: the
/// entry would otherwise land on an item whose time section is hidden.
#[tracing::instrument(skip(pool))]
pub async fn assign(
    pool: &SqlitePool,
    user_id: &str,
    entry_id: &str,
    item_id: &str,
) -> Result<TimeEntry, DomainError> {
    let item = db::items::get_one(pool, item_id, user_id)
        .await?
        .ok_or(DomainError::Forbidden)?;
    let features = db::lists::get_feature_names(pool, &item.list_id).await?;
    if !features.iter().any(|f| f == "time_tracking") {
        return Err(DomainError::FeatureRequired("time_tracking"));
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

#[tracing::instrument(skip(pool))]
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

#[tracing::instrument(skip(pool))]
pub async fn list_inbox(pool: &SqlitePool, user_id: &str) -> Result<Vec<TimeEntry>, DomainError> {
    let rows = db::time_entries::list_inbox(pool, user_id).await?;
    Ok(rows.into_iter().map(row_to_entry).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn list_all_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<TimeEntry>, DomainError> {
    let rows = db::time_entries::list_all_for_user(pool, user_id).await?;
    Ok(rows.into_iter().map(row_to_entry).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn summary_for_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
) -> Result<ItemTimeSummary, DomainError> {
    if db::items::get_one(pool, item_id, user_id).await?.is_none() {
        return Err(DomainError::Forbidden);
    }
    let (total_seconds, entry_count) =
        db::time_entries::summary_for_item(pool, item_id, user_id).await?;
    Ok(ItemTimeSummary {
        total_seconds,
        entry_count,
    })
}

#[tracing::instrument(skip(pool))]
pub async fn get_active(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<TimeEntry>, DomainError> {
    let row = db::time_entries::get_running(pool, user_id).await?;
    Ok(row.map(row_to_entry))
}

// ── Aggregation (recursive roll-up) ───────────────────────────────────────────
//
// Semantics: a total is every completed entry in scope plus the live elapsed time
// of a running entry in scope. Inbox entries (`item_id IS NULL`) never belong to
// an item, list or container, so they count nowhere here.

/// Total tracked time for one item.
#[tracing::instrument(skip(pool))]
pub async fn total_for_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
) -> Result<TimeTotal, DomainError> {
    if db::items::get_one(pool, item_id, user_id).await?.is_none() {
        return Err(DomainError::Forbidden);
    }
    Ok(total_to_domain(
        db::time_entries::item_total(pool, user_id, item_id).await?,
    ))
}

/// Total tracked time for a list, **including every sublist at any depth**.
#[tracing::instrument(skip(pool))]
pub async fn total_for_list(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
) -> Result<TimeTotal, DomainError> {
    if db::lists::get_one(pool, list_id, user_id).await?.is_none() {
        return Err(DomainError::Forbidden);
    }
    Ok(total_to_domain(
        db::time_entries::list_total(pool, user_id, list_id).await?,
    ))
}

/// Total tracked time for a container, **including all sub-containers at any
/// depth, their lists and those lists' sublists**.
#[tracing::instrument(skip(pool))]
pub async fn total_for_container(
    pool: &SqlitePool,
    user_id: &str,
    container_id: &str,
) -> Result<TimeTotal, DomainError> {
    if db::containers::get_one(pool, container_id, user_id)
        .await?
        .is_none()
    {
        return Err(DomainError::Forbidden);
    }
    Ok(total_to_domain(
        db::time_entries::container_total(pool, user_id, container_id).await?,
    ))
}

/// Per-item totals in one query, for list pages (no N+1).
/// Every requested id is present in the map; unknown or foreign ids map to zero.
#[tracing::instrument(skip(pool))]
pub async fn totals_for_items(
    pool: &SqlitePool,
    user_id: &str,
    item_ids: &[String],
) -> Result<HashMap<String, TimeTotal>, DomainError> {
    let rows = db::time_entries::item_totals(pool, user_id, item_ids).await?;
    Ok(complete_map(item_ids, rows))
}

/// Per-list totals (each including its own sublists) in one query.
/// Every requested id is present in the map; unknown or foreign ids map to zero.
#[tracing::instrument(skip(pool))]
pub async fn totals_for_lists(
    pool: &SqlitePool,
    user_id: &str,
    list_ids: &[String],
) -> Result<HashMap<String, TimeTotal>, DomainError> {
    let rows = db::time_entries::list_totals(pool, user_id, list_ids).await?;
    Ok(complete_map(list_ids, rows))
}

/// Fill in a zero total for every requested id that has no entries.
fn complete_map(
    requested: &[String],
    found: HashMap<String, db::time_entries::TimeTotalRow>,
) -> HashMap<String, TimeTotal> {
    requested
        .iter()
        .map(|id| {
            let total = found
                .get(id)
                .map_or_else(TimeTotal::default, |r| TimeTotal {
                    seconds: r.seconds,
                    running_since: r.running_since.clone(),
                });
            (id.clone(), total)
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{
        create_test_item, create_test_list, create_test_user, test_pool,
    };

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
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        let features = serde_json::json!({"time_tracking": {}});
        let mut tx = pool.begin().await.unwrap();
        kartoteka_db::lists::set_features(&mut tx, &list_id, &features)
            .await
            .unwrap();
        tx.commit().await.unwrap();

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
        let list_id = create_test_list(&pool, &owner).await;
        let item_id = create_test_item(&pool, &list_id).await;

        // attacker creates their own entry
        let entry = start(&pool, &attacker, None).await.unwrap();

        // attacker tries to assign to owner's item
        let result = assign(&pool, &attacker, &entry.id, &item_id).await;
        assert!(matches!(result, Err(DomainError::Forbidden)));
    }

    /// Turn on `time_tracking` for a list, the same way the UI does.
    async fn enable_time_tracking(pool: &SqlitePool, list_id: &str) {
        let features = serde_json::json!({"time_tracking": {}});
        let mut tx = pool.begin().await.unwrap();
        kartoteka_db::lists::set_features(&mut tx, list_id, &features)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    #[tokio::test]
    async fn assign_blocked_when_time_tracking_feature_missing() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;
        let entry = start(&pool, &uid, None).await.unwrap();

        let err = assign(&pool, &uid, &entry.id, &item_id).await.unwrap_err();
        assert!(matches!(err, DomainError::FeatureRequired("time_tracking")));

        // The entry stays in the inbox, unassigned.
        let inbox = list_inbox(&pool, &uid).await.unwrap();
        assert_eq!(inbox.len(), 1);
        assert!(inbox[0].item_id.is_none());
    }

    #[tokio::test]
    async fn assign_moves_entry_out_of_inbox() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;
        enable_time_tracking(&pool, &list_id).await;

        let entry = start(&pool, &uid, None).await.unwrap();
        assert_eq!(list_inbox(&pool, &uid).await.unwrap().len(), 1);

        let assigned = assign(&pool, &uid, &entry.id, &item_id).await.unwrap();
        assert_eq!(assigned.item_id.as_deref(), Some(item_id.as_str()));
        assert!(list_inbox(&pool, &uid).await.unwrap().is_empty());

        let for_item = list_for_item(&pool, &uid, &item_id).await.unwrap();
        assert_eq!(for_item.len(), 1);
        assert_eq!(for_item[0].id, entry.id);
    }

    #[tokio::test]
    async fn assign_not_found_for_unknown_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;
        enable_time_tracking(&pool, &list_id).await;

        let err = assign(&pool, &uid, "missing-entry", &item_id)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound("time_entry")));
    }

    #[tokio::test]
    async fn summary_for_item_sums_completed_entries() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        let features = serde_json::json!({"time_tracking": {}});
        let mut tx = pool.begin().await.unwrap();
        kartoteka_db::lists::set_features(&mut tx, &list_id, &features)
            .await
            .unwrap();
        tx.commit().await.unwrap();

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

    #[tokio::test]
    async fn start_blocked_when_time_tracking_feature_missing() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        let err = start(&pool, &uid, Some(&item_id)).await.unwrap_err();
        assert!(matches!(err, DomainError::FeatureRequired("time_tracking")));
    }

    #[tokio::test]
    async fn start_without_item_id_always_allowed() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let entry = start(&pool, &uid, None).await.unwrap();
        assert!(entry.item_id.is_none());
    }

    #[tokio::test]
    async fn log_manual_blocked_when_time_tracking_feature_missing() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        let err = log_manual(
            &pool,
            &uid,
            Some(&item_id),
            "2026-01-01 10:00:00",
            "2026-01-01 11:00:00",
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::FeatureRequired("time_tracking")));
    }

    #[tokio::test]
    async fn start_allowed_when_time_tracking_feature_present() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        let features = serde_json::json!({"time_tracking": {}});
        let mut tx = pool.begin().await.unwrap();
        kartoteka_db::lists::set_features(&mut tx, &list_id, &features)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let entry = start(&pool, &uid, Some(&item_id)).await.unwrap();
        assert_eq!(entry.item_id.as_deref(), Some(item_id.as_str()));
    }

    // ── Timer modes ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn start_with_pomodoro_mode_stores_mode() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let entry = start_with_mode(&pool, &uid, None, TimerMode::Pomodoro)
            .await
            .unwrap();
        assert_eq!(entry.mode.as_deref(), Some("pomodoro"));

        let stopped = stop(&pool, &uid).await.unwrap().unwrap();
        assert_eq!(stopped.mode.as_deref(), Some("pomodoro"));
    }

    #[tokio::test]
    async fn start_defaults_to_stopwatch() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let entry = start(&pool, &uid, None).await.unwrap();
        assert_eq!(entry.mode.as_deref(), Some("stopwatch"));
    }

    #[test]
    fn timer_mode_parse_accepts_known_values() {
        assert_eq!(TimerMode::parse("stopwatch").unwrap(), TimerMode::Stopwatch);
        assert_eq!(TimerMode::parse("Pomodoro").unwrap(), TimerMode::Pomodoro);
        assert_eq!(TimerMode::default().as_str(), "stopwatch");
        assert!(matches!(
            TimerMode::parse("egg_timer"),
            Err(DomainError::Validation("invalid_timer_mode"))
        ));
    }

    #[test]
    fn format_h_mm_formats_durations() {
        assert_eq!(format_h_mm(0), "0:00");
        assert_eq!(format_h_mm(125), "0:02");
        assert_eq!(format_h_mm(7500), "2:05");
        assert_eq!(format_h_mm(-5), "0:00");
    }

    // ── Aggregation ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn total_for_item_includes_running_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;
        let features = serde_json::json!({"time_tracking": {}});
        let mut tx = pool.begin().await.unwrap();
        kartoteka_db::lists::set_features(&mut tx, &list_id, &features)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        log_manual(
            &pool,
            &uid,
            Some(&item_id),
            "2026-01-01 10:00:00",
            "2026-01-01 11:00:00",
            None,
        )
        .await
        .unwrap();

        let total = total_for_item(&pool, &uid, &item_id).await.unwrap();
        assert_eq!(total.seconds, 3600);
        assert!(total.running_since.is_none());

        start(&pool, &uid, Some(&item_id)).await.unwrap();
        let total = total_for_item(&pool, &uid, &item_id).await.unwrap();
        assert!(total.seconds >= 3600);
        assert!(total.running_since.is_some());
    }

    #[tokio::test]
    async fn total_for_item_forbidden_for_other_users_item() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let attacker = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &owner).await;
        let item_id = create_test_item(&pool, &list_id).await;

        let result = total_for_item(&pool, &attacker, &item_id).await;
        assert!(matches!(result, Err(DomainError::Forbidden)));
    }

    #[tokio::test]
    async fn totals_roll_up_lists_and_containers() {
        use kartoteka_db::test_helpers::{
            create_test_container, create_test_list_in_container, create_test_sublist,
        };

        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let root = create_test_container(&pool, &uid, None).await;
        let child = create_test_container(&pool, &uid, Some(&root)).await;
        let list = create_test_list_in_container(&pool, &uid, &child).await;
        let sublist = create_test_sublist(&pool, &uid, &list).await;
        let item_in_list = create_test_item(&pool, &list).await;
        let item_in_sublist = create_test_item(&pool, &sublist).await;

        for (item, secs, end_str) in [
            (&item_in_list, 60, "2026-01-01 10:01:00"),
            (&item_in_sublist, 120, "2026-01-01 10:02:00"),
        ] {
            // log_manual checks the list's time_tracking feature; insert directly
            // so the test only exercises aggregation.
            kartoteka_db::time_entries::insert(
                &pool,
                kartoteka_db::time_entries::InsertTimeEntryInput {
                    id: &Uuid::new_v4().to_string(),
                    item_id: Some(item),
                    user_id: &uid,
                    description: None,
                    started_at: "2026-01-01 10:00:00",
                    source: "manual",
                    mode: None,
                    ended_at: Some(end_str),
                    duration: Some(secs),
                },
            )
            .await
            .unwrap();
        }

        assert_eq!(
            total_for_list(&pool, &uid, &list).await.unwrap().seconds,
            180
        );
        assert_eq!(
            total_for_container(&pool, &uid, &root)
                .await
                .unwrap()
                .seconds,
            180
        );

        // Batch variants agree and fill in zeros.
        let items = totals_for_items(
            &pool,
            &uid,
            &[item_in_list.clone(), item_in_sublist.clone()],
        )
        .await
        .unwrap();
        assert_eq!(items.get(&item_in_list).unwrap().seconds, 60);
        assert_eq!(items.get(&item_in_sublist).unwrap().seconds, 120);

        let lists = totals_for_lists(&pool, &uid, std::slice::from_ref(&list))
            .await
            .unwrap();
        assert_eq!(lists.get(&list).unwrap().seconds, 180);

        let empty = totals_for_items(&pool, &uid, &[]).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn inbox_entries_do_not_count_towards_any_total() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        // Inbox entry (item_id NULL) plus a running one.
        kartoteka_db::time_entries::insert(
            &pool,
            kartoteka_db::time_entries::InsertTimeEntryInput {
                id: &Uuid::new_v4().to_string(),
                item_id: None,
                user_id: &uid,
                description: None,
                started_at: "2026-01-01 10:00:00",
                source: "manual",
                mode: None,
                ended_at: Some("2026-01-01 11:00:00"),
                duration: Some(3600),
            },
        )
        .await
        .unwrap();
        start(&pool, &uid, None).await.unwrap();

        assert_eq!(
            total_for_list(&pool, &uid, &list_id).await.unwrap().seconds,
            0
        );
        assert_eq!(
            total_for_item(&pool, &uid, &item_id).await.unwrap().seconds,
            0
        );
    }
}
