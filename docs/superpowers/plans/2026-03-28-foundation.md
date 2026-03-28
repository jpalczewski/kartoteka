# Foundation Implementation Plan (Plan 1 of 5)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the foundational Cargo workspace with models, database layer, and Axum API server — replacing Cloudflare Workers + D1 with native Rust + SQLite.

**Architecture:** Three new crates (`kartoteka-models`, `kartoteka-db`, `kartoteka-server`) in the existing workspace. Models are pure types, DB handles all sqlx queries, Server is a thin Axum layer. Old Cloudflare crates (`api`, `shared`) remain untouched — new crates coexist until migration is complete.

**Tech Stack:** Rust, Axum 0.8, sqlx 0.8 (SQLite), serde, uuid, tokio, tower-http, tracing

**Spec:** `docs/superpowers/specs/2026-03-28-cloudflare-exit-rewrite-design.md`

---

## File Structure

```
crates/
  models/
    Cargo.toml
    src/lib.rs              — re-exports all modules
    src/container.rs        — Container, ContainerDetail, CreateContainerRequest, etc.
    src/list.rs             — List, ListFeature, CreateListRequest, etc.
    src/item.rs             — Item, DateItem, DaySummary, DayItems, CreateItemRequest, etc.
    src/tag.rs              — Tag, CreateTagRequest, ItemTagLink, etc.
    src/enums.rs            — ContainerStatus, ListType, DateField
    src/home.rs             — HomeItem, HomeData
  db/
    Cargo.toml
    src/lib.rs              — pool helper, re-exports
    src/containers.rs       — container queries
    src/lists.rs            — list queries
    src/items.rs            — item queries (including by_date, calendar)
    src/tags.rs             — tag queries (including merge, cycle detection)
    src/home.rs             — home dashboard query
    migrations/
      0001_initial.sql      — full schema (consolidated from 10 D1 migrations)
  server/
    Cargo.toml
    src/main.rs             — Axum entry point
    src/error.rs            — AppError type + IntoResponse
    src/extractors.rs       — UserId extractor (from header for now, auth in Plan 2)
    src/routes/mod.rs       — route tree
    src/routes/containers.rs
    src/routes/lists.rs
    src/routes/items.rs
    src/routes/tags.rs
    src/routes/home.rs
```

---

