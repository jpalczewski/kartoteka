# B1: Containers + Home — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `db::containers`, `db::home`, `domain::rules::containers`, `domain::containers`, `domain::home`, and REST endpoints `/api/containers/*` + `/api/home` — so `curl containers CRUD` works without auth.

**Architecture:** Three-layer pipeline: db:: (pure SQLite queries, RETURNING, ContainerRow) → domain:: (orchestration, validation, Container domain type) → server:: (Axum REST handlers, X-User-Id header as placeholder auth). All reads/writes go through domain::; db:: is never called directly from server::. tokio::join! parallelizes the 3-query home aggregation.

**Tech Stack:** sqlx 0.8 (sqlite), axum 0.8, tokio 1, tracing 0.1, serde 1, uuid 1, thiserror 2. Rust 2024 edition (async fn in traits stable — no `#[async_trait]`).

---

## File Structure

```
crates/shared/src/types.rs         — MODIFY: add Container, request types, ContainerProgress, HomeData
crates/db/src/lib.rs               — MODIFY: add pub mod containers, home
crates/db/src/containers.rs        — CREATE: all container queries + ContainerProgressRow
crates/db/src/home.rs              — CREATE: ContainerHomeData + parallel query via tokio::join!
crates/domain/src/lib.rs           — MODIFY: add pub mod containers, home, rules
crates/domain/src/rules/mod.rs     — CREATE: pub mod containers
crates/domain/src/rules/containers.rs — CREATE: validate_hierarchy, validate_move (pure, sync)
crates/domain/src/containers.rs    — CREATE: Container type, From<ContainerRow>, orchestration fns
crates/domain/src/home.rs          — CREATE: HomeData + pass-through query()
Cargo.toml (root)                  — MODIFY: add axum, tracing-subscriber to workspace.dependencies
crates/server/Cargo.toml           — MODIFY: add axum, tracing-subscriber to [dependencies]
crates/server/src/lib.rs           — MODIFY: AppState, router() fn
crates/server/src/main.rs          — CREATE: tokio::main entry point
crates/server/src/error.rs         — CREATE: AppError + IntoResponse + From<DomainError>
crates/server/src/extractors.rs    — CREATE: UserId extractor (reads X-User-Id header)
crates/server/src/routes/mod.rs    — CREATE: routes() fn
crates/server/src/routes/containers.rs — CREATE: REST handlers for /api/containers/*
crates/server/src/routes/home.rs   — CREATE: REST handler for GET /api/home
```

**Key design decisions:**
- New rewrite types live in `shared::types` (distinct from old Cloudflare Workers types in `shared::` root). No conflict — old code uses `kartoteka_shared::Container`, new code uses `kartoteka_shared::types::Container`.
- `db::containers::insert` accepts `&CreateContainerRequest` (from `shared::types`) — db/ can import from shared/ without circular deps.
- `ContainerProgressRow` is a private query-result struct in `db/src/containers.rs` (not a table row, not in types.rs).
- `next_position` is a private helper in `db/src/containers.rs`.
- `HomeData` in B1 is container-only; B2 extends it with list fields.
- `UserId` extractor reads `X-User-Id` header — replaced by real auth middleware in C1.

---

### Task 1: Shared types — Container, requests, HomeData

**Files:**
- Modify: `crates/shared/src/types.rs`

- [ ] **Step 1: Add Container domain type and request types to shared/src/types.rs**

Append to the end of `crates/shared/src/types.rs` (after the existing `sqlx_impl` module):

```rust
// =====================================================================
// Rewrite domain types — new SQLite-based API
// These live in shared::types (not shared:: root) to avoid conflict
// with the Cloudflare Workers types in shared/src/lib.rs
// =====================================================================

use serde::{Deserialize, Serialize};

/// Domain type for a container (folder or project).
/// status = None → folder; status = Some("active"|"done"|"paused") → project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// None = folder; Some("active"|"done"|"paused"|...) = project
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
    pub position: i32,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// None = folder. Some("active") = project with status.
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateContainerRequest {
    /// None = no change
    pub name: Option<String>,
    /// None = no change; Some(None) = clear; Some(Some(v)) = set
    pub icon: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub status: Option<Option<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoveContainerRequest {
    /// None = move to root; Some(id) = move under that parent
    pub parent_container_id: Option<String>,
    /// None = append to end (server computes next_position)
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerProgress {
    pub total_lists: i64,
    pub total_items: i64,
    pub completed_items: i64,
}

/// Home page data (container-only in B1; B2 adds list fields).
#[derive(Debug, Clone, Serialize)]
pub struct HomeData {
    pub pinned_containers: Vec<Container>,
    pub recent_containers: Vec<Container>,
    pub root_containers: Vec<Container>,
}
```

- [ ] **Step 2: Verify shared crate still compiles**

```bash
cd /path/to/repo
cargo check -p kartoteka-shared
```

Expected: no errors. The new types are additive — existing code unchanged.

- [ ] **Step 3: Commit**

```bash
git add crates/shared/src/types.rs
git commit -m "feat(shared): add Container domain types for rewrite"
```

---

### Task 2: db::containers — all query functions

**Files:**
- Create: `crates/db/src/containers.rs`
- Modify: `crates/db/src/lib.rs`

- [ ] **Step 1: Write failing tests for db::containers**

Create `crates/db/src/containers.rs` with only the test module (functions not yet implemented):

