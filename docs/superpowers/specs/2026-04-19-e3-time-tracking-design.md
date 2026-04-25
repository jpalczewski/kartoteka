# E3: Time Tracking — Design Spec

**Goal:** Implement stopwatch-based time tracking with per-item timer widget, manual log, and an inbox page for unassigned entries.

**Architecture:** `db::time_entries` → `domain::time_entries` (rules + orchestration) → REST `/api/time-entries/*` + Leptos server functions → `ItemTimerWidget` on ItemDetailPage + `/time` inbox page. No schema changes — `time_entries` table and its indexes are already in the migration.

**Tech Stack:** Rust/SQLite (existing `time_entries` STRICT table), Axum REST, Leptos 0.8 SSR server functions, DaisyUI 5 components. `chrono` for duration arithmetic.

**Out of scope (follow-up issues):** Pomodoro mode, full timer UI (session history per item, charts), global floating timer widget, grouping by day on `/time`, Today page time aggregate.

---

## Schema (existing — no changes)

```sql
CREATE TABLE IF NOT EXISTS time_entries (
    id TEXT PRIMARY KEY,
    item_id TEXT REFERENCES items(id) ON DELETE SET NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    description TEXT,
    started_at TEXT NOT NULL,          -- datetime('now') server-side
    ended_at TEXT,                     -- NULL = running
    duration INTEGER,                  -- seconds, set on stop
    source TEXT NOT NULL,              -- 'timer' | 'manual'
    mode TEXT,                         -- 'stopwatch' (pomodoro = future)
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;
```

Running timer detection: `ended_at IS NULL`. At most one running timer per user enforced by domain (auto-stop on start).

---

## File Map

| Action | File |
|--------|------|
| Modify | `crates/db/src/types.rs` — add `TimeEntryRow` |
| Create | `crates/db/src/time_entries.rs` |
| Modify | `crates/db/src/lib.rs` — add `pub mod time_entries;` |
| Create | `crates/domain/src/time_entries.rs` |
| Modify | `crates/domain/src/lib.rs` — add `pub mod time_entries;` |
| Modify | `crates/shared/src/types.rs` — add `TimeEntry`, `ItemTimeSummary` |
| Create | `crates/server/src/time_entries.rs` — REST handlers |
| Modify | `crates/server/src/lib.rs` — add `pub mod time_entries;` |
| Modify | `crates/server/src/routes/mod.rs` — mount `/time-entries` |
| Create | `crates/frontend-v2/src/server_fns/time_entries.rs` |
| Modify | `crates/frontend-v2/src/server_fns/mod.rs` — add `pub mod time_entries;` |
| Create | `crates/frontend-v2/src/components/time_entries/mod.rs` — `ItemTimerWidget` |
| Modify | `crates/frontend-v2/src/components/mod.rs` — add `pub mod time_entries;` |
| Modify | `crates/frontend-v2/src/pages/item_detail.rs` — add `ItemTimerWidget` |
| Create | `crates/frontend-v2/src/pages/time.rs` — `/time` inbox page |
| Modify | `crates/frontend-v2/src/app.rs` (or router) — add `/time` route |

---

## Layer Design

### db::time_entries

```rust
pub struct TimeEntryRow {
    pub id: String,
    pub item_id: Option<String>,
    pub user_id: String,
    pub description: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration: Option<i64>,   // seconds
    pub source: String,
    pub mode: Option<String>,
    pub created_at: String,
}

pub struct InsertTimeEntryInput<'a> {
    pub id: &'a str,
    pub item_id: Option<&'a str>,
    pub user_id: &'a str,
    pub description: Option<&'a str>,
    pub started_at: &'a str,     // datetime('now') from domain
    pub source: &'a str,         // "timer" | "manual"
    pub mode: Option<&'a str>,   // "stopwatch" | None
    pub ended_at: Option<&'a str>,
    pub duration: Option<i64>,
}

pub async fn get_running(pool, user_id) -> Result<Option<TimeEntryRow>, DbError>
pub async fn list_for_item(pool, item_id, user_id) -> Result<Vec<TimeEntryRow>, DbError>
pub async fn list_inbox(pool, user_id) -> Result<Vec<TimeEntryRow>, DbError>  // item_id IS NULL
pub async fn insert(pool, input: InsertTimeEntryInput) -> Result<TimeEntryRow, DbError>
pub async fn stop(pool, id, user_id, ended_at, duration) -> Result<bool, DbError>
pub async fn assign(pool, id, item_id, user_id) -> Result<bool, DbError>
pub async fn delete(pool, id, user_id) -> Result<bool, DbError>
```

