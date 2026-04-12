# B2: Lists Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `db::lists`, `domain::lists`, `domain::rules::lists`, and REST `/api/lists/*` endpoints so lists CRUD with features works end-to-end via curl.

**Architecture:** Follows the Read → Think → Write pattern from spec 02-db-domain.md. `db::lists` owns all SQL; `domain::lists` owns orchestration and the public `List` type; `domain::rules::lists` validates list_type/feature compatibility with pure sync functions. REST handlers in `server/src/lists.rs` use an `X-User-Id` header extractor (replaced by auth middleware in C1). Create requires a transaction (INSERT list + INSERT features atomically).

**Tech Stack:** sqlx 0.8 (sqlite), axum 0.8, serde/serde_json, uuid, thiserror, tokio

---

## File Structure

```
crates/db/Cargo.toml                        — MODIFY: add serde_json, test-helpers feature
crates/db/src/lib.rs                        — MODIFY: pub mod lists, test-helpers cfg
crates/db/src/lists.rs                      — CREATE: ListRow, FeatureRow, CreateItemContext, all queries
crates/domain/Cargo.toml                    — MODIFY: add serde, serde_json, uuid, tokio (dev)
crates/domain/src/lib.rs                    — MODIFY: DomainError, pub mod lists, pub mod rules
crates/domain/src/rules/mod.rs              — CREATE: pub mod lists
crates/domain/src/rules/lists.rs            — CREATE: validate_list_type_features + unit tests
crates/domain/src/lists.rs                  — CREATE: List, ListFeature, ListType, request types, orchestration + integration tests
crates/server/Cargo.toml                    — MODIFY: add axum, serde_json, tokio, tracing-subscriber deps
crates/server/src/lib.rs                    — MODIFY: AppState, AppError, UserId extractor, lists_router, main
crates/server/src/lists.rs                  — CREATE: REST handlers
```

---

### Task 1: db crate — Cargo.toml + lib.rs + test-helpers feature

**Files:**
- Modify: `crates/db/Cargo.toml`
- Modify: `crates/db/src/lib.rs`

- [ ] **Step 1: Add serde_json dep + test-helpers feature to db/Cargo.toml**

Replace the existing `crates/db/Cargo.toml` content:

```toml
[package]
name = "kartoteka-db"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[features]
test-helpers = []

[dependencies]
kartoteka-shared = { path = "../shared" }
sqlx.workspace = true
chrono.workspace = true
thiserror.workspace = true
uuid.workspace = true
tracing.workspace = true
serde_json = "1"

[dev-dependencies]
tokio.workspace = true
```

- [ ] **Step 2: Update db/src/lib.rs to expose lists module + test-helpers**

Replace stub content of `crates/db/src/lib.rs`:

```rust
use sqlx::SqlitePool;

pub mod lists;
pub mod test_helpers;

pub use kartoteka_shared::types::FlexDate;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub async fn create_pool(url: &str) -> Result<SqlitePool, DbError> {
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
    use std::str::FromStr;

    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    SqlitePoolOptions::new()
        .max_connections(8)
        .min_connections(2)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA busy_timeout = 5000").execute(&mut *conn).await?;
                sqlx::query("PRAGMA mmap_size = 268435456").execute(&mut *conn).await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .map_err(DbError::Sqlx)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
```