```rust
use crate::{DbError, SqlitePool};
use kartoteka_shared::types::{
    CreateContainerRequest, MoveContainerRequest, UpdateContainerRequest,
};
use crate::types::ContainerRow;
use uuid::Uuid;

// ── private helpers ────────────────────────────────────────────────

async fn next_position(
    pool: &SqlitePool,
    user_id: &str,
    parent_id: Option<&str>,
) -> Result<i32, DbError> {
    todo!()
}

// ── public query functions ─────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<ContainerRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn insert(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateContainerRequest,
    position: i32,
) -> Result<ContainerRow, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateContainerRequest,
) -> Result<Option<ContainerRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn children(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<ContainerRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn root(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn pinned(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn recent(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<ContainerRow>, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn move_container(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    parent_id: Option<&str>,
    position: i32,
) -> Result<Option<ContainerRow>, DbError> {
    todo!()
}

/// Returns true if `possible_descendant_id` is a descendant (or equal to) `ancestor_id`.
/// Used by domain to detect circular moves.
#[tracing::instrument(skip(pool))]
pub async fn is_descendant(
    pool: &SqlitePool,
    user_id: &str,
    ancestor_id: &str,
    possible_descendant_id: &str,
) -> Result<bool, DbError> {
    todo!()
}

#[derive(Debug, sqlx::FromRow)]
pub struct ContainerProgressRow {
    pub total_lists: i64,
    pub total_items: i64,
    pub completed_items: i64,
}

#[tracing::instrument(skip(pool))]
pub async fn progress(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<ContainerProgressRow, DbError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn touch_last_opened(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<(), DbError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn insert_and_get_one() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "My Container".into(),
            icon: Some("📦".into()),
            description: None,
            status: None,
            parent_container_id: None,
        };
        let row = insert(&pool, &uid, &req, 0).await.unwrap();
        assert_eq!(row.name, "My Container");
        assert_eq!(row.icon.as_deref(), Some("📦"));
        assert!(!row.id.is_empty());
        assert_eq!(row.position, 0);
        assert!(!row.pinned);

        let fetched = get_one(&pool, &row.id, &uid).await.unwrap();
        assert_eq!(fetched.unwrap().id, row.id);
    }

    #[tokio::test]
    async fn get_one_wrong_user_returns_none() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "Mine".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let row = insert(&pool, &uid, &req, 0).await.unwrap();
        let result = get_one(&pool, &row.id, &other).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn list_all_returns_only_own() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "Mine".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        insert(&pool, &uid, &req, 0).await.unwrap();
        insert(&pool, &other, &req, 0).await.unwrap();

        let all = list_all(&pool, &uid).await.unwrap();
        assert_eq!(all.len(), 1);
        assert!(all.iter().all(|c| c.user_id == uid));
    }

    #[tokio::test]
    async fn update_patches_fields() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "Old".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let row = insert(&pool, &uid, &req, 0).await.unwrap();

        let upd = UpdateContainerRequest {
            name: Some("New".into()),
            icon: Some(Some("🗂".into())),
            description: Some(Some("desc".into())),
            status: None,
        };
        let updated = update(&pool, &row.id, &uid, &upd).await.unwrap().unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.icon.as_deref(), Some("🗂"));
        assert_eq!(updated.description.as_deref(), Some("desc"));
        assert!(updated.status.is_none());
    }

    #[tokio::test]
    async fn update_clears_nullable_field() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "X".into(),
            icon: Some("🗂".into()),
            description: None,
            status: None,
            parent_container_id: None,
        };
        let row = insert(&pool, &uid, &req, 0).await.unwrap();

        let upd = UpdateContainerRequest {
            name: None,
            icon: Some(None), // clear icon
            description: None,
            status: None,
        };
        let updated = update(&pool, &row.id, &uid, &upd).await.unwrap().unwrap();
        assert!(updated.icon.is_none());
    }

    #[tokio::test]
    async fn delete_removes_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "ToDelete".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let row = insert(&pool, &uid, &req, 0).await.unwrap();
        let deleted = delete(&pool, &row.id, &uid).await.unwrap();
        assert!(deleted);
        let fetched = get_one(&pool, &row.id, &uid).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_wrong_user_returns_false() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "X".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let row = insert(&pool, &uid, &req, 0).await.unwrap();
        let deleted = delete(&pool, &row.id, &other).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn children_returns_direct_children_only() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent_req = CreateContainerRequest {
            name: "Parent".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let parent = insert(&pool, &uid, &parent_req, 0).await.unwrap();

        let child_req = CreateContainerRequest {
            name: "Child".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: Some(parent.id.clone()),
        };
        let child = insert(&pool, &uid, &child_req, 0).await.unwrap();

        // Grandchild should NOT appear in parent's children
        let grandchild_req = CreateContainerRequest {
            name: "Grandchild".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: Some(child.id.clone()),
        };
        insert(&pool, &uid, &grandchild_req, 0).await.unwrap();

        let kids = children(&pool, &parent.id, &uid).await.unwrap();
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].id, child.id);
    }

    #[tokio::test]
    async fn root_returns_top_level_only() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root_req = CreateContainerRequest {
            name: "Root".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let root_container = insert(&pool, &uid, &root_req, 0).await.unwrap();

        let child_req = CreateContainerRequest {
            name: "Child".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: Some(root_container.id.clone()),
        };
        insert(&pool, &uid, &child_req, 0).await.unwrap();

        let roots = root(&pool, &uid).await.unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, root_container.id);
    }

    #[tokio::test]
    async fn pinned_returns_only_pinned() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "X".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let c = insert(&pool, &uid, &req, 0).await.unwrap();
        assert!(pinned(&pool, &uid).await.unwrap().is_empty());

        toggle_pin(&pool, &c.id, &uid).await.unwrap().unwrap();
        let p = pinned(&pool, &uid).await.unwrap();
        assert_eq!(p.len(), 1);
        assert!(p[0].pinned);
    }

    #[tokio::test]
    async fn toggle_pin_flips_state() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "X".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let c = insert(&pool, &uid, &req, 0).await.unwrap();
        assert!(!c.pinned);

        let after_pin = toggle_pin(&pool, &c.id, &uid).await.unwrap().unwrap();
        assert!(after_pin.pinned);

        let after_unpin = toggle_pin(&pool, &c.id, &uid).await.unwrap().unwrap();
        assert!(!after_unpin.pinned);
    }

    #[tokio::test]
    async fn move_container_updates_parent_and_position() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "X".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let parent = insert(&pool, &uid, &req, 0).await.unwrap();
        let child = insert(&pool, &uid, &CreateContainerRequest {
            name: "Child".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        }, 0).await.unwrap();

        let moved = move_container(&pool, &child.id, &uid, Some(&parent.id), 5)
            .await.unwrap().unwrap();
        assert_eq!(moved.parent_container_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(moved.position, 5);
    }

    #[tokio::test]
    async fn is_descendant_detects_cycle() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "A".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let a = insert(&pool, &uid, &req, 0).await.unwrap();
        let b = insert(&pool, &uid, &CreateContainerRequest {
            name: "B".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: Some(a.id.clone()),
        }, 0).await.unwrap();

        // B is a descendant of A
        assert!(is_descendant(&pool, &uid, &a.id, &b.id).await.unwrap());
        // A is NOT a descendant of B
        assert!(!is_descendant(&pool, &uid, &b.id, &a.id).await.unwrap());
        // A is a descendant of itself (prevents self-move)
        assert!(is_descendant(&pool, &uid, &a.id, &a.id).await.unwrap());
    }

    #[tokio::test]
    async fn progress_counts_lists_and_items() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "P".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let c = insert(&pool, &uid, &req, 0).await.unwrap();

        // Insert a list in this container
        let list_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO lists (id, user_id, name, container_id) VALUES (?, ?, 'L1', ?)")
            .bind(&list_id).bind(&uid).bind(&c.id)
            .execute(&pool).await.unwrap();
        // Insert two items, one completed
        sqlx::query("INSERT INTO items (id, list_id, title, completed) VALUES (?, ?, 'i1', 0)")
            .bind(Uuid::new_v4().to_string()).bind(&list_id)
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, completed) VALUES (?, ?, 'i2', 1)")
            .bind(Uuid::new_v4().to_string()).bind(&list_id)
            .execute(&pool).await.unwrap();

        let p = progress(&pool, &c.id, &uid).await.unwrap();
        assert_eq!(p.total_lists, 1);
        assert_eq!(p.total_items, 2);
        assert_eq!(p.completed_items, 1);
    }

    #[tokio::test]
    async fn next_position_increments() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        assert_eq!(next_position(&pool, &uid, None).await.unwrap(), 0);

        let req = CreateContainerRequest {
            name: "X".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let pos = next_position(&pool, &uid, None).await.unwrap();
        insert(&pool, &uid, &req, pos).await.unwrap();
        let pos2 = next_position(&pool, &uid, None).await.unwrap();
        assert_eq!(pos2, 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail with "not yet implemented"**

```bash
cargo test -p kartoteka-db containers 2>&1 | head -30
```

Expected: all tests FAIL with `called \`Option::unwrap()\` on a \`None\` value` or panicked at 'not yet implemented'.

