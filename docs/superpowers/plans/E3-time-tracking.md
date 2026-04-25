# E3: Time Tracking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement stopwatch-based time tracking: db queries, domain orchestration (auto-stop, log manual, assign, summary), REST endpoints, Leptos server functions, an `ItemTimerWidget` on the item detail page, and a `/time` inbox page.

**Architecture:** `db::time_entries` (CRUD + running detection + summary) → `domain::time_entries` (auto-stop on start, validation rules) → REST `/api/time-entries/*` + server functions → `ItemTimerWidget` on `ItemDetailPage` + `TimePage` at `/time`. `TimeEntryRow` already exists in `db::types`. No schema changes needed — the `time_entries` table and its indexes are already in the migration.

**Tech Stack:** Rust/SQLite (`time_entries` STRICT table), sqlx, chrono, Axum, Leptos 0.8 SSR, DaisyUI 5.

---

## File Map

| Action | File |
|--------|------|
| Create | `crates/db/src/time_entries.rs` |
| Modify | `crates/db/src/lib.rs` — add `pub mod time_entries;` |
| Create | `crates/domain/src/time_entries.rs` |
| Modify | `crates/domain/src/lib.rs` — add `pub mod time_entries;` |
| Modify | `crates/shared/src/types.rs` — add `TimeEntry`, `ItemTimeSummary` |
| Create | `crates/server/src/time_entries.rs` |
| Modify | `crates/server/src/lib.rs` — add `pub mod time_entries;` |
| Modify | `crates/server/src/routes/mod.rs` — mount `/time-entries` |
| Create | `crates/frontend-v2/src/server_fns/time_entries.rs` |
| Modify | `crates/frontend-v2/src/server_fns/mod.rs` — add `pub mod time_entries;` |
| Create | `crates/frontend-v2/src/components/time_entries/mod.rs` |
| Modify | `crates/frontend-v2/src/components/mod.rs` — add `pub mod time_entries;` |
| Modify | `crates/frontend-v2/src/pages/item_detail.rs` — add `ItemTimerWidget` |
| Create | `crates/frontend-v2/src/pages/time.rs` |
| Modify | `crates/frontend-v2/src/pages/mod.rs` — add `pub mod time;` |
| Modify | `crates/frontend-v2/src/app.rs` — import + add `/time` route |

---

### Task 1: db::time_entries

**Files:**
- Create: `crates/db/src/time_entries.rs`
- Modify: `crates/db/src/lib.rs`

`TimeEntryRow` already exists in `crates/db/src/types.rs` — do **not** redefine it.

- [ ] **Step 1: Create file with stubs + full test module**

```rust
// crates/db/src/time_entries.rs
use crate::{DbError, types::TimeEntryRow};
use sqlx::SqlitePool;

// ── Read ──────────────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<TimeEntryRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn get_running(pool: &SqlitePool, user_id: &str) -> Result<Option<TimeEntryRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn list_for_item(
    pool: &SqlitePool,
    item_id: &str,
    user_id: &str,
) -> Result<Vec<TimeEntryRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn list_inbox(pool: &SqlitePool, user_id: &str) -> Result<Vec<TimeEntryRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn list_all_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<TimeEntryRow>, DbError> {
    todo!()
}

/// Returns (total_seconds, entry_count) for completed entries on an item.
#[tracing::instrument(skip(pool))]
pub async fn summary_for_item(
    pool: &SqlitePool,
    item_id: &str,
    user_id: &str,
) -> Result<(i64, i64), DbError> {
    todo!()
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
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn stop(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    ended_at: &str,
    duration: i32,
) -> Result<bool, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn assign(
    pool: &SqlitePool,
    id: &str,
    item_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    todo!()
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
        make_entry(&pool, &uid, Some(&item_id), Some("2026-01-01 10:01:00"), Some(60)).await;
        make_entry(&pool, &uid, Some(&item_id), Some("2026-01-01 10:03:00"), Some(120)).await;
        // One running entry (no duration) — should NOT be counted
        make_entry(&pool, &uid, Some(&item_id), None, None).await;

        let (total, count) = summary_for_item(&pool, &item_id, &uid).await.unwrap();
        assert_eq!(total, 180);
        assert_eq!(count, 2);
    }
}
```