- [ ] **Step 3: Update test_helpers.rs to always compile (not just #[cfg(test)])**

Replace `crates/db/src/test_helpers.rs` content (created in A1):

```rust
//! Test helpers — compiled when running tests or when the "test-helpers" feature is enabled.
//! Use in other crates' dev-dependencies: kartoteka-db = { path = "../db", features = ["test-helpers"] }

use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

pub async fn create_test_user(pool: &SqlitePool) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (id, email, role) VALUES (?, ?, 'user')"
    )
    .bind(&id)
    .bind(format!("{}@test.com", &id[..8]))
    .execute(pool)
    .await
    .unwrap();
    id
}
```

- [ ] **Step 4: Run cargo check on db crate**

```bash
cargo check -p kartoteka-db
```

Expected: compiles with 0 errors.

- [ ] **Step 5: Commit**

```bash
git add crates/db/Cargo.toml crates/db/src/lib.rs crates/db/src/test_helpers.rs
git commit -m "feat(b2): db crate — serde_json dep, test-helpers feature, create_pool impl"
```

---

### Task 2: db::lists — row types + all queries

**Files:**
- Create: `crates/db/src/lists.rs`

- [ ] **Step 1: Write db/src/lists.rs — row types and read queries**

Create `crates/db/src/lists.rs`:

```rust
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;
use crate::DbError;

// ── Row types (internal to db crate) ────────────────────────────────────────

#[derive(sqlx::FromRow)]
pub(crate) struct ListRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub list_type: String,
    pub parent_list_id: Option<String>,
    pub position: i64,
    pub archived: i64,
    pub container_id: Option<String>,
    pub pinned: i64,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub features_json: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct FeatureRow {
    pub feature_name: String,
    pub config: String,
}

// Used by domain::items::create (B3)
#[derive(Debug)]
pub struct CreateItemContext {
    pub features: Vec<String>,
    pub next_position: i64,
}

#[derive(sqlx::FromRow)]
struct CreateItemContextRow {
    pub next_position: i64,
    pub features_json: String,
}

// ── SQL helpers ──────────────────────────────────────────────────────────────

/// Correlated subquery fragment that returns features as a JSON array.
/// Usage: embed in SELECT alongside other list columns.
const FEATURES_SUBQUERY: &str = "COALESCE(
    (SELECT json_group_array(json_object('feature_name', lf.feature_name, 'config', json(lf.config)))
     FROM list_features lf WHERE lf.list_id = l.id),
    '[]'
) as features_json";

// ── Read queries ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.user_id = ? AND l.archived = 0 AND l.parent_list_id IS NULL \
         ORDER BY l.pinned DESC, l.position",
        FEATURES_SUBQUERY
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_archived(pool: &SqlitePool, user_id: &str) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.user_id = ? AND l.archived = 1 \
         ORDER BY l.updated_at DESC",
        FEATURES_SUBQUERY
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(pool: &SqlitePool, id: &str, user_id: &str) -> Result<Option<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l WHERE l.id = ? AND l.user_id = ?",
        FEATURES_SUBQUERY
    ))
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn sublists(pool: &SqlitePool, parent_id: &str, user_id: &str) -> Result<Vec<ListRow>, DbError> {
    sqlx::query_as::<_, ListRow>(&format!(
        "SELECT l.*, {} FROM lists l \
         WHERE l.parent_list_id = ? AND l.user_id = ? \
         ORDER BY l.position",
        FEATURES_SUBQUERY
    ))
    .bind(parent_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Returns the next available position for a new list scoped to (user, container, parent).
/// Pass None for root-level lists.
#[tracing::instrument(skip(pool))]
pub async fn next_position(
    pool: &SqlitePool,
    user_id: &str,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
) -> Result<i64, DbError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM lists \
         WHERE user_id = ? AND container_id IS ? AND parent_list_id IS ?"
    )
    .bind(user_id)
    .bind(container_id)
    .bind(parent_list_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row.0)
}

/// One query: ownership check + feature names + MAX(position) for items.
/// Returns None if list not found or not owned by user_id.
#[tracing::instrument(skip(pool))]
pub async fn get_create_item_context(
    pool: &SqlitePool,
    list_id: &str,
    user_id: &str,
) -> Result<Option<CreateItemContext>, DbError> {
    let row: Option<CreateItemContextRow> = sqlx::query_as(
        "SELECT \
            COALESCE(MAX(i.position) + 1, 0) as next_position, \
            COALESCE( \
                (SELECT json_group_array(lf.feature_name) \
                 FROM list_features lf WHERE lf.list_id = l.id), \
                '[]' \
            ) as features_json \
         FROM lists l \
         LEFT JOIN items i ON i.list_id = l.id \
         WHERE l.id = ? AND l.user_id = ? \
         GROUP BY l.id"
    )
    .bind(list_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)?;

    match row {
        None => Ok(None),
        Some(r) => {
            let features: Vec<String> = serde_json::from_str(&r.features_json)?;
            Ok(Some(CreateItemContext {
                features,
                next_position: r.next_position,
            }))
        }
    }
}

// ── Write queries (called in transaction) ────────────────────────────────────

#[tracing::instrument(skip(tx))]
pub async fn insert(
    tx: &mut SqliteConnection,
    id: &str,
    user_id: &str,
    position: i64,
    name: &str,
    icon: Option<&str>,
    description: Option<&str>,
    list_type: &str,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO lists (id, user_id, name, icon, description, list_type, \
                            container_id, parent_list_id, position) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(icon)
    .bind(description)
    .bind(list_type)
    .bind(container_id)
    .bind(parent_list_id)
    .bind(position)
    .execute(&mut *tx)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

/// Replace all features for a list. Caller must be in a transaction.
#[tracing::instrument(skip(tx))]
pub async fn replace_features(
    tx: &mut SqliteConnection,
    list_id: &str,
    features: &[String],
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM list_features WHERE list_id = ?")
        .bind(list_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Sqlx)?;

    for feature in features {
        sqlx::query(
            "INSERT INTO list_features (list_id, feature_name, config) VALUES (?, ?, '{}')"
        )
        .bind(list_id)
        .bind(feature)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Sqlx)?;
    }
    Ok(())
}

// ── Write queries (no transaction needed) ────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    name: Option<&str>,
    icon: Option<Option<&str>>,       // Some(Some(v)) = set, Some(None) = clear, None = skip
    description: Option<Option<&str>>,
    list_type: Option<&str>,
) -> Result<bool, DbError> {
    // Build dynamic SET clause — only update provided fields
    let mut sets: Vec<&str> = vec!["updated_at = datetime('now')"];
    if name.is_some()        { sets.push("name = ?1"); }
    if list_type.is_some()   { sets.push("list_type = ?2"); }
    // icon and description use COALESCE-style: caller passes explicit Some/None
    // For simplicity, always set them when provided
    let sql = format!(
        "UPDATE lists SET {} WHERE id = ? AND user_id = ?",
        sets.join(", ")
    );
    // This dynamic approach gets complex — use explicit full-field update instead:
    let rows = sqlx::query(
        "UPDATE lists \
         SET name = COALESCE(?, name), \
             icon = CASE WHEN ?2 = 1 THEN ?3 ELSE icon END, \
             description = CASE WHEN ?4 = 1 THEN ?5 ELSE description END, \
             list_type = COALESCE(?, list_type), \
             updated_at = datetime('now') \
         WHERE id = ? AND user_id = ?"
    )
    .bind(name)
    .bind(icon.is_some() as i32)
    .bind(icon.and_then(|v| v))
    .bind(description.is_some() as i32)
    .bind(description.and_then(|v| v))
    .bind(list_type)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM lists WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_archived(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE lists SET archived = CASE WHEN archived = 0 THEN 1 ELSE 0 END, \
                          updated_at = datetime('now') \
         WHERE id = ? AND user_id = ?"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pinned(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE lists SET pinned = CASE WHEN pinned = 0 THEN 1 ELSE 0 END, \
                          updated_at = datetime('now') \
         WHERE id = ? AND user_id = ?"
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

/// Delete all items in a list (reset). Ownership must be verified by caller.
#[tracing::instrument(skip(pool))]
pub async fn delete_items(pool: &SqlitePool, list_id: &str) -> Result<u64, DbError> {
    let rows = sqlx::query("DELETE FROM items WHERE list_id = ?")
        .bind(list_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected())
}

#[tracing::instrument(skip(pool))]
pub async fn move_list(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    position: i64,
    container_id: Option<&str>,
    parent_list_id: Option<&str>,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE lists SET position = ?, container_id = ?, parent_list_id = ?, \
                          updated_at = datetime('now') \
         WHERE id = ? AND user_id = ?"
    )
    .bind(position)
    .bind(container_id)
    .bind(parent_list_id)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

// ── db-level tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};

    async fn insert_test_list(pool: &SqlitePool, user_id: &str, name: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let mut tx = pool.begin().await.unwrap();
        insert(&mut tx, &id, user_id, 0, name, None, None, "checklist", None, None)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        id
    }

    #[tokio::test]
    async fn insert_and_get_one() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "My List").await;

        let row = get_one(&pool, &id, &user_id).await.unwrap().unwrap();
        assert_eq!(row.name, "My List");
        assert_eq!(row.list_type, "checklist");
        assert_eq!(row.archived, 0);
        assert_eq!(row.pinned, 0);
        assert_eq!(row.features_json, "[]");
    }

    #[tokio::test]
    async fn get_one_wrong_user_returns_none() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let other_user = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "Private").await;

        let row = get_one(&pool, &id, &other_user).await.unwrap();
        assert!(row.is_none());
    }

    #[tokio::test]
    async fn list_all_excludes_archived() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "Active").await;
        insert_test_list(&pool, &user_id, "Active2").await;
        toggle_archived(&pool, &id, &user_id).await.unwrap();

        let rows = list_all(&pool, &user_id).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Active2");
    }

    #[tokio::test]
    async fn features_roundtrip() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "With Features").await;

        let mut tx = pool.begin().await.unwrap();
        replace_features(&mut tx, &id, &["deadlines".into(), "quantity".into()])
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let row = get_one(&pool, &id, &user_id).await.unwrap().unwrap();
        let features: Vec<serde_json::Value> = serde_json::from_str(&row.features_json).unwrap();
        assert_eq!(features.len(), 2);
        let names: Vec<&str> = features.iter()
            .map(|f| f["feature_name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"deadlines"));
        assert!(names.contains(&"quantity"));
    }

    #[tokio::test]
    async fn toggle_archived_flips() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "List").await;

        toggle_archived(&pool, &id, &user_id).await.unwrap();
        let row = get_one(&pool, &id, &user_id).await.unwrap().unwrap();
        assert_eq!(row.archived, 1);

        toggle_archived(&pool, &id, &user_id).await.unwrap();
        let row = get_one(&pool, &id, &user_id).await.unwrap().unwrap();
        assert_eq!(row.archived, 0);
    }

    #[tokio::test]
    async fn get_create_item_context_no_items() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "Empty List").await;

        let mut tx = pool.begin().await.unwrap();
        replace_features(&mut tx, &id, &["deadlines".into()]).await.unwrap();
        tx.commit().await.unwrap();

        let ctx = get_create_item_context(&pool, &id, &user_id).await.unwrap().unwrap();
        assert_eq!(ctx.next_position, 0);
        assert_eq!(ctx.features, vec!["deadlines"]);
    }

    #[tokio::test]
    async fn get_create_item_context_wrong_user_returns_none() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "List").await;

        let ctx = get_create_item_context(&pool, &id, &other).await.unwrap();
        assert!(ctx.is_none());
    }

    #[tokio::test]
    async fn delete_removes_list() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let id = insert_test_list(&pool, &user_id, "Doomed").await;

        let deleted = delete(&pool, &id, &user_id).await.unwrap();
        assert!(deleted);
        assert!(get_one(&pool, &id, &user_id).await.unwrap().is_none());
    }
}
```

- [ ] **Step 2: Run db tests**

```bash
cargo test -p kartoteka-db -- lists 2>&1 | head -60
```

Expected: all `lists::tests::*` pass.

- [ ] **Step 3: Commit**

```bash
git add crates/db/src/lists.rs crates/db/src/lib.rs
git commit -m "feat(b2): db::lists — CRUD, features, toggle, move, get_create_item_context"
```

---

### Task 3: domain — DomainError + rules module scaffold

**Files:**
- Modify: `crates/domain/Cargo.toml`
- Modify: `crates/domain/src/lib.rs`
- Create: `crates/domain/src/rules/mod.rs`

- [ ] **Step 1: Update domain/Cargo.toml**

```toml
[package]
name = "kartoteka-domain"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-db = { path = "../db" }
thiserror.workspace = true
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
tracing.workspace = true

[dev-dependencies]
tokio.workspace = true
kartoteka-db = { path = "../db", features = ["test-helpers"] }
```

- [ ] **Step 2: Write domain/src/lib.rs with DomainError**

```rust
pub mod lists;
pub mod rules;

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

- [ ] **Step 3: Create domain/src/rules/mod.rs**

```rust
pub mod lists;
```

- [ ] **Step 4: cargo check**

```bash
cargo check -p kartoteka-domain
```

Expected: 0 errors (lists.rs and rules/lists.rs don't exist yet — add empty files to satisfy mod declarations):

Create placeholder `crates/domain/src/lists.rs`:
```rust
// implementation in Task 5
```

Create placeholder `crates/domain/src/rules/lists.rs`:
```rust
// implementation in Task 4
```

Re-run: `cargo check -p kartoteka-domain` → 0 errors.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/Cargo.toml crates/domain/src/lib.rs crates/domain/src/rules/mod.rs crates/domain/src/lists.rs crates/domain/src/rules/lists.rs
git commit -m "feat(b2): domain crate — DomainError, rules mod scaffold"
```

---

### Task 4: domain::rules::lists — validate_list_type_features

**Files:**
- Modify: `crates/domain/src/rules/lists.rs`

- [ ] **Step 1: Write failing tests first**

Replace `crates/domain/src/rules/lists.rs`:

```rust
use crate::DomainError;

/// Validate that the given features are compatible with the list_type.
///
/// Rules:
/// - `shopping`  → requires "quantity" feature (items need qty/unit fields)
/// - `habits`    → requires "deadlines" feature (items need date scheduling)
/// - `checklist` → any features allowed
/// - `log`       → any features allowed
/// - unknown     → rejected
pub fn validate_list_type_features(
    list_type: &str,
    features: &[String],
) -> Result<(), DomainError> {
    match list_type {
        "shopping" => {
            if !features.iter().any(|f| f == "quantity") {
                return Err(DomainError::Validation("shopping_lists_require_quantity"));
            }
        }
        "habits" => {
            if !features.iter().any(|f| f == "deadlines") {
                return Err(DomainError::Validation("habits_lists_require_deadlines"));
            }
        }
        "checklist" | "log" => {}
        _ => return Err(DomainError::Validation("unknown_list_type")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn checklist_allows_no_features() {
        assert!(validate_list_type_features("checklist", &[]).is_ok());
    }

    #[test]
    fn checklist_allows_any_features() {
        assert!(validate_list_type_features("checklist", &features(&["deadlines", "quantity"])).is_ok());
    }

    #[test]
    fn log_allows_any_features() {
        assert!(validate_list_type_features("log", &features(&["deadlines"])).is_ok());
    }

    #[test]
    fn shopping_requires_quantity() {
        let err = validate_list_type_features("shopping", &[]).unwrap_err();
        assert!(matches!(err, DomainError::Validation("shopping_lists_require_quantity")));
    }

    #[test]
    fn shopping_ok_with_quantity() {
        assert!(validate_list_type_features("shopping", &features(&["quantity"])).is_ok());
    }

    #[test]
    fn shopping_ok_with_quantity_and_more() {
        assert!(validate_list_type_features("shopping", &features(&["quantity", "deadlines"])).is_ok());
    }

    #[test]
    fn habits_requires_deadlines() {
        let err = validate_list_type_features("habits", &[]).unwrap_err();
        assert!(matches!(err, DomainError::Validation("habits_lists_require_deadlines")));
    }

    #[test]
    fn habits_ok_with_deadlines() {
        assert!(validate_list_type_features("habits", &features(&["deadlines"])).is_ok());
    }

    #[test]
    fn unknown_type_rejects() {
        let err = validate_list_type_features("kanban", &[]).unwrap_err();
        assert!(matches!(err, DomainError::Validation("unknown_list_type")));
    }
}
```

- [ ] **Step 2: Run rules tests**

```bash
cargo test -p kartoteka-domain rules::lists 2>&1
```

Expected: all 9 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/domain/src/rules/lists.rs
git commit -m "feat(b2): domain::rules::lists — validate_list_type_features"
```

---

### Task 5: domain::lists — List type + orchestration + integration tests

**Files:**
- Modify: `crates/domain/src/lists.rs`

- [ ] **Step 1: Write failing integration tests**

Replace `crates/domain/src/lists.rs` with tests first, then implementation:

```rust
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;
use crate::{rules, DomainError};
use kartoteka_db as db;

// ── Public domain types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    Checklist,
    Shopping,
    Habits,
    Log,
}