### domain::time_entries

**Types:**
```rust
pub struct TimeEntry {
    pub id: String,
    pub item_id: Option<String>,
    pub user_id: String,
    pub description: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration: Option<i64>,
    pub source: String,
    pub mode: Option<String>,
    pub created_at: String,
}

pub struct ItemTimeSummary {
    pub total_seconds: i64,
    pub entry_count: usize,
}
```

**Functions:**
```rust
// Start stopwatch timer. Auto-stops any currently running timer for this user.
pub async fn start(pool, user_id, item_id: Option<&str>) -> Result<TimeEntry, DomainError>

// Stop the running timer. Returns Ok(None) if no timer running (not an error).
pub async fn stop(pool, user_id) -> Result<Option<TimeEntry>, DomainError>

// Log a manual entry. Validates range and duration.
// Domain rules: ended_at > started_at, duration ≤ 86400s
pub async fn log_manual(
    pool, user_id,
    item_id: Option<&str>,
    started_at: &str,
    ended_at: &str,
    description: Option<&str>,
) -> Result<TimeEntry, DomainError>

// Assign an unassigned entry to an item. Checks item ownership.
pub async fn assign(pool, user_id, entry_id: &str, item_id: &str) -> Result<TimeEntry, DomainError>

pub async fn list_for_item(pool, user_id, item_id) -> Result<Vec<TimeEntry>, DomainError>
pub async fn list_inbox(pool, user_id) -> Result<Vec<TimeEntry>, DomainError>
pub async fn list_all_for_user(pool, user_id) -> Result<Vec<TimeEntry>, DomainError>

// Total time logged against an item (sum of duration for completed entries).
pub async fn summary_for_item(pool, user_id, item_id) -> Result<ItemTimeSummary, DomainError>

// Delete an entry. Ownership enforced via user_id in WHERE clause (returns Forbidden if not found).
pub async fn delete(pool, user_id, entry_id: &str) -> Result<(), DomainError>
```

**Domain rules:**
- `DomainError::Validation("invalid_time_range")` — `ended_at <= started_at`
- `DomainError::Validation("duration_too_long")` — duration > 86400 seconds (24h)
- `DomainError::Forbidden` — assign to item not owned by user; delete entry not owned by user

Auto-stop on start: transparent to caller, no error when previous timer exists.
Stop with no running timer: `Ok(None)`, not an error.

`started_at` uses `chrono::Utc::now()` computed in domain (not client-provided) for timer mode. For `log_manual`, client provides both timestamps as `"YYYY-MM-DD HH:MM:SS"` strings, validated via `chrono::NaiveDateTime::parse_from_str`.

### shared::types additions

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    pub item_id: Option<String>,
    pub user_id: String,
    pub description: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration: Option<i64>,
    pub source: String,
    pub mode: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTimeSummary {
    pub total_seconds: i64,
    pub entry_count: usize,
}
```

### REST endpoints (server::time_entries)

```
GET    /api/time-entries?item_id=:id     → Vec<TimeEntry>  (assigned to item)
GET    /api/time-entries/inbox           → Vec<TimeEntry>  (unassigned)
POST   /api/time-entries/start           → TimeEntry       body: {item_id?: str}
POST   /api/time-entries/stop            → TimeEntry | null
POST   /api/time-entries/log             → TimeEntry       body: {item_id?, started_at, ended_at, description?}
PATCH  /api/time-entries/:id/assign      → TimeEntry       body: {item_id: str}
DELETE /api/time-entries/:id             → 204
```

Route order: `/inbox` and `/start`/`/stop`/`/log` registered before `/:id` to avoid path conflicts.

All endpoints: `UserId` extractor (session or bearer), `AppError` mapping.

### Server functions (frontend-v2)

```rust
#[server(prefix = "/leptos")]
pub async fn get_time_summary(item_id: String) -> Result<ItemTimeSummary, ServerFnError>

#[server(prefix = "/leptos")]
pub async fn get_running_timer() -> Result<Option<TimeEntry>, ServerFnError>

#[server(prefix = "/leptos")]
pub async fn start_timer(item_id: String) -> Result<TimeEntry, ServerFnError>

#[server(prefix = "/leptos")]
pub async fn stop_timer() -> Result<Option<TimeEntry>, ServerFnError>

