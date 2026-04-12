# Plan 1 + 1a: DB Layer + Domain Layer — Design Spec

Parent: `docs/superpowers/specs/rewrite/00-main-architecture.md`

## Goal

Create `crates/db` (pure queries, SQLite-optimized) and `crates/domain` (business logic, validation, orchestration). Together they form the data + logic foundation consumed by server functions, REST handlers, and MCP tools.

## Architecture

### Crate boundary

- **db::** — pure queries. INSERT/UPDATE/DELETE enforce user_id (defense in depth). Zero business logic. SQLite-specific optimizations (RETURNING, json_group_array, partial indexes, STRICT tables).
- **domain::** — business rules, validation, orchestration. Calls db:: for data access. Manages transactions. All mutations go through domain::.
- **rules::** (submodule of domain) — pure synchronous functions. Zero I/O, zero deps. Unit testable with `#[test]`.

### Request pipeline

Every mutation follows a Read → Think → Write pipeline:

```
Axum request (tokio task)
  → domain:: orchestration (async)
    Phase 1: READ   — db:: queries (async, SQLite reader via pool)
    Phase 2: THINK  — rules:: validation (sync, pure, inline on tokio)
    Phase 3: WRITE  — db:: mutations (async, short transaction, RETURNING)
```

Each phase uses appropriate resources and releases them as fast as possible.

### All access through domain::

Consumers (server functions, REST handlers, MCP tools) NEVER call db:: directly. All access goes through domain::. This ensures:
- Centralized access control (even for reads)
- Single point for future field filtering, audit logging, sharing rules
- Consistent API — no "is this a read or write?" decision at call site

```
GET  /api/lists           → domain::lists::list_all     (pass-through now, hook for future)
POST /api/lists           → domain::lists::create        (validation + features + transaction)
GET  /api/lists/:id       → domain::lists::get_one       (pass-through now)
PUT  /api/lists/:id       → domain::lists::update         (validation)
PATCH /api/items/:id/move → domain::items::move_item      (ownership + position + validation)
```

Read functions in domain:: are thin pass-throughs today:

```rust
// domain/src/lists.rs
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<List>, DomainError> {
    // Future: access control, field filtering, audit logging
    Ok(db::lists::list_all(pool, user_id).await?)
}
```

One line now, but a single expansion point for access control later.

## crates/db

### Connection setup

```rust
let options = SqliteConnectOptions::from_str(url)?
    .create_if_missing(true)
    .foreign_keys(true)
    .journal_mode(SqliteJournalMode::Wal)
    .synchronous(SqliteSynchronous::Normal);

SqlitePoolOptions::new()
    .max_connections(8)      // WAL = concurrent reads
    .min_connections(2)      // warm pool, no cold start
    .after_connect(|conn, _meta| Box::pin(async move {
        sqlx::query("PRAGMA busy_timeout = 5000").execute(&mut *conn).await?;
        sqlx::query("PRAGMA mmap_size = 268435456").execute(&mut *conn).await?;
        sqlx::query("PRAGMA optimize = 0x10002").execute(&mut *conn).await?;
        Ok(())
    }))
    .connect_with(options)
    .await
```

### Migration features

- **STRICT tables** — all tables, enforces column types
- **RETURNING** — INSERT/UPDATE RETURNING * instead of INSERT + SELECT (1 round-trip vs 2)
- **Partial indexes** — on hot query columns:
  ```sql
  CREATE INDEX idx_items_deadline ON items(deadline) WHERE deadline IS NOT NULL;
  CREATE INDEX idx_items_start_date ON items(start_date) WHERE start_date IS NOT NULL;
  CREATE INDEX idx_lists_pinned ON lists(user_id, pinned) WHERE pinned = 1;
  CREATE INDEX idx_containers_pinned ON containers(user_id, pinned) WHERE pinned = 1;
  CREATE INDEX idx_lists_container ON lists(container_id) WHERE container_id IS NOT NULL;
  ```

### SQLite-specific features

- `json_group_array(json_object(...))` — feature aggregation in one query
- `WITH RECURSIVE` — tag tree, cycle detection
- `datetime('now')` — server-side timestamps
- `ON CONFLICT ... DO UPDATE` — upserts

### FlexDate type (chrono-based with fuzzy precision)