- [ ] **Step 2: Add module to db lib**

In `crates/db/src/lib.rs`, add after the `relations` line:

```rust
pub mod time_entries;
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test -p kartoteka-db time_entries 2>&1 | tail -20
```

Expected: compilation error from `todo!()` on the first called function.

- [ ] **Step 4: Implement all functions**

Replace the `todo!()` bodies in `crates/db/src/time_entries.rs`:

```rust
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

pub async fn get_running(pool: &SqlitePool, user_id: &str) -> Result<Option<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(
        "SELECT id, item_id, user_id, description, started_at, ended_at, duration, source, mode, created_at \
         FROM time_entries WHERE user_id = ? AND ended_at IS NULL LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

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

pub async fn assign(
    pool: &SqlitePool,
    id: &str,
    item_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE time_entries SET item_id = ? WHERE id = ? AND user_id = ?",
    )
    .bind(item_id)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM time_entries WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}
```

- [ ] **Step 5: Run tests to verify all pass**

```bash
cargo test -p kartoteka-db time_entries 2>&1 | tail -20
```

Expected: 8 tests pass, 0 failures.

- [ ] **Step 6: Commit**

```bash
git add crates/db/src/time_entries.rs crates/db/src/lib.rs
git commit -m "feat(db): add time_entries queries (CRUD, running, inbox, summary)"
```

---

### Task 2: domain::time_entries

**Files:**
- Create: `crates/domain/src/time_entries.rs`
- Modify: `crates/domain/src/lib.rs`

- [ ] **Step 1: Create file with stubs + full test module**

```rust
// crates/domain/src/time_entries.rs
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
    todo!()
}

/// Stop the running timer. Returns Ok(None) if no timer is running (not an error).
#[tracing::instrument(skip(pool))]
pub async fn stop(pool: &SqlitePool, user_id: &str) -> Result<Option<TimeEntry>, DomainError> {
    todo!()
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
    todo!()
}

/// Assign an unassigned entry to an item. Checks item ownership.
#[tracing::instrument(skip(pool))]
pub async fn assign(
    pool: &SqlitePool,
    user_id: &str,
    entry_id: &str,
    item_id: &str,
) -> Result<TimeEntry, DomainError> {
    todo!()
}

/// Delete a time entry. Returns Forbidden if not owned by user or not found.
#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, entry_id: &str) -> Result<(), DomainError> {
    todo!()
}

pub async fn list_for_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
) -> Result<Vec<TimeEntry>, DomainError> {
    todo!()
}

pub async fn list_inbox(pool: &SqlitePool, user_id: &str) -> Result<Vec<TimeEntry>, DomainError> {
    todo!()
}

pub async fn list_all_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<TimeEntry>, DomainError> {
    todo!()
}

pub async fn summary_for_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
) -> Result<ItemTimeSummary, DomainError> {
    todo!()
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
        assert!(kartoteka_db::time_entries::get_running(&pool, &uid)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn log_manual_rejects_invalid_range() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let result = log_manual(
            &pool, &uid, None,
            "2026-01-01 12:00:00", // started_at
            "2026-01-01 11:00:00", // ended_at BEFORE started_at
            None,
        )
        .await;
        assert!(matches!(result, Err(DomainError::Validation("invalid_time_range"))));
    }

    #[tokio::test]
    async fn log_manual_rejects_too_long() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let result = log_manual(
            &pool, &uid, None,
            "2026-01-01 00:00:00",
            "2026-01-02 01:00:00", // 25h — too long
            None,
        )
        .await;
        assert!(matches!(result, Err(DomainError::Validation("duration_too_long"))));
    }

    #[tokio::test]
    async fn log_manual_valid_entry_has_source_manual() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let entry = log_manual(
            &pool, &uid, None,
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

        log_manual(&pool, &uid, Some(&item_id), "2026-01-01 10:00:00", "2026-01-01 10:01:00", None).await.unwrap(); // 60s
        log_manual(&pool, &uid, Some(&item_id), "2026-01-01 11:00:00", "2026-01-01 11:02:00", None).await.unwrap(); // 120s
        // Running entry — should NOT be counted
        start(&pool, &uid, Some(&item_id)).await.unwrap();

        let summary = summary_for_item(&pool, &uid, &item_id).await.unwrap();
        assert_eq!(summary.total_seconds, 180);
        assert_eq!(summary.entry_count, 2);
    }
}
```