- [ ] **Step 3: Add pub mod containers to db/src/lib.rs**

In `crates/db/src/lib.rs`, after the existing `pub mod test_helpers;` and `pub mod types;`:

```rust
pub mod containers;
```

- [ ] **Step 4: Implement all functions in db/src/containers.rs**

Replace all `todo!()` bodies with:

```rust
async fn next_position(
    pool: &SqlitePool,
    user_id: &str,
    parent_id: Option<&str>,
) -> Result<i32, DbError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM containers WHERE user_id = ? AND parent_container_id IS ?"
    )
    .bind(user_id)
    .bind(parent_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row.0 as i32)
}

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    sqlx::query_as("SELECT * FROM containers WHERE user_id = ? ORDER BY position ASC")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<ContainerRow>, DbError> {
    sqlx::query_as("SELECT * FROM containers WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn insert(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateContainerRequest,
    position: i32,
) -> Result<ContainerRow, DbError> {
    let id = Uuid::new_v4().to_string();
    sqlx::query_as(
        "INSERT INTO containers (id, user_id, name, icon, description, status, parent_container_id, position)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING *",
    )
    .bind(&id)
    .bind(user_id)
    .bind(&req.name)
    .bind(&req.icon)
    .bind(&req.description)
    .bind(&req.status)
    .bind(&req.parent_container_id)
    .bind(position)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateContainerRequest,
) -> Result<Option<ContainerRow>, DbError> {
    let Some(existing) = get_one(pool, id, user_id).await? else {
        return Ok(None);
    };
    let name = req.name.as_deref().unwrap_or(&existing.name);
    let icon = req.icon.as_ref().map_or(existing.icon.as_deref(), |v| v.as_deref());
    let description = req
        .description
        .as_ref()
        .map_or(existing.description.as_deref(), |v| v.as_deref());
    let status = req.status.as_ref().map_or(existing.status.as_deref(), |v| v.as_deref());
    sqlx::query_as(
        "UPDATE containers SET name = ?, icon = ?, description = ?, status = ?, updated_at = datetime('now')
         WHERE id = ? AND user_id = ?
         RETURNING *",
    )
    .bind(name)
    .bind(icon)
    .bind(description)
    .bind(status)
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let result = sqlx::query("DELETE FROM containers WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(result.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn children(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<ContainerRow>, DbError> {
    sqlx::query_as(
        "SELECT * FROM containers WHERE parent_container_id = ? AND user_id = ? ORDER BY position ASC",
    )
    .bind(parent_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn root(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    sqlx::query_as(
        "SELECT * FROM containers WHERE user_id = ? AND parent_container_id IS NULL ORDER BY position ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn pinned(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    sqlx::query_as(
        "SELECT * FROM containers WHERE user_id = ? AND pinned = 1 ORDER BY position ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn recent(pool: &SqlitePool, user_id: &str) -> Result<Vec<ContainerRow>, DbError> {
    sqlx::query_as(
        "SELECT * FROM containers WHERE user_id = ?
         ORDER BY COALESCE(last_opened_at, updated_at) DESC
         LIMIT 10",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<ContainerRow>, DbError> {
    sqlx::query_as(
        "UPDATE containers SET pinned = CASE WHEN pinned = 1 THEN 0 ELSE 1 END, updated_at = datetime('now')
         WHERE id = ? AND user_id = ?
         RETURNING *",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn move_container(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    parent_id: Option<&str>,
    position: i32,
) -> Result<Option<ContainerRow>, DbError> {
    sqlx::query_as(
        "UPDATE containers SET parent_container_id = ?, position = ?, updated_at = datetime('now')
         WHERE id = ? AND user_id = ?
         RETURNING *",
    )
    .bind(parent_id)
    .bind(position)
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn is_descendant(
    pool: &SqlitePool,
    user_id: &str,
    ancestor_id: &str,
    possible_descendant_id: &str,
) -> Result<bool, DbError> {
    let row: (i64,) = sqlx::query_as(
        "WITH RECURSIVE subtree AS (
             SELECT id FROM containers WHERE id = ? AND user_id = ?
             UNION ALL
             SELECT c.id FROM containers c
             JOIN subtree s ON c.parent_container_id = s.id
             WHERE c.user_id = ?
         )
         SELECT COUNT(*) FROM subtree WHERE id = ?",
    )
    .bind(ancestor_id)
    .bind(user_id)
    .bind(user_id)
    .bind(possible_descendant_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row.0 > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn progress(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<ContainerProgressRow, DbError> {
    sqlx::query_as(
        "SELECT COUNT(DISTINCT l.id) as total_lists,
                COUNT(i.id) as total_items,
                COALESCE(SUM(CASE WHEN i.completed = 1 THEN 1 ELSE 0 END), 0) as completed_items
         FROM lists l
         LEFT JOIN items i ON i.list_id = l.id
         WHERE l.container_id = ? AND l.user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn touch_last_opened(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "UPDATE containers SET last_opened_at = datetime('now'), updated_at = datetime('now') WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}
```