Date fields use `FlexDate` enum instead of `Option<String>` (#51):

```rust
// shared/src/types.rs
pub enum FlexDate {
    Day(chrono::NaiveDate),    // "2026-05-15"
    Week(u16, u8),             // (2026, 20) → "2026-W20" (ISO week)
    Month(u16, u8),            // (2026, 5) → "2026-05"
}
```

SQLite storage: TEXT column. Format detection: 10 chars = day, 8 chars `YYYY-Wnn` = week, 7 chars `YYYY-MM` = month. Custom sqlx `Encode`/`Decode` (~30 LOC).

chrono integration — `FlexDate` wraps chrono, not replaces it:

```rust
impl FlexDate {
    pub fn start(&self) -> NaiveDate { /* first day of period */ }
    pub fn end(&self) -> NaiveDate { /* last day of period */ }
    pub fn is_fuzzy(&self) -> bool { !matches!(self, FlexDate::Day(_)) }
    pub fn matches_day(&self, day: NaiveDate) -> bool { /* exact for Day, range for Week/Month */ }
}
```

Sorting, comparisons, timezone conversion — all through chrono via `start()`/`end()`.

Time fields (`start_time`, `deadline_time`) remain `Option<chrono::NaiveTime>` — times require precise dates.

Serde: serializes to ISO 8601 string ("2026-05-15", "2026-W20", "2026-05"). Compatible with current JSON API for day-level dates.

### Context queries

Instead of many small queries per domain operation, db:: provides dedicated context functions:

```rust
// db/src/lists.rs
pub struct CreateItemContext {
    pub features: Vec<String>,
    pub next_position: i32,
}

/// One query: ownership check + features + MAX(position)
pub async fn get_create_item_context(
    pool: &SqlitePool, list_id: &str, user_id: &str
) -> Result<CreateItemContext>
```

This minimizes round-trips. Domain calls one context query, validates with rules::, then writes.

### User timezone

`user_settings` key `timezone` (default `"UTC"`). Domain layer uses `chrono-tz` to resolve "today" in user's timezone before querying db::. Example:

```rust
// domain/src/items.rs
pub async fn by_date(pool, user_id, date, ...) -> Result<Vec<DateItem>> {
    // If date == "today", resolve using user's timezone
    let tz = db::preferences::get_timezone(pool, user_id).await?;
    let resolved_date = chrono::Utc::now().with_timezone(&tz).date_naive();
    db::items::by_date(pool, user_id, resolved_date, ...).await
}
```

MCP tools and frontend use the same domain:: functions — timezone handling is centralized.

`chrono-tz` added to `crates/domain` and `crates/shared` dependencies.

### Concurrent reads with tokio::join!

For endpoints that need multiple independent queries (home, container detail):

```rust
let (pinned_lists, pinned_containers, recent_lists, recent_containers, root_containers, root_lists) = tokio::join!(
    db::lists::pinned(pool, user_id),
    db::containers::pinned(pool, user_id),
    db::lists::recent(pool, user_id),
    db::containers::recent(pool, user_id),
    db::containers::root(pool, user_id),
    db::lists::root(pool, user_id),
);
```

WAL mode allows concurrent readers — 6 parallel reads on one pool.

### Module structure

```
crates/db/src/
  lib.rs              — create_pool, DbError, re-exports
  containers.rs       — CRUD + children + progress queries
  items.rs            — CRUD + by-date + calendar queries
  lists.rs            — CRUD + sublists + features queries + context queries
  tags.rs             — CRUD + recursive CTE + tag links
  settings.rs         — user_settings key-value
  preferences.rs      — locale, timezone (reads from user_settings table, no separate preferences table)
  home.rs             — composite home query (6 parallel SELECTs)
  helpers.rs          — check_ownership, next_position, toggle_bool, get_list_features
  users.rs            — User struct, create, find_by_email, find_by_id, count
  auth_methods.rs     — create, find_by_user_and_provider
  totp.rs             — upsert, find, mark_verified, delete
  server_config.rs    — get, set, is_registration_enabled
  comments.rs        — polymorphic comments CRUD (entity_type: item/list/container, author_type: user/assistant)
  relations.rs       — entity_relations CRUD, get_unresolved_blockers, bidirectional relates_to queries
  time_entries.rs    — unified time log: CRUD, inbox (unassigned), running timer query, summary per item/list/day
  test_helpers.rs     — test_pool (in-memory), create_test_user
```

### Observability — tracing throughout

All layers instrumented with `tracing`:

**db::** — `#[tracing::instrument(skip(pool))]` on all public functions. Logs query execution with user_id, list_id etc. as span fields.

**domain::** — `#[tracing::instrument(skip(pool))]` on orchestration functions. Creates parent span that nests db:: calls, giving full request→domain→db tree.

**rules::** — no tracing (pure sync functions, nanoseconds, would add noise).

**server::** — `tower_http::TraceLayer` for HTTP request/response spans. Combined with domain::/db:: spans, produces full request lifecycle tree.

**Output format per environment:**
- Dev: `tracing_subscriber::fmt().pretty()` — human readable, colored, hierarchical span output
- Prod: `tracing_subscriber::fmt().json().with_span_list(true)` — structured JSON, machine parsable, full span context

**Example dev output for a create item request:**
```
  POST /api/lists/abc/items 200 12ms
    domain::items::create{user_id="u1" list_id="abc"}
      db::lists::get_create_item_context{list_id="abc" user_id="u1"}
      db::items::insert{list_id="abc"}
```

**Filtering:** `RUST_LOG=kartoteka_server=debug,kartoteka_domain=debug,kartoteka_db=debug,tower_http=debug`

## crates/domain

### rules/ — pure validation

Zero I/O, zero async, zero dependencies beyond shared types. Testable with `#[test]`.

```
crates/domain/src/
  rules/
    mod.rs
    items.rs        — validate_features, should_auto_complete, validate_can_complete (blocker check)
    containers.rs   — validate_hierarchy, validate_move
    tags.rs         — validate_merge, validate_parent, validate_exclusive_type (priority=single), validate_location_hierarchy (country→city→address)
    lists.rs        — validate_list_type_features
    auth.rs         — determine_role, validate_password_strength
```

Examples:

```rust
// domain/src/rules/items.rs

/// Check that item's date/quantity fields are allowed by list features.
pub fn validate_features(
    features: &[String],
    has_date_fields: bool,
    has_quantity_fields: bool,
) -> Result<(), DomainError> {
    if has_date_fields && !features.iter().any(|f| f == "deadlines") {
        return Err(DomainError::FeatureRequired("deadlines"));
    }
    if has_quantity_fields && !features.iter().any(|f| f == "quantity") {
        return Err(DomainError::FeatureRequired("quantity"));
    }
    Ok(())
}

/// Determine if item should be auto-completed based on quantity.
pub fn should_auto_complete(actual_quantity: i32, target_quantity: i32) -> bool {
    actual_quantity >= target_quantity
}
```

```rust
// domain/src/rules/containers.rs

/// Parent container must be a folder (status IS NULL), not a project.
pub fn validate_hierarchy(parent_status: Option<&str>) -> Result<(), DomainError> {
    if parent_status.is_some() {
        return Err(DomainError::Validation("invalid_container_hierarchy"));
    }
    Ok(())
}
```

```rust
// domain/src/rules/auth.rs

pub fn determine_role(user_count: i64) -> &'static str {
    if user_count == 0 { "admin" } else { "user" }
}
```

### Orchestration — async, calls db::

```
crates/domain/src/
  lib.rs            — DomainError, re-exports
  items.rs          — create, update, move_item, delete, toggle_complete (blocker check)
  containers.rs     — create, update, move_container, delete, toggle_pin
  lists.rs          — create, update, reset, toggle_pin, toggle_archive, add_feature, remove_feature
  tags.rs           — create, update, merge, delete, assign/remove links
  relations.rs      — create, delete, get_for_entity (bidirectional), validate ownership
  time_entries.rs   — start_timer (auto-stop previous), stop_timer, log_manual, assign_to_item, summary
  auth.rs           — register, (future: social auth, bearer tokens)
  rules/            — (as above)
```

### Orchestration pattern

```rust
// domain/src/items.rs
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    req: &CreateItemRequest,
) -> Result<Item, DomainError> {
    // Phase 1: READ — one context query
    let ctx = db::lists::get_create_item_context(pool, list_id, user_id).await?;

    // Phase 2: THINK — pure validation
    let has_dates = req.start_date.is_some() || req.deadline.is_some() || req.hard_deadline.is_some();
    let has_quantity = req.quantity.is_some() || req.unit.is_some();
    rules::items::validate_features(&ctx.features, has_dates, has_quantity)?;

    // Phase 3: WRITE — single INSERT RETURNING
    let item = db::items::insert(pool, user_id, list_id, ctx.next_position, req).await?;
    Ok(item)
}
```

### Transaction scope

Validation OUTSIDE transaction (read-only, doesn't hold lock). Transaction only for writes, as short as possible:

```rust
// domain/src/tags.rs
pub async fn merge(
    pool: &SqlitePool,
    user_id: &str,
    source_id: &str,
    target_id: &str,
) -> Result<Tag, DomainError> {
    // Validation — before transaction, no lock held
    let source = db::tags::get_one(pool, source_id, user_id).await?
        .ok_or(DomainError::NotFound("tag"))?;
    let target = db::tags::get_one(pool, target_id, user_id).await?
        .ok_or(DomainError::NotFound("tag"))?;
    rules::tags::validate_merge(&source, &target)?;

    // Transaction — short, writes only
    let mut tx = pool.begin().await?;
    db::tags::reassign_item_links(&mut *tx, source_id, target_id).await?;
    db::tags::reassign_list_links(&mut *tx, source_id, target_id).await?;
    db::tags::reparent_children(&mut *tx, source_id, target_id).await?;
    db::tags::delete_by_id(&mut *tx, source_id).await?;
    tx.commit().await?;

    db::tags::get_one(pool, target_id, user_id).await?
        .ok_or(DomainError::NotFound("tag"))
}
```

### CPU-bound work — off tokio pool

```rust
// domain/src/auth.rs
pub async fn register(pool, email, password, name) -> Result<User, DomainError> {
    // Validation
    let enabled = db::server_config::is_registration_enabled(pool).await?;
    if !enabled { return Err(DomainError::Forbidden); }
    rules::auth::validate_password(&password)?;

    // CPU-bound: argon2 hash — off tokio thread pool
    let hash = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default().hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    }).await.map_err(|e| DomainError::Internal(e.to_string()))??;

    // DB writes
    let user_count = db::users::count(pool).await?;
    let role = rules::auth::determine_role(user_count);
    let user = db::users::create(pool, email, name, role).await?;
    db::auth_methods::create(pool, &user.id, "password", email, Some(&hash)).await?;
    Ok(user)
}
```

### Error type

```rust
// domain/src/lib.rs
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("validation: {0}")]
    Validation(&'static str),
    #[error("feature required: {0}")]
    FeatureRequired(&'static str),
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Db(#[from] kartoteka_db::DbError),
}
```

Consumed by server::AppError which maps to HTTP status codes.

### Tokio runtime summary

| Operation | Where | Why |
|-----------|-------|-----|
| db:: queries | tokio task (async) | I/O bound, non-blocking |
| rules:: validation | inline on tokio task | Sync, nanoseconds, not worth offloading |
| argon2 hash/verify | `spawn_blocking` | ~100ms CPU, would block tokio worker |
| TOTP verify | inline | ~1ms HMAC, fast enough |
| Transaction commit | tokio task | I/O, short |
| `tokio::join!` parallel reads | tokio task | Independent queries, WAL allows concurrent reads |

### Rayon

Not needed. Bottleneck is SQLite I/O, not CPU. No batch data processing or parallel transforms. `spawn_blocking` sufficient for argon2.

## Background job integration

Domain layer can enqueue jobs via `apalis::SqliteStorage` (injected alongside pool):

```rust
// domain/src/items.rs
pub async fn update(
    pool: &SqlitePool,
    job_storage: &SqliteStorage<SendNotificationJob>,
    user_id: &str,
    id: &str,
    req: &UpdateItemRequest,
) -> Result<Item, DomainError> {
    // ... validation, update ...
    
    // If deadline approaching, schedule notification
    if should_notify_deadline(&updated_item) {
        job_storage.push(SendNotificationJob { ... }).await?;
    }
    
    Ok(updated_item)
}
```

Domain functions that need job enqueueing take `&SqliteStorage<T>` as parameter alongside `&SqlitePool`. Functions that don't need it don't take it (no unnecessary coupling).

## Testing strategy

### rules/ — unit tests

Pure functions, `#[test]`, zero setup:

```rust
#[test]
fn reject_dates_without_deadline_feature() {
    let features = vec!["quantity".into()];
    assert!(rules::items::validate_features(&features, true, false).is_err());
}

#[test]
fn first_user_is_admin() {
    assert_eq!(rules::auth::determine_role(0), "admin");
    assert_eq!(rules::auth::determine_role(1), "user");
}
```

### domain:: orchestration — integration tests

In-memory SQLite via `db::test_helpers::test_pool()`. Tests business scenarios, not individual queries:

```rust
#[tokio::test]
async fn create_item_rejects_dates_without_feature() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list = create_test_list(&pool, &user_id, ListType::Checklist).await; // no deadline feature

    let req = CreateItemRequest { title: "test".into(), deadline: Some(date), ..Default::default() };
    let result = domain::items::create(&pool, &user_id, &list.id, &req).await;
    assert!(matches!(result, Err(DomainError::FeatureRequired("deadlines"))));
}

#[tokio::test]
async fn merge_tags_reassigns_links() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    // create tags, items, links...
    domain::tags::merge(&pool, &user_id, &source.id, &target.id).await.unwrap();
    // verify source deleted, links reassigned
}
```

### db:: — query tests

In-memory SQLite, verify SQL correctness (RETURNING, json_group_array, partial indexes used):

```rust
#[tokio::test]
async fn insert_returning_gives_item_back() {
    let pool = test_pool().await;
    let item = db::items::insert(&pool, ...).await.unwrap();
    assert!(!item.id.is_empty());
    assert_eq!(item.title, "test");
}
```

## Consumers

```
crates/frontend/  server functions  → domain::  (always)
crates/server/    REST handlers     → domain::  (always)
crates/mcp/       MCP tools         → domain::  (always)
```

All three use the same domain:: layer. Business logic tested once, used everywhere. db:: is an internal implementation detail of domain:: — consumers never import it.