- [ ] **Step 2: Add module to domain lib**

In `crates/domain/src/lib.rs`, add after the `relations` line:

```rust
pub mod time_entries;
```

- [ ] **Step 3: Run to see failures**

```bash
cargo test -p kartoteka-domain time_entries 2>&1 | tail -20
```

Expected: compilation error from `todo!()`.

- [ ] **Step 4: Implement all functions**

Replace the `todo!()` bodies in `crates/domain/src/time_entries.rs`:

```rust
pub async fn start(
    pool: &SqlitePool,
    user_id: &str,
    item_id: Option<&str>,
) -> Result<TimeEntry, DomainError> {
    // Auto-stop any running timer
    if let Some(running) = db::time_entries::get_running(pool, user_id).await? {
        let now = chrono::Utc::now();
        let now_s = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let started = chrono::NaiveDateTime::parse_from_str(&running.started_at, "%Y-%m-%d %H:%M:%S")
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
    Ok(ItemTimeSummary { total_seconds, entry_count })
}
```

- [ ] **Step 5: Run tests to verify all pass**

```bash
cargo test -p kartoteka-domain time_entries 2>&1 | tail -20
```

Expected: 10 tests pass, 0 failures.

- [ ] **Step 6: Run full test suite**

```bash
cargo test --workspace 2>&1 | tail -10
```

Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add crates/domain/src/time_entries.rs crates/domain/src/lib.rs
git commit -m "feat(domain): add time_entries orchestration (start/stop/log/assign/summary)"
```

---

### Task 3: shared types + REST endpoints

**Files:**
- Modify: `crates/shared/src/types.rs`
- Create: `crates/server/src/time_entries.rs`
- Modify: `crates/server/src/lib.rs`
- Modify: `crates/server/src/routes/mod.rs`

- [ ] **Step 1: Add shared types**

In `crates/shared/src/types.rs`, append after the `Relation` struct (end of file):

```rust
// --- Time Entries ---

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
```

- [ ] **Step 2: Create server/src/time_entries.rs**

```rust
use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};

#[derive(serde::Deserialize)]
struct ItemQuery {
    item_id: String,
}

#[derive(serde::Deserialize)]
struct StartRequest {
    item_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct LogRequest {
    item_id: Option<String>,
    started_at: String,
    ended_at: String,
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct AssignRequest {
    item_id: String,
}

pub fn time_entries_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_for_item))
        .route("/inbox", get(list_inbox))
        .route("/start", post(start_timer))
        .route("/stop", post(stop_timer))
        .route("/log", post(log_time))
        .route("/:id/assign", patch(assign_entry))
        .route("/:id", delete(delete_entry))
}

#[tracing::instrument(skip_all, fields(action = "list_time_entries", item_id = %q.item_id))]
async fn list_for_item(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Query(q): Query<ItemQuery>,
) -> Result<impl IntoResponse, AppError> {
    let entries =
        kartoteka_domain::time_entries::list_for_item(&state.pool, &uid, &q.item_id).await?;
    Ok(Json(entries))
}