- [ ] **Step 5: Run tests to verify they all pass**

```bash
cargo test -p kartoteka-db containers
```

Expected: all 12 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/db/src/containers.rs crates/db/src/lib.rs
git commit -m "feat(db): add containers queries with CRUD, pin, move, progress"
```

---

### Task 3: db::home — parallel container home query

**Files:**
- Create: `crates/db/src/home.rs`
- Modify: `crates/db/src/lib.rs`

- [ ] **Step 1: Write failing test for db::home**

Create `crates/db/src/home.rs`:

```rust
use crate::{containers, DbError, SqlitePool};
use crate::types::ContainerRow;

/// Parallel home data (container-only in B1; B2 adds list fields).
pub struct ContainerHomeData {
    pub pinned: Vec<ContainerRow>,
    pub recent: Vec<ContainerRow>,
    pub root: Vec<ContainerRow>,
}

/// Fetch all home container data in parallel via tokio::join!.
#[tracing::instrument(skip(pool))]
pub async fn query(pool: &SqlitePool, user_id: &str) -> Result<ContainerHomeData, DbError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
    use kartoteka_shared::types::CreateContainerRequest;

    #[tokio::test]
    async fn home_query_returns_empty_for_new_user() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let data = query(&pool, &uid).await.unwrap();
        assert!(data.pinned.is_empty());
        assert!(data.recent.is_empty());
        assert!(data.root.is_empty());
    }

    #[tokio::test]
    async fn home_query_returns_root_containers() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "Root".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        containers::insert(&pool, &uid, &req, 0).await.unwrap();

        let data = query(&pool, &uid).await.unwrap();
        assert_eq!(data.root.len(), 1);
        assert_eq!(data.root[0].name, "Root");
    }

    #[tokio::test]
    async fn home_query_separates_pinned() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "C".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let c = containers::insert(&pool, &uid, &req, 0).await.unwrap();
        containers::toggle_pin(&pool, &c.id, &uid).await.unwrap();

        let data = query(&pool, &uid).await.unwrap();
        assert_eq!(data.pinned.len(), 1);
        assert!(data.pinned[0].pinned);
    }
}
```

- [ ] **Step 2: Run to verify tests fail**

```bash
cargo test -p kartoteka-db home 2>&1 | head -15
```

Expected: FAIL with "not yet implemented".

- [ ] **Step 3: Implement query() with tokio::join!**

Replace the `todo!()` in `query`:

```rust
#[tracing::instrument(skip(pool))]
pub async fn query(pool: &SqlitePool, user_id: &str) -> Result<ContainerHomeData, DbError> {
    let (pinned, recent, root) = tokio::join!(
        containers::pinned(pool, user_id),
        containers::recent(pool, user_id),
        containers::root(pool, user_id),
    );
    Ok(ContainerHomeData {
        pinned: pinned?,
        recent: recent?,
        root: root?,
    })
}
```

Add `pub mod home;` to `crates/db/src/lib.rs`.

Also add `tokio` to `crates/db/Cargo.toml` [dependencies] (not just dev-dependencies — tokio::join! is used at runtime):

```toml
[dependencies]
# ... existing ...
tokio = { workspace = true, features = ["rt"] }
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p kartoteka-db home
```

Expected: 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/db/src/home.rs crates/db/src/lib.rs crates/db/Cargo.toml
git commit -m "feat(db): add home parallel query with tokio::join!"
```

---

### Task 4: domain::rules::containers — pure validation

**Files:**
- Create: `crates/domain/src/rules/mod.rs`
- Create: `crates/domain/src/rules/containers.rs`
- Modify: `crates/domain/src/lib.rs`

- [ ] **Step 1: Write failing tests for rules**

Create `crates/domain/src/rules/containers.rs`:

```rust
use crate::DomainError;

/// Validate that a parent container is a folder (status IS NULL), not a project.
/// Called before creating or moving a container under a parent.
pub fn validate_hierarchy(parent_status: Option<&str>) -> Result<(), DomainError> {
    todo!()
}

/// Validate that a container is not being moved to itself.
/// Deep cycle detection (descendant check) is done in orchestration with db::is_descendant.
pub fn validate_move(container_id: &str, new_parent_id: Option<&str>) -> Result<(), DomainError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_is_valid_parent() {
        assert!(validate_hierarchy(None).is_ok());
    }

    #[test]
    fn project_is_invalid_parent() {
        let result = validate_hierarchy(Some("active"));
        assert!(matches!(result, Err(DomainError::Validation("invalid_container_hierarchy"))));
    }

    #[test]
    fn any_status_rejects_as_parent() {
        assert!(validate_hierarchy(Some("done")).is_err());
        assert!(validate_hierarchy(Some("paused")).is_err());
        assert!(validate_hierarchy(Some("anything")).is_err());
    }

    #[test]
    fn move_to_different_parent_is_valid() {
        assert!(validate_move("container-1", Some("container-2")).is_ok());
    }

    #[test]
    fn move_to_self_is_invalid() {
        let result = validate_move("container-1", Some("container-1"));
        assert!(matches!(result, Err(DomainError::Validation("cannot_move_to_self"))));
    }

    #[test]
    fn move_to_root_is_valid() {
        assert!(validate_move("container-1", None).is_ok());
    }
}
```