### Task 1: Workspace Scaffolding

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/models/Cargo.toml`
- Create: `crates/models/src/lib.rs`
- Create: `crates/db/Cargo.toml`
- Create: `crates/db/src/lib.rs`
- Create: `crates/server/Cargo.toml`
- Create: `crates/server/src/main.rs`

- [ ] **Step 1: Add new workspace members to root Cargo.toml**

Add to the `[workspace] members` array:
```toml
"crates/models",
"crates/db",
"crates/server",
```

- [ ] **Step 2: Create `crates/models/Cargo.toml`**

```toml
[package]
name = "kartoteka-models"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
sqlx = { version = "0.8", features = ["sqlite"], default-features = false }
```

- [ ] **Step 3: Create `crates/db/Cargo.toml`**

```toml
[package]
name = "kartoteka-db"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-models = { path = "../models" }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
```

- [ ] **Step 4: Create `crates/server/Cargo.toml`**

```toml
[package]
name = "kartoteka-server"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[dependencies]
kartoteka-models = { path = "../models" }
kartoteka-db = { path = "../db" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace", "fs"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
uuid = { version = "1", features = ["v4"] }
anyhow = "1"

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
```

- [ ] **Step 5: Create stub files**

`crates/models/src/lib.rs`:
```rust
pub mod container;
pub mod enums;
pub mod home;
pub mod item;
pub mod list;
pub mod tag;
```

`crates/db/src/lib.rs`:
```rust
use sqlx::SqlitePool;

pub mod containers;
pub mod home;
pub mod items;
pub mod lists;
pub mod tags;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePool::connect(database_url).await?;
    sqlx::query("PRAGMA journal_mode=WAL").execute(&pool).await?;
    sqlx::query("PRAGMA busy_timeout=5000").execute(&pool).await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
```

`crates/server/src/main.rs`:
```rust
fn main() {
    println!("kartoteka-server stub");
}
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p kartoteka-models -p kartoteka-db -p kartoteka-server`
Expected: compiles (with warnings about empty modules)

- [ ] **Step 7: Commit**

```bash
git add crates/models crates/db crates/server Cargo.toml
git commit -m "feat: scaffold new workspace crates (models, db, server)"
```

---

### Task 2: Models Crate

**Files:**
- Create: `crates/models/src/enums.rs`
- Create: `crates/models/src/container.rs`
- Create: `crates/models/src/list.rs`
- Create: `crates/models/src/item.rs`
- Create: `crates/models/src/tag.rs`
- Create: `crates/models/src/home.rs`

All types are extracted from `crates/shared/src/lib.rs`, cleaned up:
- Remove `bool_from_number`, `u32_from_number` custom deserializers (sqlx maps correctly)
- Remove `features_from_json` deserializer (handle in db layer)
- Add `sqlx::FromRow` derives where needed
- Add `schemars::JsonSchema` for MCP tool params (Plan 3)

- [ ] **Step 1: Create `crates/models/src/enums.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ContainerStatus {
    Active,
    Done,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ListType {
    Checklist,
    Zakupy,
    Pakowanie,
    Terminarz,
    Custom,
}

impl Default for ListType {
    fn default() -> Self {
        Self::Checklist
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateField {
    StartDate,
    Deadline,
    HardDeadline,
}

impl DateField {
    pub fn column_name(&self) -> &'static str {
        match self {
            Self::StartDate => "start_date",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }
}
```

- [ ] **Step 2: Create `crates/models/src/container.rs`**

```rust
use serde::{Deserialize, Serialize};
use crate::enums::ContainerStatus;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Container {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
    pub position: i32,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDetail {
    #[serde(flatten)]
    pub container: Container,
    pub completed_items: i64,
    pub total_items: i64,
    pub completed_lists: i64,
    pub total_lists: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContainerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<Option<ContainerStatus>>,
}

#[derive(Debug, Deserialize)]
pub struct MoveContainerRequest {
    pub parent_container_id: Option<String>,
}
```

- [ ] **Step 3: Create `crates/models/src/list.rs`**

```rust
use serde::{Deserialize, Serialize};
use crate::enums::ListType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFeature {
    pub name: String,
    pub config: serde_json::Value,
}

pub const FEATURE_QUANTITY: &str = "quantity";
pub const FEATURE_DEADLINES: &str = "deadlines";

/// List as returned from the database. The `features` field requires
/// special handling (JSON aggregate from a subquery), so we don't derive
/// FromRow here — the db layer constructs this manually.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub list_type: ListType,
    pub parent_list_id: Option<String>,
    pub position: i32,
    pub archived: bool,
    pub features: Vec<ListFeature>,
    pub container_id: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Raw row from database before features are parsed.
#[derive(Debug, sqlx::FromRow)]
pub struct ListRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub list_type: ListType,
    pub parent_list_id: Option<String>,
    pub position: i32,
    pub archived: bool,
    pub features: String, // JSON string from json_group_array
    pub container_id: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ListRow {
    pub fn into_list(self) -> List {
        let features: Vec<ListFeature> = serde_json::from_str(&self.features)
            .unwrap_or_default();
        List {
            id: self.id,
            user_id: self.user_id,
            name: self.name,
            description: self.description,
            list_type: self.list_type,
            parent_list_id: self.parent_list_id,
            position: self.position,
            archived: self.archived,
            features,
            container_id: self.container_id,
            pinned: self.pinned,
            last_opened_at: self.last_opened_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    #[serde(default)]
    pub list_type: ListType,
    pub features: Option<Vec<ListFeature>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub list_type: Option<ListType>,
    pub archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct FeatureConfigRequest {
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MoveListRequest {
    pub container_id: Option<String>,
}
```

- [ ] **Step 4: Create `crates/models/src/item.rs`**

```rust
use serde::{Deserialize, Serialize};
use crate::enums::ListType;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Item {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DateItem {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub list_name: String,
    pub list_type: ListType,
    pub date_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummary {
    pub date: String,
    pub total: i64,
    pub completed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayItems {
    pub date: String,
    pub items: Vec<DateItem>,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub title: String,
    pub description: Option<String>,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
    pub position: Option<i32>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<Option<String>>,
    pub start_time: Option<Option<String>>,
    pub deadline: Option<Option<String>>,
    pub deadline_time: Option<Option<String>>,
    pub hard_deadline: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct MoveItemRequest {
    pub list_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ByDateQuery {
    pub date: String,
    pub field: Option<String>,
    pub include_overdue: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub from: String,
    pub to: String,
    pub field: Option<String>,
    pub mode: Option<String>, // "counts" or "full"
}
```

- [ ] **Step 5: Create `crates/models/src/tag.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct MergeTagRequest {
    pub target_tag_id: String,
}

#[derive(Debug, Deserialize)]
pub struct TagAssignment {
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ItemTagLink {
    pub item_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ListTagLink {
    pub list_id: String,
    pub tag_id: String,
}
```

- [ ] **Step 6: Create `crates/models/src/home.rs`**

```rust
use serde::{Deserialize, Serialize};
use crate::container::Container;
use crate::enums::ContainerStatus;
use crate::list::List;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeItem {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
    pub list_type: Option<String>,
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeData {
    pub pinned: Vec<HomeItem>,
    pub recent: Vec<HomeItem>,
    pub root_containers: Vec<Container>,
    pub root_lists: Vec<List>,
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check -p kartoteka-models`
Expected: compiles cleanly

- [ ] **Step 8: Commit**

```bash
git add crates/models/
git commit -m "feat(models): add all domain types for the new stack"
```

---

### Task 3: Database — Setup + Migration

**Files:**
- Create: `crates/db/migrations/0001_initial.sql`
- Modify: `crates/db/src/lib.rs`

- [ ] **Step 1: Create consolidated migration**

Create `crates/db/migrations/0001_initial.sql` with the full schema (consolidated from the 10 existing D1 migrations). See spec for complete schema. Include all tables: `containers`, `lists`, `items`, `tags`, `item_tags`, `list_tags`, `list_features`, plus indices.

Include all relevant CREATE INDEX statements from the original D1 migrations at `crates/api/migrations/`. Key indices: `items(list_id)`, `lists(user_id)`, `lists(container_id)`, `containers(user_id)`, `containers(parent_container_id)`, `tags(user_id)`, `item_tags(item_id)`, `item_tags(tag_id)`, `list_tags(list_id)`, `list_tags(tag_id)`, `items(start_date)`, `items(deadline)`, `items(hard_deadline)`.

- [ ] **Step 2: Write a test for pool creation + migration**

In `crates/db/src/lib.rs`, add:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_and_migration() {
        let pool = create_pool("sqlite::memory:").await.unwrap();
        migrate(&pool).await.unwrap();

        // Verify tables exist
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='lists'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(result.0, 1);
    }
}
```

- [ ] **Step 3: Run test**

Run: `cargo test -p kartoteka-db`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/db/
git commit -m "feat(db): add SQLite migration and pool setup"
```

---

### Task 4: Database — Container Queries

**Files:**
- Create: `crates/db/src/containers.rs`

Implement all container queries as `pub async fn` functions taking `&SqlitePool` and returning `Result<T, sqlx::Error>`. All SQL queries are taken verbatim from `crates/api/src/handlers/containers.rs`.

Functions to implement:
- `list_all(pool, user_id) -> Vec<Container>`
- `create(pool, user_id, req) -> Container`
- `get_one(pool, user_id, id) -> Option<ContainerDetail>`
- `update(pool, user_id, id, req) -> Option<Container>`
- `delete(pool, user_id, id) -> bool`
- `get_children(pool, user_id, id) -> (Vec<Container>, Vec<List>)`
- `move_container(pool, user_id, id, req) -> Result<(), AppError>` (with cycle detection)
- `toggle_pin(pool, user_id, id) -> Option<Container>`

Each function uses `sqlx::query_as` for typed results. Ownership verification is done inside each function (return None/false if not owned).

Include validation logic:
- create: projects cannot have sub-containers as children
- move: WITH RECURSIVE cycle prevention query

- [ ] **Step 1: Implement container queries**

Write all functions. Use `uuid::Uuid::new_v4().to_string()` for ID generation.

- [ ] **Step 2: Write tests**

Test at minimum: `create` + `list_all`, `get_one`, `update`, `delete`, ownership isolation (user A can't see user B's containers).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_pool, migrate};

    async fn setup() -> SqlitePool {
        let pool = create_pool("sqlite::memory:").await.unwrap();
        migrate(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let pool = setup().await;
        let req = CreateContainerRequest { name: "Test".into(), status: None, parent_container_id: None };
        let c = create(&pool, "user1", &req).await.unwrap();
        assert_eq!(c.name, "Test");

        let all = list_all(&pool, "user1").await.unwrap();
        assert_eq!(all.len(), 1);

        // Different user sees nothing
        let other = list_all(&pool, "user2").await.unwrap();
        assert!(other.is_empty());
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kartoteka-db -- containers`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/db/src/containers.rs
git commit -m "feat(db): add container queries with tests"
```

---

### Task 5: Database — List Queries

**Files:**
- Create: `crates/db/src/lists.rs`

Functions to implement:
- `list_all(pool, user_id) -> Vec<List>` (root lists, not archived, no container)
- `list_archived(pool, user_id) -> Vec<List>`
- `create(pool, user_id, req) -> List` (with default features based on list_type)
- `get_one(pool, user_id, id) -> Option<List>` (updates last_opened_at)
- `update(pool, user_id, id, req) -> Option<List>`
- `delete(pool, user_id, id) -> bool`
- `list_sublists(pool, user_id, parent_id) -> Vec<List>`
- `create_sublist(pool, user_id, parent_id, req) -> List`
- `toggle_archive(pool, user_id, id) -> Option<List>`
- `reset(pool, user_id, id) -> bool`
- `add_feature(pool, user_id, list_id, name, config) -> bool`
- `remove_feature(pool, user_id, list_id, name) -> bool`
- `move_list(pool, user_id, id, container_id) -> Option<List>`
- `toggle_pin(pool, user_id, id) -> Option<List>`
- `get_children_lists(pool, container_id) -> Vec<List>` (used by containers::get_children)

Key: The `LIST_SELECT` constant includes a `json_group_array` subquery for features. Use `ListRow` + `into_list()` conversion.

- [ ] **Step 1: Implement list queries**
- [ ] **Step 2: Write tests** (create, list, features, sublists, archive)
- [ ] **Step 3: Run tests**: `cargo test -p kartoteka-db -- lists`
- [ ] **Step 4: Commit**

```bash
git add crates/db/src/lists.rs
git commit -m "feat(db): add list queries with tests"
```

---

### Task 6: Database — Item Queries

**Files:**
- Create: `crates/db/src/items.rs`

Functions to implement:
- `list_all(pool, user_id, list_id) -> Vec<Item>`
- `create(pool, user_id, list_id, req) -> Item`
- `update(pool, user_id, id, req) -> Option<Item>` (with auto-complete logic)
- `delete(pool, user_id, id) -> bool`
- `move_item(pool, user_id, id, target_list_id) -> Option<Item>`
- `by_date(pool, user_id, query) -> Vec<DateItem>` (UNION ALL queries)
- `calendar_counts(pool, user_id, query) -> Vec<DaySummary>` (GROUP BY date)
- `calendar_full(pool, user_id, query) -> Vec<DateItem>` (UNION ALL with date range, grouped into DayItems by handler)

Complex queries:
- `by_date` with all fields: 3-way UNION ALL across start_date, deadline, hard_deadline with overdue support
- `by_date` with single field: simpler query with optional overdue
- `calendar` full mode: UNION ALL with date range
- `calendar` counts mode: GROUP BY date with COUNT/SUM
- `update` auto-complete: when `actual_quantity >= quantity`, set `completed = true`
- `update` nullable date fields: `Option<Option<String>>` — None means skip, Some(None) means set NULL, Some(Some(v)) means set value

Note: Dynamic SQL (column names in format!) is needed for `by_date` and `calendar` with single field. Use validated enum values, never user input directly.

- [ ] **Step 1: Implement item queries**
- [ ] **Step 2: Write tests** (CRUD, auto-complete, by_date, calendar counts)
- [ ] **Step 3: Run tests**: `cargo test -p kartoteka-db -- items`
- [ ] **Step 4: Commit**

```bash
git add crates/db/src/items.rs
git commit -m "feat(db): add item queries with calendar support"
```

---

### Task 7: Database — Tag Queries

**Files:**
- Create: `crates/db/src/tags.rs`

Functions to implement:
- `list_all(pool, user_id) -> Vec<Tag>`
- `create(pool, user_id, req) -> Tag`
- `update(pool, user_id, id, req) -> Option<Tag>` (with cycle detection)
- `delete(pool, user_id, id) -> bool`
- `merge(pool, user_id, source_id, target_id) -> Option<Tag>`
- `assign_to_item(pool, user_id, item_id, tag_id) -> bool`
- `remove_from_item(pool, user_id, item_id, tag_id) -> bool`
- `assign_to_list(pool, user_id, list_id, tag_id) -> bool`
- `remove_from_list(pool, user_id, list_id, tag_id) -> bool`
- `tag_items(pool, user_id, tag_id, recursive) -> Vec<Item>` (WITH RECURSIVE for recursive mode)
- `all_item_tag_links(pool, user_id) -> Vec<ItemTagLink>`
- `all_list_tag_links(pool, user_id) -> Vec<ListTagLink>`

Complex queries:
- update parent: WITH RECURSIVE cycle detection
- merge: INSERT OR IGNORE to move item_tags and list_tags, reparent children, delete source

- [ ] **Step 1: Implement tag queries**
- [ ] **Step 2: Write tests** (CRUD, cycle detection, merge, tag assignment)
- [ ] **Step 3: Run tests**: `cargo test -p kartoteka-db -- tags`
- [ ] **Step 4: Commit**

```bash
git add crates/db/src/tags.rs
git commit -m "feat(db): add tag queries with merge and cycle detection"
```

---

### Task 8: Database — Home Query

**Files:**
- Create: `crates/db/src/home.rs`

Function:
- `get_home(pool, user_id) -> HomeData`

This combines multiple queries:
1. Pinned lists (non-archived, root)
2. Pinned containers
3. Recent lists (by last_opened_at, limit 5)
4. Recent containers (by last_opened_at, limit 5)
5. Root containers (no parent)
6. Root lists (no container, no parent, not archived)

Build `HomeData` with `pinned` (merged lists+containers as `HomeItem`), `recent` (merged), `root_containers`, `root_lists`.

- [ ] **Step 1: Implement home query**
- [ ] **Step 2: Write test**
- [ ] **Step 3: Run tests**: `cargo test -p kartoteka-db -- home`
- [ ] **Step 4: Commit**

```bash
git add crates/db/src/home.rs
git commit -m "feat(db): add home dashboard query"
```

---

### Task 9: Server — Entry Point + Error Handling

**Files:**
- Create: `crates/server/src/error.rs`
- Create: `crates/server/src/extractors.rs`
- Create: `crates/server/src/routes/mod.rs`
- Modify: `crates/server/src/main.rs`

- [ ] **Step 1: Create error type**

`crates/server/src/error.rs`:
```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub enum AppError {
    NotFound,
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!("Database error: {e}");
        Self::Internal("Database error".to_string())
    }
}
```

- [ ] **Step 2: Create UserId extractor**

`crates/server/src/extractors.rs`:
```rust
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use crate::error::AppError;

/// Extracts user ID. For now from X-User-Id header (dev mode).
/// Plan 2 replaces this with session-based extraction.
pub struct UserId(pub String);

impl<S: Send + Sync> FromRequestParts<S> for UserId {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.headers.get("X-User-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| UserId(s.to_string()))
            .ok_or(AppError::BadRequest("Missing X-User-Id header".to_string()))
    }
}
```

- [ ] **Step 3: Create route tree stub**

`crates/server/src/routes/mod.rs`:
```rust
use axum::Router;
use sqlx::SqlitePool;

pub mod containers;
pub mod home;
pub mod items;
pub mod lists;
pub mod tags;

pub fn api_routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(containers::routes())
        .merge(lists::routes())
        .merge(items::routes())
        .merge(tags::routes())
        .merge(home::routes())
        .with_state(pool)
}
```

- [ ] **Step 4: Write main.rs**

```rust
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod error;
mod extractors;
mod routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://data.db?mode=rwc".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let pool = kartoteka_db::create_pool(&db_url).await?;
    kartoteka_db::migrate(&pool).await?;

    let app = axum::Router::new()
        .nest("/api", routes::api_routes(pool))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

Add `anyhow = "1"` to `crates/server/Cargo.toml`.

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p kartoteka-server`
Expected: compiles (route modules will be stubs)

- [ ] **Step 6: Commit**

```bash
git add crates/server/
git commit -m "feat(server): add Axum entry point with error handling"
```

---

### Task 10: Server Routes — Containers + Lists

**Files:**
- Create: `crates/server/src/routes/containers.rs`
- Create: `crates/server/src/routes/lists.rs`
- Create: `crates/server/src/routes/home.rs`

Pattern for all route handlers:
```rust
use axum::{extract::{Path, State}, Json, Router, routing::{get, post, put, delete, patch}};
use sqlx::SqlitePool;
use crate::{error::AppError, extractors::UserId};

pub fn routes() -> Router<SqlitePool> {
    Router::new()
        .route("/containers", get(list_all).post(create))
        .route("/containers/{id}", get(get_one).put(update).delete(delete_one))
        // ... etc
}

async fn list_all(
    State(pool): State<SqlitePool>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<Container>>, AppError> {
    let containers = kartoteka_db::containers::list_all(&pool, &user_id).await?;
    Ok(Json(containers))
}
```

Each handler: extract State + UserId + Path/Json → call db function → return Json or AppError.

Container routes (9 endpoints):
- `GET /containers` → `list_all`
- `POST /containers` → `create`
- `GET /containers/:id` → `get_one`
- `PUT /containers/:id` → `update`
- `DELETE /containers/:id` → `delete`
- `GET /containers/:id/children` → `get_children`
- `PATCH /containers/:id/move` → `move_container`
- `PATCH /containers/:id/pin` → `toggle_pin`

List routes (14 endpoints):
- `GET /lists` → `list_all`
- `POST /lists` → `create`
- `GET /lists/:id` → `get_one`
- `PUT /lists/:id` → `update`
- `DELETE /lists/:id` → `delete`
- `GET /lists/archived` → `list_archived`
- `PATCH /lists/:id/archive` → `toggle_archive`
- `POST /lists/:id/reset` → `reset`
- `GET /lists/:id/sublists` → `list_sublists`
- `POST /lists/:id/sublists` → `create_sublist`
- `POST /lists/:id/features/:name` → `add_feature`
- `DELETE /lists/:id/features/:name` → `remove_feature`
- `PATCH /lists/:id/container` → `move_list`
- `PATCH /lists/:id/pin` → `toggle_pin`

Home route:
- `GET /home` → `get_home`

- [ ] **Step 1: Implement container routes**
- [ ] **Step 2: Implement list routes**
- [ ] **Step 3: Implement home route**
- [ ] **Step 4: Verify compilation**: `cargo check -p kartoteka-server`
- [ ] **Step 5: Commit**

```bash
git add crates/server/src/routes/
git commit -m "feat(server): add container, list, and home routes"
```

---

### Task 11: Server Routes — Items + Tags

**Files:**
- Create: `crates/server/src/routes/items.rs`
- Create: `crates/server/src/routes/tags.rs`

Item routes (8 endpoints):
- `GET /lists/:list_id/items` → `list_all`
- `POST /lists/:list_id/items` → `create`
- `PUT /lists/:list_id/items/:id` → `update`
- `DELETE /lists/:list_id/items/:id` → `delete`
- `PATCH /items/:id/move` → `move_item`
- `GET /items/by-date` → `by_date` (query params: date, field, include_overdue)
- `GET /items/calendar` → `calendar` (query params: from, to, field, mode)

Tag routes (11 endpoints):
- `GET /tags` → `list_all`
- `POST /tags` → `create`
- `PUT /tags/:id` → `update`
- `DELETE /tags/:id` → `delete`
- `POST /tags/:id/merge` → `merge`
- `POST /items/:item_id/tags` → `assign_to_item`
- `DELETE /items/:item_id/tags/:tag_id` → `remove_from_item`
- `POST /lists/:list_id/tags` → `assign_to_list`
- `DELETE /lists/:list_id/tags/:tag_id` → `remove_from_list`
- `GET /tags/{id}/items` → `tag_items` (query param: recursive)
- `GET /tag-links/items` → `all_item_tag_links`
- `GET /tag-links/lists` → `all_list_tag_links`

- [ ] **Step 1: Implement item routes** (including calendar/by-date with query param extraction)
- [ ] **Step 2: Implement tag routes**
- [ ] **Step 3: Verify compilation**: `cargo check -p kartoteka-server`
- [ ] **Step 4: Commit**

```bash
git add crates/server/src/routes/
git commit -m "feat(server): add item and tag routes"
```

---

### Task 12: Integration Smoke Test

**Files:**
- Create: `crates/server/tests/smoke.rs`

- [ ] **Step 1: Write integration test**

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

async fn app() -> axum::Router {
    let pool = kartoteka_db::create_pool("sqlite::memory:").await.unwrap();
    kartoteka_db::migrate(&pool).await.unwrap();
    // Build the router same as main.rs
    kartoteka_server::routes::api_routes(pool)
}

#[tokio::test]
async fn test_full_crud_flow() {
    let app = app().await;

    // Create a list
    let resp = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/lists")
            .header("Content-Type", "application/json")
            .header("X-User-Id", "test-user")
            .body(Body::from(r#"{"name":"Groceries","list_type":"zakupy"}"#))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // List should appear
    let resp = app.clone().oneshot(
        Request::builder()
            .uri("/lists")
            .header("X-User-Id", "test-user")
            .body(Body::empty())
            .unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Add an item
    // ... similar pattern
}

#[tokio::test]
async fn test_user_isolation() {
    // User A creates data, User B can't see it
}
```

Note: For this to work, the route builder function needs to be `pub`. Adjust `crates/server/src/routes/mod.rs` to export `api_routes` and make the server a library+binary crate:

In `crates/server/Cargo.toml`, add:
```toml
[[bin]]
name = "kartoteka-server"
path = "src/main.rs"

[lib]
name = "kartoteka_server"
path = "src/lib.rs"
```

Create `crates/server/src/lib.rs`:
```rust
pub mod error;
pub mod extractors;
pub mod routes;
```

- [ ] **Step 2: Run integration test**

Run: `cargo test -p kartoteka-server --test smoke`
Expected: PASS

- [ ] **Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass (including old crates)

- [ ] **Step 4: Manual smoke test**

```bash
DATABASE_URL="sqlite://test.db?mode=rwc" cargo run -p kartoteka-server &
sleep 1
curl -s localhost:3000/api/lists -H "X-User-Id: dev" | jq .
curl -s -X POST localhost:3000/api/lists -H "X-User-Id: dev" -H "Content-Type: application/json" -d '{"name":"Test","list_type":"checklist"}' | jq .
kill %1
rm test.db
```

- [ ] **Step 5: Final commit**

```bash
git add crates/server/
git commit -m "feat(server): add integration smoke test"
```

---

## Completion Criteria

After all tasks:
- `cargo check --workspace` passes
- `cargo test --workspace` passes
- `cargo run -p kartoteka-server` starts and serves the API on port 3000
- All 45 API endpoints respond correctly with `X-User-Id` header auth (including health)
- SQLite database created automatically on first run
- Same endpoint paths and response shapes as the existing Cloudflare Workers API
- Old crates (`api`, `shared`, `frontend`) still compile (untouched)

## Next Plans

- **Plan 2: Auth** — Replace `X-User-Id` header with GitHub OAuth + tower-sessions
- **Plan 3: MCP** — Add rmcp server + oxide-auth OAuth provider
- **Plan 4: Frontend** — Adjust Leptos CSR for new backend
- **Plan 5: Deploy** — Mikrus setup + GitHub Actions CI/CD