impl ListType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ListType::Checklist => "checklist",
            ListType::Shopping  => "shopping",
            ListType::Habits    => "habits",
            ListType::Log       => "log",
        }
    }
}

impl TryFrom<&str> for ListType {
    type Error = DomainError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "checklist" => Ok(ListType::Checklist),
            "shopping"  => Ok(ListType::Shopping),
            "habits"    => Ok(ListType::Habits),
            "log"       => Ok(ListType::Log),
            _           => Err(DomainError::Validation("unknown_list_type")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFeature {
    pub feature_name: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub list_type: String,
    pub parent_list_id: Option<String>,
    pub position: i64,
    pub archived: bool,
    pub container_id: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub features: Vec<ListFeature>,
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: Option<String>,   // defaults to "checklist"
    pub icon: Option<String>,
    pub description: Option<String>,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
    pub features: Vec<String>,       // feature_name strings
}

#[derive(Debug, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub icon: Option<Option<String>>,         // Some(None) clears the field
    pub description: Option<Option<String>>,
    pub list_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveListRequest {
    pub position: i64,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetFeaturesRequest {
    pub features: Vec<String>,
}

// ── Conversion from db row ────────────────────────────────────────────────────

fn row_to_list(row: db::lists::ListRow) -> Result<List, DomainError> {
    let features: Vec<ListFeature> = serde_json::from_str(&row.features_json)
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(List {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        icon: row.icon,
        description: row.description,
        list_type: row.list_type,
        parent_list_id: row.parent_list_id,
        position: row.position,
        archived: row.archived != 0,
        container_id: row.container_id,
        pinned: row.pinned != 0,
        last_opened_at: row.last_opened_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
        features,
    })
}

// ── Orchestration ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<List>, DomainError> {
    let rows = db::lists::list_all(pool, user_id).await?;
    rows.into_iter().map(row_to_list).collect()
}

#[tracing::instrument(skip(pool))]
pub async fn list_archived(pool: &SqlitePool, user_id: &str) -> Result<Vec<List>, DomainError> {
    let rows = db::lists::list_archived(pool, user_id).await?;
    rows.into_iter().map(row_to_list).collect()
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(pool: &SqlitePool, id: &str, user_id: &str) -> Result<Option<List>, DomainError> {
    db::lists::get_one(pool, id, user_id).await?
        .map(row_to_list)
        .transpose()
}

#[tracing::instrument(skip(pool))]
pub async fn sublists(pool: &SqlitePool, parent_id: &str, user_id: &str) -> Result<Vec<List>, DomainError> {
    let rows = db::lists::sublists(pool, parent_id, user_id).await?;
    rows.into_iter().map(row_to_list).collect()
}

#[tracing::instrument(skip(pool))]
pub async fn create(pool: &SqlitePool, user_id: &str, req: &CreateListRequest) -> Result<List, DomainError> {
    let list_type = req.list_type.as_deref().unwrap_or("checklist");

    // Phase 2: THINK
    rules::lists::validate_list_type_features(list_type, &req.features)?;

    // Phase 1: READ (position — before transaction, doesn't hold write lock)
    let position = db::lists::next_position(
        pool,
        user_id,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    ).await?;

    // Phase 3: WRITE
    let list_id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await?;
    db::lists::insert(
        &mut *tx,
        &list_id,
        user_id,
        position,
        &req.name,
        req.icon.as_deref(),
        req.description.as_deref(),
        list_type,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    ).await?;
    if !req.features.is_empty() {
        db::lists::replace_features(&mut *tx, &list_id, &req.features).await?;
    }
    tx.commit().await?;

    db::lists::get_one(pool, &list_id, user_id).await?
        .map(row_to_list)
        .transpose()?
        .ok_or_else(|| DomainError::Internal("list disappeared after create".into()))
}

#[tracing::instrument(skip(pool))]
pub async fn update(pool: &SqlitePool, id: &str, user_id: &str, req: &UpdateListRequest) -> Result<Option<List>, DomainError> {
    // Phase 1: READ — need current list to validate type change
    let current = match db::lists::get_one(pool, id, user_id).await? {
        Some(l) => l,
        None => return Ok(None),
    };

    // Phase 2: THINK — validate new list_type against existing features
    if let Some(new_type) = req.list_type.as_deref() {
        let features: Vec<ListFeature> = serde_json::from_str(&current.features_json)
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        let feature_names: Vec<String> = features.iter().map(|f| f.feature_name.clone()).collect();
        rules::lists::validate_list_type_features(new_type, &feature_names)?;
    }

    // Phase 3: WRITE
    let updated = db::lists::update(
        pool,
        id,
        user_id,
        req.name.as_deref(),
        req.icon.as_ref().map(|v| v.as_deref()),
        req.description.as_ref().map(|v| v.as_deref()),
        req.list_type.as_deref(),
    ).await?;

    if !updated {
        return Ok(None);
    }

    db::lists::get_one(pool, id, user_id).await?
        .map(row_to_list)
        .transpose()
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DomainError> {
    Ok(db::lists::delete(pool, id, user_id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_archive(pool: &SqlitePool, id: &str, user_id: &str) -> Result<Option<List>, DomainError> {
    let toggled = db::lists::toggle_archived(pool, id, user_id).await?;
    if !toggled {
        return Ok(None);
    }
    db::lists::get_one(pool, id, user_id).await?
        .map(row_to_list)
        .transpose()
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(pool: &SqlitePool, id: &str, user_id: &str) -> Result<Option<List>, DomainError> {
    let toggled = db::lists::toggle_pinned(pool, id, user_id).await?;
    if !toggled {
        return Ok(None);
    }
    db::lists::get_one(pool, id, user_id).await?
        .map(row_to_list)
        .transpose()
}

/// Delete all items in the list (keep the list itself).
#[tracing::instrument(skip(pool))]
pub async fn reset(pool: &SqlitePool, id: &str, user_id: &str) -> Result<u64, DomainError> {
    // Verify ownership
    db::lists::get_one(pool, id, user_id).await?
        .ok_or(DomainError::NotFound("list"))?;
    Ok(db::lists::delete_items(pool, id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn move_list(pool: &SqlitePool, id: &str, user_id: &str, req: &MoveListRequest) -> Result<Option<List>, DomainError> {
    let moved = db::lists::move_list(
        pool, id, user_id,
        req.position,
        req.container_id.as_deref(),
        req.parent_list_id.as_deref(),
    ).await?;
    if !moved {
        return Ok(None);
    }
    db::lists::get_one(pool, id, user_id).await?
        .map(row_to_list)
        .transpose()
}

/// Replace all features for a list (validates against current list_type).
#[tracing::instrument(skip(pool))]
pub async fn set_features(pool: &SqlitePool, id: &str, user_id: &str, req: &SetFeaturesRequest) -> Result<Option<List>, DomainError> {
    // Phase 1: READ
    let current = match db::lists::get_one(pool, id, user_id).await? {
        Some(l) => l,
        None => return Ok(None),
    };

    // Phase 2: THINK
    rules::lists::validate_list_type_features(&current.list_type, &req.features)?;

    // Phase 3: WRITE
    let mut tx = pool.begin().await?;
    db::lists::replace_features(&mut *tx, id, &req.features).await?;
    tx.commit().await?;

    db::lists::get_one(pool, id, user_id).await?
        .map(row_to_list)
        .transpose()
}

// Also re-export CreateItemContext so server/mcp don't need to import db directly
pub use db::lists::CreateItemContext;

pub async fn get_create_item_context(
    pool: &SqlitePool,
    list_id: &str,
    user_id: &str,
) -> Result<Option<CreateItemContext>, DomainError> {
    Ok(db::lists::get_create_item_context(pool, list_id, user_id).await?)
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    fn checklist_req(name: &str) -> CreateListRequest {
        CreateListRequest {
            name: name.to_string(),
            list_type: Some("checklist".into()),
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            features: vec![],
        }
    }

    #[tokio::test]
    async fn create_checklist_no_features() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Todo")).await.unwrap();

        assert_eq!(list.name, "Todo");
        assert_eq!(list.list_type, "checklist");
        assert!(list.features.is_empty());
        assert!(!list.archived);
        assert!(!list.pinned);
    }

    #[tokio::test]
    async fn create_with_features_roundtrip() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Deadlined".into(),
            list_type: Some("checklist".into()),
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            features: vec!["deadlines".into(), "quantity".into()],
        };
        let list = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(list.features.len(), 2);
        let names: Vec<&str> = list.features.iter().map(|f| f.feature_name.as_str()).collect();
        assert!(names.contains(&"deadlines"));
        assert!(names.contains(&"quantity"));
    }

    #[tokio::test]
    async fn create_shopping_requires_quantity() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Groceries".into(),
            list_type: Some("shopping".into()),
            features: vec![],
            icon: None, description: None, container_id: None, parent_list_id: None,
        };
        let err = create(&pool, &uid, &req).await.unwrap_err();
        assert!(matches!(err, DomainError::Validation("shopping_lists_require_quantity")));
    }

    #[tokio::test]
    async fn create_shopping_with_quantity_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Groceries".into(),
            list_type: Some("shopping".into()),
            features: vec!["quantity".into()],
            icon: None, description: None, container_id: None, parent_list_id: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();
        assert_eq!(list.list_type, "shopping");
        assert_eq!(list.features.len(), 1);
    }

    #[tokio::test]
    async fn toggle_archive_flips_and_returns_updated() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Archivable")).await.unwrap();

        let updated = toggle_archive(&pool, &list.id, &uid).await.unwrap().unwrap();
        assert!(updated.archived);

        let updated2 = toggle_archive(&pool, &list.id, &uid).await.unwrap().unwrap();
        assert!(!updated2.archived);
    }

    #[tokio::test]
    async fn reset_deletes_items_not_list() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Resettable")).await.unwrap();

        // Insert 2 items directly
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES ('i1', ?, 'A'), ('i2', ?, 'B')")
            .bind(&list.id)
            .bind(&list.id)
            .execute(&pool)
            .await
            .unwrap();

        let deleted = reset(&pool, &list.id, &uid).await.unwrap();
        assert_eq!(deleted, 2);

        // List still exists
        let found = get_one(&pool, &list.id, &uid).await.unwrap();
        assert!(found.is_some());

        // Items gone
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE list_id = ?")
            .bind(&list.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn reset_wrong_user_returns_not_found() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let list = create(&pool, &uid, &checklist_req("Mine")).await.unwrap();

        let err = reset(&pool, &list.id, &other).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound("list")));
    }

    #[tokio::test]
    async fn set_features_validates_against_list_type() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateListRequest {
            name: "Shop".into(),
            list_type: Some("shopping".into()),
            features: vec!["quantity".into()],
            icon: None, description: None, container_id: None, parent_list_id: None,
        };
        let list = create(&pool, &uid, &req).await.unwrap();

        // Removing quantity from a shopping list → invalid
        let err = set_features(&pool, &list.id, &uid, &SetFeaturesRequest {
            features: vec!["deadlines".into()], // no quantity!
        }).await.unwrap_err();
        assert!(matches!(err, DomainError::Validation("shopping_lists_require_quantity")));
    }

    #[tokio::test]
    async fn list_all_excludes_archived() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let l1 = create(&pool, &uid, &checklist_req("Active")).await.unwrap();
        create(&pool, &uid, &checklist_req("Active2")).await.unwrap();
        toggle_archive(&pool, &l1.id, &uid).await.unwrap();

        let lists = list_all(&pool, &uid).await.unwrap();
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].name, "Active2");
    }

    #[tokio::test]
    async fn positions_increment_on_create() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let l1 = create(&pool, &uid, &checklist_req("First")).await.unwrap();
        let l2 = create(&pool, &uid, &checklist_req("Second")).await.unwrap();
        assert_eq!(l1.position, 0);
        assert_eq!(l2.position, 1);
    }
}
```

- [ ] **Step 2: Run domain integration tests**

```bash
cargo test -p kartoteka-domain lists 2>&1 | tail -30
```

Expected: all `lists::tests::*` pass.

- [ ] **Step 3: Commit**

```bash
git add crates/domain/src/lists.rs
git commit -m "feat(b2): domain::lists — List type, orchestration, integration tests"
```

---

### Task 6: server — AppState + REST handlers

**Files:**
- Modify: `crates/server/Cargo.toml`
- Modify: `crates/server/src/lib.rs`
- Create: `crates/server/src/lists.rs`

- [ ] **Step 1: Update server/Cargo.toml**

```toml
[package]
name = "kartoteka-server"
version.workspace = true
edition.workspace = true
publish = false