Create `crates/domain/src/rules/mod.rs`:
```rust
pub mod containers;
```

Add to `crates/domain/src/lib.rs` (after existing content):
```rust
pub mod rules;
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p kartoteka-domain rules 2>&1 | head -15
```

Expected: FAIL with "not yet implemented".

- [ ] **Step 3: Implement validation functions**

Replace `todo!()` in `rules/containers.rs`:

```rust
pub fn validate_hierarchy(parent_status: Option<&str>) -> Result<(), DomainError> {
    if parent_status.is_some() {
        return Err(DomainError::Validation("invalid_container_hierarchy"));
    }
    Ok(())
}

pub fn validate_move(container_id: &str, new_parent_id: Option<&str>) -> Result<(), DomainError> {
    if new_parent_id == Some(container_id) {
        return Err(DomainError::Validation("cannot_move_to_self"));
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test -p kartoteka-domain rules
```

Expected: 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/rules/ crates/domain/src/lib.rs
git commit -m "feat(domain): add container validation rules (validate_hierarchy, validate_move)"
```

---

### Task 5: domain::containers — Container type + orchestration

**Files:**
- Create: `crates/domain/src/containers.rs`
- Modify: `crates/domain/src/lib.rs`

- [ ] **Step 1: Write failing integration tests**

Create `crates/domain/src/containers.rs` with stubs + tests:

```rust
use crate::{rules, DomainError};
use kartoteka_db::types::ContainerRow;
use kartoteka_db::{containers as db_containers, SqlitePool};
use kartoteka_shared::types::{
    Container, ContainerProgress, CreateContainerRequest, MoveContainerRequest,
    UpdateContainerRequest,
};

impl From<ContainerRow> for Container {
    fn from(r: ContainerRow) -> Self {
        Container {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            icon: r.icon,
            description: r.description,
            status: r.status,
            parent_container_id: r.parent_container_id,
            position: r.position,
            pinned: r.pinned,
            last_opened_at: r.last_opened_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

impl From<kartoteka_db::containers::ContainerProgressRow> for ContainerProgress {
    fn from(r: kartoteka_db::containers::ContainerProgressRow) -> Self {
        ContainerProgress {
            total_lists: r.total_lists,
            total_items: r.total_items,
            completed_items: r.completed_items,
        }
    }
}

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<Container>, DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Container, DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateContainerRequest,
) -> Result<Container, DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateContainerRequest,
) -> Result<Container, DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<(), DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn move_container(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &MoveContainerRequest,
) -> Result<Container, DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Container, DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn get_progress(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<ContainerProgress, DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn touch_last_opened(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    todo!()
}

#[tracing::instrument(skip(pool))]
pub async fn get_children(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<Container>, DomainError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    async fn make_container(pool: &SqlitePool, user_id: &str, name: &str) -> Container {
        create(
            pool,
            user_id,
            &CreateContainerRequest {
                name: name.into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: None,
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn create_and_get_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let c = make_container(&pool, &uid, "Test").await;
        assert_eq!(c.name, "Test");
        assert_eq!(c.position, 0);
        assert!(!c.pinned);

        let fetched = get_one(&pool, &c.id, &uid).await.unwrap();
        assert_eq!(fetched.id, c.id);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_not_found() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let result = get_one(&pool, "no-such-id", &uid).await;
        assert!(matches!(result, Err(DomainError::NotFound("container"))));
    }

    #[tokio::test]
    async fn create_under_folder_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let folder = make_container(&pool, &uid, "Folder").await;
        assert!(folder.status.is_none(), "folder has no status");

        let child = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Child".into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: Some(folder.id.clone()),
            },
        )
        .await
        .unwrap();
        assert_eq!(child.parent_container_id.as_deref(), Some(folder.id.as_str()));
    }

    #[tokio::test]
    async fn create_under_project_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let project = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Project".into(),
                icon: None,
                description: None,
                status: Some("active".into()),
                parent_container_id: None,
            },
        )
        .await
        .unwrap();

        let result = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Child".into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: Some(project.id.clone()),
            },
        )
        .await;
        assert!(matches!(result, Err(DomainError::Validation("invalid_container_hierarchy"))));
    }

    #[tokio::test]
    async fn positions_auto_increment() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let c1 = make_container(&pool, &uid, "A").await;
        let c2 = make_container(&pool, &uid, "B").await;
        let c3 = make_container(&pool, &uid, "C").await;
        assert_eq!(c1.position, 0);
        assert_eq!(c2.position, 1);
        assert_eq!(c3.position, 2);
    }

    #[tokio::test]
    async fn update_returns_patched_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let c = make_container(&pool, &uid, "Old").await;

        let updated = update(
            &pool,
            &c.id,
            &uid,
            &UpdateContainerRequest {
                name: Some("New".into()),
                icon: Some(Some("📂".into())),
                description: None,
                status: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.icon.as_deref(), Some("📂"));
    }

    #[tokio::test]
    async fn delete_removes_and_not_found_after() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let c = make_container(&pool, &uid, "Del").await;
        delete(&pool, &c.id, &uid).await.unwrap();
        let result = get_one(&pool, &c.id, &uid).await;
        assert!(matches!(result, Err(DomainError::NotFound("container"))));
    }

    #[tokio::test]
    async fn move_to_self_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let c = make_container(&pool, &uid, "X").await;

        let result = move_container(
            &pool,
            &c.id,
            &uid,
            &MoveContainerRequest {
                parent_container_id: Some(c.id.clone()),
                position: None,
            },
        )
        .await;
        assert!(matches!(result, Err(DomainError::Validation("cannot_move_to_self"))));
    }

    #[tokio::test]
    async fn move_to_descendant_is_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_container(&pool, &uid, "Parent").await;
        let child = create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Child".into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: Some(parent.id.clone()),
            },
        )
        .await
        .unwrap();

        // Try moving parent under child — circular
        let result = move_container(
            &pool,
            &parent.id,
            &uid,
            &MoveContainerRequest {
                parent_container_id: Some(child.id.clone()),
                position: None,
            },
        )
        .await;
        assert!(matches!(result, Err(DomainError::Validation("circular_container_move"))));
    }

    #[tokio::test]
    async fn toggle_pin_twice_restores_state() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let c = make_container(&pool, &uid, "X").await;
        assert!(!c.pinned);
        let pinned = toggle_pin(&pool, &c.id, &uid).await.unwrap();
        assert!(pinned.pinned);
        let unpinned = toggle_pin(&pool, &c.id, &uid).await.unwrap();
        assert!(!unpinned.pinned);
    }

    #[tokio::test]
    async fn list_all_returns_domain_types() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        make_container(&pool, &uid, "A").await;
        make_container(&pool, &uid, "B").await;
        let all = list_all(&pool, &uid).await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