#[server(prefix = "/leptos")]
pub async fn get_inbox() -> Result<Vec<TimeEntry>, ServerFnError>

#[server(prefix = "/leptos")]
pub async fn log_time(
    item_id: Option<String>,
    started_at: String,
    ended_at: String,
    description: Option<String>,
) -> Result<TimeEntry, ServerFnError>

#[server(prefix = "/leptos")]
pub async fn assign_time_entry(entry_id: String, item_id: String) -> Result<TimeEntry, ServerFnError>

#[server(prefix = "/leptos")]
pub async fn list_all_entries() -> Result<Vec<TimeEntry>, ServerFnError>

#[server(prefix = "/leptos")]
pub async fn delete_time_entry(entry_id: String) -> Result<(), ServerFnError>
```

---

## Frontend Components

### ItemTimerWidget

Location: `crates/frontend-v2/src/components/time_entries/mod.rs`
Added to: `ItemDetailPage` after `RelatedEntities` section

**States:**
- **Idle, no time logged:** `"Czas: —  [▶ Start]"`
- **Idle, time logged:** `"Czas: 2h 15min  [▶ Start]"` (total_seconds from summary)
- **Running (this item):** `"● 00:42  [■ Stop]"` — elapsed ticks client-side via `set_interval(1000ms)`
- **Running (other item):** `"Czas: 2h 15min  [▶ Start (zatrzyma inny)]"` — start still allowed, auto-stops other

**Signals:**
- `summary_res: Resource<ItemTimeSummary>` — loads `get_time_summary(item_id)`
- `running_res: Resource<Option<TimeEntry>>` — loads `get_running_timer()`
- `elapsed: RwSignal<i64>` — ticks when running; initialized from `started_at` diff on load
- `(refresh, set_refresh): Signal<u32>` — increment to invalidate both resources

Start → `start_timer(item_id)` → `set_refresh.update(|n| *n += 1)` + reset elapsed.
Stop → `stop_timer()` → `set_refresh.update(...)`.
Errors → toast.

### TimePage (`/time`)

Location: `crates/frontend-v2/src/pages/time.rs`

Two sections:
1. **Inbox (nieprzypisane)** — entries where `item_id IS NULL`. Each row: `started_at`, `duration`, description, input field for item ID, "Przypisz" button → `assign_time_entry`. On success: refresh.
2. **Wszystkie wpisy** — full list for user via `list_all_entries` server fn → `domain::time_entries::list_all_for_user`. Columns: `item_id[..8]` or "—" (no JOIN — item title is a follow-up), `started_at`, formatted duration, delete button → `delete_time_entry`.

---

## Testing

### db::time_entries tests

- `insert_returns_entry` — fields match input
- `get_running_returns_active` — entry with `ended_at IS NULL` found; entry with `ended_at` set → None
- `get_running_returns_none_when_stopped` — after stop(), get_running() = None
- `stop_sets_ended_at_and_duration` — duration > 0, ended_at set
- `list_inbox_returns_only_unassigned` — 2 entries (1 assigned, 1 not) → inbox has 1
- `list_for_item_returns_assigned_entries` — 2 entries for different items → only correct one returned
- `assign_changes_item_id` — entry.item_id == item_id after assign
- `delete_removes_entry` — list_for_item empty after delete
- `delete_wrong_user_returns_false` — ownership check

### domain::time_entries tests

- `start_auto_stops_previous_timer` — start twice → first has ended_at, second running
- `start_with_item_assigns_item_id`
- `stop_returns_none_when_no_timer`
- `stop_sets_duration_correctly` — insert running entry with started_at 60s ago → duration ≈ 60
- `log_manual_rejects_invalid_range` — ended_at < started_at → Err(Validation("invalid_time_range"))
- `log_manual_rejects_too_long` — 25h range → Err(Validation("duration_too_long"))
- `log_manual_valid_entry` — correct times → entry with source="manual"
- `assign_forbidden_for_other_users_item` → Err(Forbidden)
- `summary_for_item_sums_completed_entries` — 2 entries (60s + 120s) → total_seconds=180, entry_count=2
- `summary_excludes_running_entry` — running entry (no duration) not counted in total

---

## Follow-up Issues (not in E3)

- **Pomodoro mode** — `mode = "pomodoro"`, configurable duration, break tracking
- **Full timer UI** — session history per item, charts, weekly summary
- **Global floating timer** — persistent widget across pages
- **Today page time aggregate** — "Dziś zalogowany czas" section
- **Grouping by day** on `/time` page