[[bin]]
name = "kartoteka"
path = "src/main.rs"

[lints]
workspace = true

[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-db = { path = "../db" }
kartoteka-domain = { path = "../domain" }
kartoteka-auth = { path = "../auth" }
kartoteka-mcp = { path = "../mcp" }
kartoteka-oauth = { path = "../oauth" }
kartoteka-jobs = { path = "../jobs" }
kartoteka-frontend-v2 = { path = "../frontend-v2" }
axum = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio.workspace = true
tracing.workspace = true
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror.workspace = true
```

- [ ] **Step 2: Create server/src/lib.rs with AppState, AppError, UserId extractor**

Replace stub `crates/server/src/lib.rs`:

```rust
pub mod lists;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use sqlx::SqlitePool;

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

// ── UserId extractor (dev: reads X-User-Id header; replaced by auth in C1) ───

#[derive(Clone, Debug)]
pub struct UserId(pub String);

impl<S> FromRequestParts<S> for UserId
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| UserId(s.to_string()))
            .ok_or(AppError::Unauthorized)
    }
}

// ── AppError ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("internal: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::NotFound       => (StatusCode::NOT_FOUND,            "not found".to_string()),
            AppError::BadRequest(m)  => (StatusCode::BAD_REQUEST,          m.clone()),
            AppError::Unauthorized   => (StatusCode::UNAUTHORIZED,         "missing X-User-Id header".to_string()),
            AppError::Internal(m)    => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        (status, msg).into_response()
    }
}