```

Add `pub mod containers;` to `crates/domain/src/lib.rs`.

- [ ] **Step 2: Run to verify tests fail**

```bash
cargo test -p kartoteka-domain containers 2>&1 | head -20
```

Expected: all FAIL.

- [ ] **Step 3: Implement all orchestration functions**

Replace all `todo!()` bodies:

```rust
#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<Container>, DomainError> {
    let rows = db_containers::list_all(pool, user_id).await?;
    Ok(rows.into_iter().map(Container::from).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Container, DomainError> {
    db_containers::get_one(pool, id, user_id)
        .await?
        .map(Container::from)
        .ok_or(DomainError::NotFound("container"))
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateContainerRequest,
) -> Result<Container, DomainError> {
    // Phase 1: READ
    if let Some(parent_id) = &req.parent_container_id {
        let parent = db_containers::get_one(pool, parent_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("container"))?;
        // Phase 2: THINK
        rules::containers::validate_hierarchy(parent.status.as_deref())?;
    }
    // Phase 3: WRITE
    let position =
        db_containers::next_position(pool, user_id, req.parent_container_id.as_deref()).await?;
    let row = db_containers::insert(pool, user_id, req, position).await?;
    Ok(Container::from(row))
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &UpdateContainerRequest,
) -> Result<Container, DomainError> {
    db_containers::update(pool, id, user_id, req)
        .await?
        .map(Container::from)
        .ok_or(DomainError::NotFound("container"))
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<(), DomainError> {
    let deleted = db_containers::delete(pool, id, user_id).await?;
    if !deleted {
        return Err(DomainError::NotFound("container"));
    }
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn move_container(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    req: &MoveContainerRequest,
) -> Result<Container, DomainError> {
    // Phase 2: THINK (cheap self-check first, no I/O)
    rules::containers::validate_move(id, req.parent_container_id.as_deref())?;

    // Phase 1: READ — validate parent and check for cycle
    if let Some(parent_id) = &req.parent_container_id {
        let parent = db_containers::get_one(pool, parent_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("container"))?;
        rules::containers::validate_hierarchy(parent.status.as_deref())?;
        if db_containers::is_descendant(pool, user_id, id, parent_id).await? {
            return Err(DomainError::Validation("circular_container_move"));
        }
    }

    // Phase 3: WRITE
    let position = if let Some(pos) = req.position {
        pos
    } else {
        db_containers::next_position(pool, user_id, req.parent_container_id.as_deref()).await?
    };
    let row =
        db_containers::move_container(pool, id, user_id, req.parent_container_id.as_deref(), position)
            .await?
            .ok_or(DomainError::NotFound("container"))?;
    Ok(Container::from(row))
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_pin(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Container, DomainError> {
    db_containers::toggle_pin(pool, id, user_id)
        .await?
        .map(Container::from)
        .ok_or(DomainError::NotFound("container"))
}

#[tracing::instrument(skip(pool))]
pub async fn get_progress(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<ContainerProgress, DomainError> {
    // Verify ownership first
    db_containers::get_one(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("container"))?;
    let row = db_containers::progress(pool, id, user_id).await?;
    Ok(ContainerProgress::from(row))
}

#[tracing::instrument(skip(pool))]
pub async fn touch_last_opened(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    db_containers::touch_last_opened(pool, id, user_id).await?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn get_children(
    pool: &SqlitePool,
    parent_id: &str,
    user_id: &str,
) -> Result<Vec<Container>, DomainError> {
    // Verify parent ownership
    db_containers::get_one(pool, parent_id, user_id)
        .await?
        .ok_or(DomainError::NotFound("container"))?;
    let rows = db_containers::children(pool, parent_id, user_id).await?;
    Ok(rows.into_iter().map(Container::from).collect())
}
```

Note: `db_containers::next_position` is `pub(crate)` in db/. Change it to `pub` so domain/ can call it.

In `crates/db/src/containers.rs`, change the `next_position` signature:

```rust
pub async fn next_position(   // was: async fn (private)
    pool: &SqlitePool,
    user_id: &str,
    parent_id: Option<&str>,
) -> Result<i32, DbError> {
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p kartoteka-domain containers
```

Expected: 11 tests PASS.

- [ ] **Step 5: Run full workspace check**

```bash
cargo check --workspace
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add crates/domain/src/containers.rs crates/domain/src/lib.rs crates/db/src/containers.rs
git commit -m "feat(domain): add containers orchestration with hierarchy and move validation"
```

---

### Task 6: domain::home — HomeData pass-through

**Files:**
- Create: `crates/domain/src/home.rs`
- Modify: `crates/domain/src/lib.rs`

- [ ] **Step 1: Write failing test**

Create `crates/domain/src/home.rs`:

```rust
use crate::DomainError;
use crate::containers::Container;
use kartoteka_db::{home as db_home, SqlitePool};
use kartoteka_shared::types::HomeData;

/// Fetch home page data. Container-only in B1; B2 adds lists.
#[tracing::instrument(skip(pool))]
pub async fn query(pool: &SqlitePool, user_id: &str) -> Result<HomeData, DomainError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};
    use kartoteka_shared::types::CreateContainerRequest;

    #[tokio::test]
    async fn home_empty_for_new_user() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let data = query(&pool, &uid).await.unwrap();
        assert!(data.pinned_containers.is_empty());
        assert!(data.recent_containers.is_empty());
        assert!(data.root_containers.is_empty());
    }

    #[tokio::test]
    async fn home_includes_root_containers() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        crate::containers::create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Root".into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: None,
            },
        )
        .await
        .unwrap();

        let data = query(&pool, &uid).await.unwrap();
        assert_eq!(data.root_containers.len(), 1);
        assert_eq!(data.root_containers[0].name, "Root");
    }
}
```

Add `pub mod home;` to `crates/domain/src/lib.rs`.

- [ ] **Step 2: Run to verify tests fail**

```bash
cargo test -p kartoteka-domain home 2>&1 | head -10
```

Expected: FAIL.

- [ ] **Step 3: Implement query()**

```rust
#[tracing::instrument(skip(pool))]
pub async fn query(pool: &SqlitePool, user_id: &str) -> Result<HomeData, DomainError> {
    let data = db_home::query(pool, user_id).await?;
    Ok(HomeData {
        pinned_containers: data.pinned.into_iter().map(Container::from).collect(),
        recent_containers: data.recent.into_iter().map(Container::from).collect(),
        root_containers: data.root.into_iter().map(Container::from).collect(),
    })
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p kartoteka-domain home
```

Expected: 2 tests PASS.

- [ ] **Step 5: Run all domain tests**

```bash
cargo test -p kartoteka-domain
```

Expected: all tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/domain/src/home.rs crates/domain/src/lib.rs
git commit -m "feat(domain): add home pass-through returning container home data"
```

---

### Task 7: Server scaffolding — axum, AppState, AppError, UserId, main.rs

**Files:**
- Modify: `Cargo.toml` (root)
- Modify: `crates/server/Cargo.toml`
- Modify: `crates/server/src/lib.rs`
- Create: `crates/server/src/main.rs`
- Create: `crates/server/src/error.rs`
- Create: `crates/server/src/extractors.rs`

- [ ] **Step 1: Add axum and tracing-subscriber to workspace dependencies**

In root `Cargo.toml`, append to `[workspace.dependencies]`:

```toml
axum = { version = "0.8", features = ["json", "macros"] }
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 2: Update server/Cargo.toml**

Replace the contents of `crates/server/Cargo.toml` with:

```toml
[package]
name = "kartoteka-server"
version.workspace = true
edition.workspace = true
publish = false

[[bin]]
name = "kartoteka-server"
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
axum.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

- [ ] **Step 3: Create server/src/error.rs**

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use kartoteka_domain::DomainError;

#[derive(Debug)]
pub enum AppError {
    NotFound(&'static str),
    Validation(String),
    Forbidden,
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(),
            AppError::Validation(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg).into_response()
            }
            AppError::Forbidden => StatusCode::FORBIDDEN.into_response(),
            AppError::Internal(msg) => {
                tracing::error!("internal server error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
            }
        }
    }
}

impl From<DomainError> for AppError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::NotFound(msg) => AppError::NotFound(msg),
            DomainError::Validation(msg) => AppError::Validation(msg.to_string()),
            DomainError::FeatureRequired(f) => {
                AppError::Validation(format!("feature required: {f}"))
            }
            DomainError::Forbidden => AppError::Forbidden,
            DomainError::Internal(msg) => AppError::Internal(msg),
            DomainError::Db(e) => AppError::Internal(e.to_string()),
        }
    }
}
```

- [ ] **Step 4: Create server/src/extractors.rs**

```rust
use axum::{extract::FromRequestParts, http::{request::Parts, StatusCode}};