#[tracing::instrument(skip_all, fields(action = "list_inbox"))]
async fn list_inbox(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let entries = kartoteka_domain::time_entries::list_inbox(&state.pool, &uid).await?;
    Ok(Json(entries))
}

#[tracing::instrument(skip_all, fields(action = "start_timer"))]
async fn start_timer(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<StartRequest>,
) -> Result<impl IntoResponse, AppError> {
    let entry =
        kartoteka_domain::time_entries::start(&state.pool, &uid, req.item_id.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

#[tracing::instrument(skip_all, fields(action = "stop_timer"))]
async fn stop_timer(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let entry = kartoteka_domain::time_entries::stop(&state.pool, &uid).await?;
    Ok(Json(entry))
}

#[tracing::instrument(skip_all, fields(action = "log_time"))]
async fn log_time(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<LogRequest>,
) -> Result<impl IntoResponse, AppError> {
    let entry = kartoteka_domain::time_entries::log_manual(
        &state.pool,
        &uid,
        req.item_id.as_deref(),
        &req.started_at,
        &req.ended_at,
        req.description.as_deref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

#[tracing::instrument(skip_all, fields(action = "assign_time_entry", entry_id = %id))]
async fn assign_entry(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<AssignRequest>,
) -> Result<impl IntoResponse, AppError> {
    let entry =
        kartoteka_domain::time_entries::assign(&state.pool, &uid, &id, &req.item_id).await?;
    Ok(Json(entry))
}

#[tracing::instrument(skip_all, fields(action = "delete_time_entry", entry_id = %id))]
async fn delete_entry(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::time_entries::delete(&state.pool, &uid, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 3: Wire module and route**

In `crates/server/src/lib.rs`, add after `pub mod relations;`:

```rust
pub mod time_entries;
```

In `crates/server/src/routes/mod.rs`, add after `.nest("/relations", ...)`:

```rust
.nest("/time-entries", crate::time_entries::time_entries_router())
```

- [ ] **Step 4: Run cargo check**

```bash
cargo check -p kartoteka-server 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add crates/shared/src/types.rs \
    crates/server/src/time_entries.rs \
    crates/server/src/lib.rs \
    crates/server/src/routes/mod.rs
git commit -m "feat(server): add time_entries REST endpoints (start/stop/log/assign/inbox)"
```

---

### Task 4: Server functions

**Files:**
- Create: `crates/frontend-v2/src/server_fns/time_entries.rs`
- Modify: `crates/frontend-v2/src/server_fns/mod.rs`

- [ ] **Step 1: Create server_fns/time_entries.rs**

```rust
use kartoteka_shared::types::{ItemTimeSummary, TimeEntry};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession, kartoteka_auth::KartotekaBackend, kartoteka_domain as domain,
    sqlx::SqlitePool,
};

#[cfg(feature = "ssr")]
fn domain_entry_to_shared(e: domain::time_entries::TimeEntry) -> TimeEntry {
    TimeEntry {
        id: e.id,
        item_id: e.item_id,
        user_id: e.user_id,
        description: e.description,
        started_at: e.started_at,
        ended_at: e.ended_at,
        duration: e.duration,
        source: e.source,
        mode: e.mode,
        created_at: e.created_at,
    }
}

#[cfg(feature = "ssr")]
fn domain_summary_to_shared(s: domain::time_entries::ItemTimeSummary) -> ItemTimeSummary {
    ItemTimeSummary { total_seconds: s.total_seconds, entry_count: s.entry_count }
}

#[cfg(feature = "ssr")]
async fn current_user_id() -> Result<String, ServerFnError> {
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    auth.user
        .map(|u| u.id)
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))
}

#[server(prefix = "/leptos")]
pub async fn get_time_summary(item_id: String) -> Result<ItemTimeSummary, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let summary = domain::time_entries::summary_for_item(&pool, &uid, &item_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_summary_to_shared(summary))
}

#[server(prefix = "/leptos")]
pub async fn get_running_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = kartoteka_db::time_entries::get_running(&pool, &uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(entry.map(|row| TimeEntry {
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
    }))
}

#[server(prefix = "/leptos")]
pub async fn start_timer(item_id: String) -> Result<TimeEntry, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = domain::time_entries::start(&pool, &uid, Some(&item_id))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn stop_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = domain::time_entries::stop(&pool, &uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(entry.map(domain_entry_to_shared))
}

#[server(prefix = "/leptos")]
pub async fn get_inbox() -> Result<Vec<TimeEntry>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entries = domain::time_entries::list_inbox(&pool, &uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(entries.into_iter().map(domain_entry_to_shared).collect())
}

#[server(prefix = "/leptos")]
pub async fn list_all_entries() -> Result<Vec<TimeEntry>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entries = domain::time_entries::list_all_for_user(&pool, &uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(entries.into_iter().map(domain_entry_to_shared).collect())
}

#[server(prefix = "/leptos")]
pub async fn log_time(
    item_id: Option<String>,
    started_at: String,
    ended_at: String,
    description: Option<String>,
) -> Result<TimeEntry, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = domain::time_entries::log_manual(
        &pool,
        &uid,
        item_id.as_deref(),
        &started_at,
        &ended_at,
        description.as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn assign_time_entry(
    entry_id: String,
    item_id: String,
) -> Result<TimeEntry, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = domain::time_entries::assign(&pool, &uid, &entry_id, &item_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn delete_time_entry(entry_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    domain::time_entries::delete(&pool, &uid, &entry_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
```

- [ ] **Step 2: Add to server_fns/mod.rs**

In `crates/frontend-v2/src/server_fns/mod.rs`, add:

```rust
pub mod time_entries;
```

- [ ] **Step 3: Cargo check**

```bash
cargo check -p kartoteka-frontend-v2 --features ssr 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add crates/frontend-v2/src/server_fns/time_entries.rs \
    crates/frontend-v2/src/server_fns/mod.rs
git commit -m "feat(frontend-v2): add time_entries server functions"
```

---

### Task 5: ItemTimerWidget component

**Files:**
- Create: `crates/frontend-v2/src/components/time_entries/mod.rs`
- Modify: `crates/frontend-v2/src/components/mod.rs`
- Modify: `crates/frontend-v2/src/pages/item_detail.rs`

- [ ] **Step 1: Create the component**

```rust
// crates/frontend-v2/src/components/time_entries/mod.rs
use kartoteka_shared::types::TimeEntry;
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::server_fns::time_entries::{get_running_timer, get_time_summary, start_timer, stop_timer};

fn format_seconds(secs: i64) -> String {
    if secs == 0 {
        return "—".to_string();
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m:02}min")
    } else if m > 0 {
        format!("{m}min {s:02}s")
    } else {
        format!("{s}s")
    }
}

#[component]
pub fn ItemTimerWidget(item_id: Signal<String>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let summary_res = Resource::new(
        move || (item_id.get(), refresh.get()),
        move |(eid, _)| get_time_summary(eid),
    );

    let running_res = Resource::new(
        move || refresh.get(),
        move |_| get_running_timer(),
    );

    // True when this item has the active running timer
    let is_running_this_item = move || {
        let eid = item_id.get();
        running_res
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|e: TimeEntry| e.item_id.as_deref() == Some(eid.as_str()))
            .unwrap_or(false)
    };

    // True when a different item has an active timer
    let is_running_other = move || {
        let eid = item_id.get();
        running_res
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|e: TimeEntry| e.item_id.as_deref() != Some(eid.as_str()))
            .unwrap_or(false)
    };

    let on_start = move |_: leptos::ev::MouseEvent| {
        let eid = item_id.get();
        leptos::task::spawn_local(async move {
            match start_timer(eid).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let on_stop = move |_: leptos::ev::MouseEvent| {
        leptos::task::spawn_local(async move {
            match stop_timer().await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="mt-6">
            <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-3">
                "Czas"
            </h3>

            <div class="flex items-center gap-3">
                <Suspense fallback=|| view! { <span class="loading loading-dots loading-xs"></span> }>
                    {move || {
                        let total = summary_res
                            .get()
                            .and_then(|r| r.ok())
                            .map(|s| s.total_seconds)
                            .unwrap_or(0);

                        if is_running_this_item() {
                            view! {
                                <span class="text-sm font-mono text-success">
                                    {"● Trwa"}
                                </span>
                            }.into_any()
                        } else {
                            view! {
                                <span class="text-sm text-base-content/70">
                                    {format_seconds(total)}
                                </span>
                            }.into_any()
                        }
                    }}
                </Suspense>

                <Suspense fallback=|| view! {}>
                    {move || {
                        if is_running_this_item() {
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-sm btn-error"
                                    on:click=on_stop
                                >
                                    {"■ Stop"}
                                </button>
                            }.into_any()
                        } else {
                            let label = if is_running_other() {
                                "▶ Start (zatrzyma inny)"
                            } else {
                                "▶ Start"
                            };
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-sm btn-outline"
                                    on:click=on_start
                                >
                                    {label}
                                </button>
                            }.into_any()
                        }
                    }}
                </Suspense>
            </div>
        </div>
    }
}
```

- [ ] **Step 2: Add to components/mod.rs**

In `crates/frontend-v2/src/components/mod.rs`, add:

```rust
pub mod time_entries;
```

- [ ] **Step 3: Add ItemTimerWidget to ItemDetailPage**

In `crates/frontend-v2/src/pages/item_detail.rs`:

Add import at the top:

```rust
use crate::components::time_entries::ItemTimerWidget;
```

After the `// Relations` block (line ~139), add:

```rust
                                // Time tracking
                                <ItemTimerWidget
                                    item_id=Signal::derive(item_id)
                                />
```

- [ ] **Step 4: Cargo check**

```bash
cargo check -p kartoteka-frontend-v2 --features ssr 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add crates/frontend-v2/src/components/time_entries/mod.rs \
    crates/frontend-v2/src/components/mod.rs \
    crates/frontend-v2/src/pages/item_detail.rs
git commit -m "feat(frontend-v2): add ItemTimerWidget on ItemDetailPage"
```

---

### Task 6: TimePage + routing

**Files:**
- Create: `crates/frontend-v2/src/pages/time.rs`
- Modify: `crates/frontend-v2/src/pages/mod.rs`
- Modify: `crates/frontend-v2/src/app.rs`

- [ ] **Step 1: Create the TimePage**

```rust
// crates/frontend-v2/src/pages/time.rs
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::server_fns::time_entries::{assign_time_entry, delete_time_entry, get_inbox, list_all_entries};

fn format_duration(secs: Option<i32>) -> String {
    match secs {
        None => "Trwa…".to_string(),
        Some(s) => {
            let h = s / 3600;
            let m = (s % 3600) / 60;
            let sec = s % 60;
            if h > 0 {
                format!("{h}h {m:02}min")
            } else if m > 0 {
                format!("{m}min {sec:02}s")
            } else {
                format!("{sec}s")
            }
        }
    }
}

fn truncate_id(id: &str) -> &str {
    &id[..8.min(id.len())]
}

#[component]
pub fn TimePage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let inbox_res = Resource::new(move || refresh.get(), move |_| get_inbox());
    let all_res = Resource::new(move || refresh.get(), move |_| list_all_entries());

    // Per-row item_id inputs for assign
    let assign_inputs: RwSignal<std::collections::HashMap<String, String>> =
        RwSignal::new(std::collections::HashMap::new());

    let on_assign = move |entry_id: String| {
        let item_id = assign_inputs.with(|m| m.get(&entry_id).cloned().unwrap_or_default());
        if item_id.trim().is_empty() {
            return;
        }
        leptos::task::spawn_local(async move {
            match assign_time_entry(entry_id, item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let on_delete = move |entry_id: String| {
        leptos::task::spawn_local(async move {
            match delete_time_entry(entry_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="container mx-auto max-w-3xl p-4">
            <h1 class="text-2xl font-bold mb-6">"Czas"</h1>

            // ── Inbox ──────────────────────────────────────────────────────────
            <section class="mb-8">
                <h2 class="text-lg font-semibold mb-3">"Nieprzypisane wpisy (inbox)"</h2>
                <Suspense fallback=|| view! { <span class="loading loading-dots"></span> }>
                    {move || {
                        match inbox_res.get() {
                            Some(Ok(entries)) if entries.is_empty() => view! {
                                <p class="text-base-content/40 italic">"Brak nieprzypisanych wpisów."</p>
                            }.into_any(),
                            Some(Ok(entries)) => view! {
                                <div class="overflow-x-auto">
                                    <table class="table table-sm">
                                        <thead>
                                            <tr>
                                                <th>"Rozpoczęto"</th>
                                                <th>"Czas"</th>
                                                <th>"Opis"</th>
                                                <th>"Przypisz do zadania"</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {entries.into_iter().map(|e| {
                                                let eid = e.id.clone();
                                                let eid2 = e.id.clone();
                                                let eid3 = e.id.clone();
                                                view! {
                                                    <tr>
                                                        <td class="font-mono text-xs">{e.started_at.clone()}</td>
                                                        <td>{format_duration(e.duration)}</td>
                                                        <td>{e.description.clone().unwrap_or_default()}</td>
                                                        <td>
                                                            <div class="flex gap-1">
                                                                <input
                                                                    type="text"
                                                                    class="input input-bordered input-xs w-32"
                                                                    placeholder="ID zadania…"
                                                                    prop:value=move || assign_inputs.with(|m| m.get(&eid).cloned().unwrap_or_default())
                                                                    on:input=move |ev| {
                                                                        let val = event_target_value(&ev);
                                                                        assign_inputs.update(|m| { m.insert(eid2.clone(), val); });
                                                                    }
                                                                />
                                                                <button
                                                                    type="button"
                                                                    class="btn btn-xs btn-primary"
                                                                    on:click=move |_| on_assign(eid3.clone())
                                                                >
                                                                    "Przypisz"
                                                                </button>
                                                            </div>
                                                        </td>
                                                        <td>
                                                            <button
                                                                type="button"
                                                                class="btn btn-xs btn-ghost text-error"
                                                                on:click={
                                                                    let id = e.id.clone();
                                                                    move |_| on_delete(id.clone())
                                                                }
                                                            >
                                                                "✕"
                                                            </button>
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                            }.into_any(),
                            Some(Err(e)) => view! {
                                <p class="text-error">"Błąd: " {e.to_string()}</p>
                            }.into_any(),
                            None => view! {}.into_any(),
                        }
                    }}
                </Suspense>
            </section>

            // ── All entries ────────────────────────────────────────────────────
            <section>
                <h2 class="text-lg font-semibold mb-3">"Wszystkie wpisy"</h2>
                <Suspense fallback=|| view! { <span class="loading loading-dots"></span> }>
                    {move || {
                        match all_res.get() {
                            Some(Ok(entries)) if entries.is_empty() => view! {
                                <p class="text-base-content/40 italic">"Brak wpisów."</p>
                            }.into_any(),
                            Some(Ok(entries)) => view! {
                                <div class="overflow-x-auto">
                                    <table class="table table-sm">
                                        <thead>
                                            <tr>
                                                <th>"Zadanie"</th>
                                                <th>"Rozpoczęto"</th>
                                                <th>"Czas"</th>
                                                <th>"Źródło"</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {entries.into_iter().map(|e| {
                                                let item_label = e.item_id
                                                    .as_deref()
                                                    .map(truncate_id)
                                                    .map(|s| s.to_string())
                                                    .unwrap_or_else(|| "—".to_string());
                                                let id = e.id.clone();
                                                view! {
                                                    <tr>
                                                        <td class="font-mono text-xs">{item_label}</td>
                                                        <td class="font-mono text-xs">{e.started_at.clone()}</td>
                                                        <td>{format_duration(e.duration)}</td>
                                                        <td>{e.source.clone()}</td>
                                                        <td>
                                                            <button
                                                                type="button"
                                                                class="btn btn-xs btn-ghost text-error"
                                                                on:click=move |_| on_delete(id.clone())
                                                            >
                                                                "✕"
                                                            </button>
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>
                                </div>
                            }.into_any(),
                            Some(Err(e)) => view! {
                                <p class="text-error">"Błąd: " {e.to_string()}</p>
                            }.into_any(),
                            None => view! {}.into_any(),
                        }
                    }}
                </Suspense>
            </section>
        </div>
    }
}
```

- [ ] **Step 2: Add to pages/mod.rs**

In `crates/frontend-v2/src/pages/mod.rs`, add:

```rust
pub mod time;
```

- [ ] **Step 3: Add route and import to app.rs**

In `crates/frontend-v2/src/app.rs`, add `time::TimePage` to the existing use block:

```rust
use crate::pages::{
    // existing entries...
    time::TimePage,
    // ...
};
```

Add route inside `<Routes ...>`, after the `/today` route:

```rust
<Route path=path!("/time") view=TimePage/>
```

- [ ] **Step 4: Cargo check**

```bash
cargo check -p kartoteka-frontend-v2 --features ssr 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 5: Run full test suite**

```bash
cargo test --workspace 2>&1 | tail -20
```

Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/frontend-v2/src/pages/time.rs \
    crates/frontend-v2/src/pages/mod.rs \
    crates/frontend-v2/src/app.rs
git commit -m "feat(frontend-v2): add TimePage at /time (inbox + all entries)"
```

---

## Self-Review

### Spec coverage

| Spec requirement | Task |
|-----------------|------|
| `db::time_entries` — CRUD, running, inbox, summary | Task 1 |
| `domain::time_entries` — start (auto-stop), stop, log_manual, assign, summary | Task 2 |
| Domain rules: invalid_time_range, duration_too_long, Forbidden | Task 2 |
| Shared types: `TimeEntry`, `ItemTimeSummary` | Task 3 |
| REST: GET /inbox, POST /start, POST /stop, POST /log, PATCH /:id/assign, DELETE /:id | Task 3 |
| Server functions: all 9 functions | Task 4 |
| `ItemTimerWidget` on ItemDetailPage | Task 5 |
| `/time` page: inbox + all entries + assign + delete | Task 6 |
| MCP tool prep: `start_timer`, `stop_timer`, `log_time` domain functions ready for F1 | ✅ Task 2 |

### Placeholder scan

No TBD, TODO, or "similar to Task N" references. All code blocks are complete.

### Type consistency

- `TimeEntryRow.duration: Option<i32>` (existing types.rs) — used throughout ✓
- `domain::time_entries::TimeEntry.duration: Option<i32>` — matches ✓
- `shared::types::TimeEntry.duration: Option<i32>` — matches ✓
- `ItemTimeSummary.total_seconds: i64`, `.entry_count: i64` — consistent across db (i64,i64), domain, shared ✓
- `format_seconds(i64)` in widget vs `format_duration(Option<i32>)` in TimePage — correct, different use cases ✓
- `domain_entry_to_shared` and `domain_summary_to_shared` in server_fns — all field names match domain types ✓
- `get_running_timer` converts `TimeEntryRow` inline (no domain call needed — just db read) ✓
- Route `.route("/inbox", get(list_inbox))` registered before `"/:id"` — no conflict ✓