impl From<kartoteka_domain::DomainError> for AppError {
    fn from(e: kartoteka_domain::DomainError) -> Self {
        use kartoteka_domain::DomainError::*;
        match e {
            NotFound(_)          => AppError::NotFound,
            Validation(msg)      => AppError::BadRequest(msg.to_string()),
            FeatureRequired(f)   => AppError::BadRequest(format!("feature required: {f}")),
            Forbidden            => AppError::BadRequest("forbidden".into()),
            Internal(msg)        => AppError::Internal(msg),
            Db(e)                => AppError::Internal(e.to_string()),
        }
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn api_router() -> axum::Router<AppState> {
    // B1 will add containers_router() here; B2 adds lists_router()
    lists::lists_router()
}
```

- [ ] **Step 3: Create server/src/lists.rs — REST handlers**

Create `crates/server/src/lists.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use kartoteka_domain::lists::{
    CreateListRequest, MoveListRequest, SetFeaturesRequest, UpdateListRequest,
};
use crate::{AppError, AppState, UserId};

pub fn lists_router() -> Router<AppState> {
    Router::new()
        .route("/lists",               get(list_all).post(create))
        .route("/lists/archived",      get(list_archived))
        .route("/lists/{id}",          get(get_one).put(update).delete(delete_list))
        .route("/lists/{id}/sublists", get(sublists))
        .route("/lists/{id}/archive",  post(toggle_archive))
        .route("/lists/{id}/pin",      post(toggle_pin))
        .route("/lists/{id}/reset",    post(reset))
        .route("/lists/{id}/move",     post(move_list))
        .route("/lists/{id}/features", put(set_features))
}

async fn list_all(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::list_all(&state.pool, &uid).await?;
    Ok(Json(lists))
}

async fn list_archived(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::list_archived(&state.pool, &uid).await?;
    Ok(Json(lists))
}

async fn get_one(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::get_one(&state.pool, &id, &uid)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn sublists(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::sublists(&state.pool, &id, &uid).await?;
    Ok(Json(lists))
}

async fn create(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<CreateListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let list = kartoteka_domain::lists::create(&state.pool, &uid, &req).await?;
    Ok((StatusCode::CREATED, Json(list)))
}

async fn update(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateListRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::update(&state.pool, &id, &uid, &req)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn delete_list(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = kartoteka_domain::lists::delete(&state.pool, &id, &uid).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound)
    }
}

async fn toggle_archive(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::toggle_archive(&state.pool, &id, &uid)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn toggle_pin(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::toggle_pin(&state.pool, &id, &uid)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn reset(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = kartoteka_domain::lists::reset(&state.pool, &id, &uid).await?;
    Ok(Json(serde_json::json!({ "deleted_items": deleted })))
}

async fn move_list(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<MoveListRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::move_list(&state.pool, &id, &uid, &req)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn set_features(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<SetFeaturesRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::set_features(&state.pool, &id, &uid, &req)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound)
}
```

- [ ] **Step 4: Create server/src/main.rs**

Create `crates/server/src/main.rs`:

```rust
use axum::Router;
use kartoteka_server::{api_router, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kartoteka_server=debug,kartoteka_domain=debug,kartoteka_db=debug,tower_http=debug".into())
        )
        .pretty()
        .init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data.db".into());
    let pool = kartoteka_db::create_pool(&db_url).await.expect("db connect");
    kartoteka_db::run_migrations(&pool).await.expect("migrations");

    let state = AppState { pool };
    let app = Router::new()
        .nest("/api", api_router())
        .with_state(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 5: cargo check server**

```bash
cargo check -p kartoteka-server
```

Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add crates/server/Cargo.toml crates/server/src/lib.rs crates/server/src/lists.rs crates/server/src/main.rs
git commit -m "feat(b2): server — AppState, AppError, UserId extractor, lists REST handlers"
```

---

### Task 7: Full workspace check + smoke test

**Files:** none (verification only)

- [ ] **Step 1: Run full workspace tests**

```bash
cargo test --workspace 2>&1 | tail -40
```

Expected: all `db::lists::tests::*` and `domain::lists::tests::*` and `domain::rules::lists::tests::*` pass. No failures.

- [ ] **Step 2: Build the server binary**

```bash
cargo build -p kartoteka-server 2>&1
```

Expected: compiles cleanly. Binary at `target/debug/kartoteka`.

- [ ] **Step 3: Smoke test with curl**

In one terminal:
```bash
DATABASE_URL=sqlite:///tmp/test-b2.db RUST_LOG=debug ./target/debug/kartoteka
```

In another terminal:

```bash
# Create a user directly in DB (no auth yet)
sqlite3 /tmp/test-b2.db "INSERT INTO users (id, email, role) VALUES ('u1', 'test@test.com', 'user')"

# Create a checklist
curl -s -X POST http://localhost:3000/api/lists \
  -H "Content-Type: application/json" \
  -H "X-User-Id: u1" \
  -d '{"name":"My Todo","list_type":"checklist","features":[]}' | jq .

# Create a shopping list with quantity feature
curl -s -X POST http://localhost:3000/api/lists \
  -H "Content-Type: application/json" \
  -H "X-User-Id: u1" \
  -d '{"name":"Groceries","list_type":"shopping","features":["quantity"]}' | jq .

# Fail: shopping list without quantity feature
curl -s -X POST http://localhost:3000/api/lists \
  -H "Content-Type: application/json" \
  -H "X-User-Id: u1" \
  -d '{"name":"Bad Shop","list_type":"shopping","features":[]}' -w "\nHTTP %{http_code}\n"
# Expected: HTTP 400

# List all
curl -s http://localhost:3000/api/lists -H "X-User-Id: u1" | jq 'length'
# Expected: 2

# Get one (use ID from create response)
LIST_ID=$(curl -s http://localhost:3000/api/lists -H "X-User-Id: u1" | jq -r '.[0].id')
curl -s http://localhost:3000/api/lists/$LIST_ID -H "X-User-Id: u1" | jq .name

# Toggle archive
curl -s -X POST http://localhost:3000/api/lists/$LIST_ID/archive -H "X-User-Id: u1" | jq .archived
# Expected: true

# Check archived endpoint
curl -s http://localhost:3000/api/lists/archived -H "X-User-Id: u1" | jq 'length'
# Expected: 1

# Toggle archive back
curl -s -X POST http://localhost:3000/api/lists/$LIST_ID/archive -H "X-User-Id: u1" | jq .archived
# Expected: false

# Set features
curl -s -X PUT http://localhost:3000/api/lists/$LIST_ID/features \
  -H "Content-Type: application/json" \
  -H "X-User-Id: u1" \
  -d '{"features":["deadlines"]}' | jq '.features | length'
# Expected: 1

# Delete
curl -s -X DELETE http://localhost:3000/api/lists/$LIST_ID -H "X-User-Id: u1" -w "%{http_code}"
# Expected: 204
```

- [ ] **Step 4: Run cargo clippy**

```bash
cargo clippy -p kartoteka-db -p kartoteka-domain -p kartoteka-server -- -D warnings 2>&1
```

Fix any warnings before proceeding.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat(b2): lists CRUD complete — db, domain, rules, REST handlers, tests"
```

---

## Self-Review

### Spec coverage

| Spec requirement | Task |
|---|---|
| `db::lists` — CRUD | Task 2 |
| `db::lists` — sublists | Task 2 (`sublists` fn) |
| `db::lists` — features | Task 2 (`replace_features`, `features_json`) |
| `db::lists` — archive, pin | Task 2 (`toggle_archived`, `toggle_pinned`) |
| `db::lists` — reset | Task 2 (`delete_items`) |
| `db::lists` — move | Task 2 (`move_list`) |
| `db::lists` — `get_create_item_context` | Task 2 |
| `domain::lists` — create with features (transaction) | Task 5 (`create` fn) |
| `domain::lists` — reset | Task 5 (`reset` fn) |
| `domain::lists` — toggle archive/pin | Task 5 |
| `domain::rules::lists` — `validate_list_type_features` | Task 4 |
| REST `/api/lists/*` | Task 6 |
| Tests: db + domain + feature slice validation | Tasks 2, 4, 5 |
| Deliverable: curl lists CRUD with features | Task 7 |

### Notes for B1 merge

When B1 (Containers+Home) is merged with B2, update `server/src/lib.rs`:

```rust
pub fn api_router() -> axum::Router<AppState> {
    containers::containers_router()   // from B1
        .merge(lists::lists_router()) // from B2
}
```

B1 and B2 both create `server/src/lib.rs` — coordinate the merge before committing to main.

---

**Plan complete and saved to `docs/superpowers/plans/B2-lists.md`.**

**Two execution options:**

**1. Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