/// Placeholder user identity extractor.
/// Reads `X-User-Id` header. Replaced by real auth middleware in C1.
pub struct UserId(pub String);

impl<S: Send + Sync> FromRequestParts<S> for UserId {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| UserId(s.to_string()))
            .ok_or((StatusCode::UNAUTHORIZED, "X-User-Id header required"))
    }
}
```

- [ ] **Step 5: Create server/src/routes/mod.rs (empty stub for now)**

```
crates/server/src/routes/mod.rs
```

```rust
pub mod containers;
pub mod home;

use axum::Router;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/containers", containers::routes())
        .merge(home::routes())
}
```

Create empty stubs (will be filled in Task 8):

`crates/server/src/routes/containers.rs`:
```rust
use axum::Router;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
}
```

`crates/server/src/routes/home.rs`:
```rust
use axum::Router;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
}
```

- [ ] **Step 6: Rewrite server/src/lib.rs**

```rust
pub mod error;
pub mod extractors;
pub mod routes;

use axum::Router;
use kartoteka_db::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

pub fn router(pool: SqlitePool) -> Router {
    let state = AppState { pool };
    Router::new()
        .nest("/api", routes::routes())
        .with_state(state)
}
```

- [ ] **Step 7: Create server/src/main.rs**

```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kartoteka_server=debug,kartoteka_domain=debug,kartoteka_db=info,tower_http=debug".into()),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "kartoteka.db".to_string());
    let pool = kartoteka_db::create_pool(&database_url)
        .await
        .expect("failed to create database pool");
    kartoteka_db::run_migrations(&pool)
        .await
        .expect("failed to run migrations");

    let app = kartoteka_server::router(pool);
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind_addr}: {e}"));
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 8: Verify workspace compiles**

```bash
cargo check --workspace
```

Expected: no errors.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml crates/server/
git commit -m "feat(server): add axum scaffold — AppState, AppError, UserId extractor, main.rs"
```

---

### Task 8: REST endpoints — /api/containers/* and /api/home

**Files:**
- Modify: `crates/server/src/routes/containers.rs`
- Modify: `crates/server/src/routes/home.rs`

- [ ] **Step 1: Implement containers REST handlers**

Replace contents of `crates/server/src/routes/containers.rs`:

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use kartoteka_domain::containers as domain;
use kartoteka_shared::types::{
    Container, ContainerProgress, CreateContainerRequest, MoveContainerRequest,
    UpdateContainerRequest,
};

use crate::{error::AppError, extractors::UserId, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_containers).post(create_container))
        .route("/:id", get(get_container).put(update_container).delete(delete_container))
        .route("/:id/pin", patch(toggle_pin))
        .route("/:id/move", patch(move_container_handler))
        .route("/:id/progress", get(get_progress))
        .route("/:id/children", get(get_children))
}

async fn list_containers(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<Container>>, AppError> {
    let containers = domain::list_all(&state.pool, &user_id).await?;
    Ok(Json(containers))
}

async fn get_container(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<Container>, AppError> {
    domain::touch_last_opened(&state.pool, &id, &user_id).await.ok(); // best-effort
    let container = domain::get_one(&state.pool, &id, &user_id).await?;
    Ok(Json(container))
}

async fn create_container(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<CreateContainerRequest>,
) -> Result<(StatusCode, Json<Container>), AppError> {
    let container = domain::create(&state.pool, &user_id, &req).await?;
    Ok((StatusCode::CREATED, Json(container)))
}

async fn update_container(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateContainerRequest>,
) -> Result<Json<Container>, AppError> {
    let container = domain::update(&state.pool, &id, &user_id, &req).await?;
    Ok(Json(container))
}

async fn delete_container(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    domain::delete(&state.pool, &id, &user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_pin(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<Container>, AppError> {
    let container = domain::toggle_pin(&state.pool, &id, &user_id).await?;
    Ok(Json(container))
}

async fn move_container_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<MoveContainerRequest>,
) -> Result<Json<Container>, AppError> {
    let container = domain::move_container(&state.pool, &id, &user_id, &req).await?;
    Ok(Json(container))
}

async fn get_progress(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<ContainerProgress>, AppError> {
    let progress = domain::get_progress(&state.pool, &id, &user_id).await?;
    Ok(Json(progress))
}

async fn get_children(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<Vec<Container>>, AppError> {
    let children = domain::get_children(&state.pool, &id, &user_id).await?;
    Ok(Json(children))
}
```

- [ ] **Step 2: Implement home REST handler**

Replace contents of `crates/server/src/routes/home.rs`:

```rust
use axum::{extract::State, routing::get, Json, Router};
use kartoteka_domain::home as domain;
use kartoteka_shared::types::HomeData;

use crate::{error::AppError, extractors::UserId, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/home", get(home_handler))
}

async fn home_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<HomeData>, AppError> {
    let data = domain::query(&state.pool, &user_id).await?;
    Ok(Json(data))
}
```

- [ ] **Step 3: Verify workspace compiles cleanly**

```bash
cargo check --workspace
```

Expected: no errors.

- [ ] **Step 4: Build the server binary**

```bash
cargo build -p kartoteka-server
```

Expected: compiles successfully, binary at `target/debug/kartoteka-server`.

- [ ] **Step 5: Manual smoke test — verify REST endpoints work**

In one terminal, start the server:
```bash
DATABASE_URL=:memory: RUST_LOG=debug cargo run -p kartoteka-server
```

In another terminal, run these curl commands (replace `test-user-id` with any string):

```bash
# Create a container
curl -s -X POST http://localhost:3000/api/containers \
  -H "Content-Type: application/json" \
  -H "X-User-Id: test-user-id" \
  -d '{"name":"My Project","status":"active"}' | jq .
# Expected: {"id":"...","name":"My Project","status":"active","position":0,"pinned":false,...}

# List containers
curl -s http://localhost:3000/api/containers \
  -H "X-User-Id: test-user-id" | jq .
# Expected: array with 1 container

# Get home
curl -s http://localhost:3000/api/home \
  -H "X-User-Id: test-user-id" | jq .
# Expected: {"pinned_containers":[],"recent_containers":[{"id":...}],"root_containers":[{"id":...}]}

# Missing X-User-Id header
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/api/containers
# Expected: 401
```

- [ ] **Step 6: Run full test suite**

```bash
cargo test --workspace
```

Expected: all tests pass (db + domain + shared + i18n).

- [ ] **Step 7: Final commit**

```bash
git add crates/server/src/routes/
git commit -m "feat(server): add REST endpoints for /api/containers/* and /api/home"
```

---

## Self-Review

### Spec coverage

| Requirement | Task |
|---|---|
| `db::containers` — CRUD, children, progress, pin, move | Task 2 |
| `db::home` — composite query with tokio::join! | Task 3 |
| `domain::containers` — orchestration | Task 5 |
| `domain::rules::containers` — validate_hierarchy, validate_move | Task 4 |
| `domain::home` — pass-through | Task 6 |
| REST `/api/containers/*` | Task 8 |
| REST `/api/home` | Task 8 |
| Tests: db queries | Task 2 |
| Tests: domain rules | Task 4 |
| Tests: domain integration | Task 5 |
| Deliverable: curl containers CRUD works | Task 8, Step 5 |

### Placeholder scan

No TBD, TODO, or "add later" items. All steps have complete code.

### Type consistency

- `ContainerRow` (db/types.rs, A1) → `Container` (domain/containers.rs, Task 5) via `From` impl
- `CreateContainerRequest` used in db::insert (Task 2) and domain::create (Task 5) — same import from `kartoteka_shared::types`
- `ContainerProgressRow` defined Task 2, used in Task 5 `From` impl
- `HomeData` struct (Task 1, shared::types) returned by domain::home (Task 6) and REST handler (Task 8)
- `next_position` is `pub` after Task 5 note (Task 2, Step 4 note)

---

**Plan complete and saved to `docs/superpowers/plans/B1-containers.md`.**

**Two execution options:**

**1. Subagent-Driven (recommended)** — fresh subagent per task, review between tasks

**2. Inline Execution** — execute tasks in this session using executing-plans skill

**Which approach?**
