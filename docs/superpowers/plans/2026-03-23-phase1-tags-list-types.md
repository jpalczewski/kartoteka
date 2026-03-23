# Phase 1: Tag System + List Type UI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add hierarchical tags with colors and proper list type UI to Kartoteka.

**Architecture:** Tags are per-user entities stored in D1, linked to items/lists via junction tables. The router passes `user_id` through `Router::with_data()` to avoid double Hanko validation. Frontend fetches tags once and joins client-side with items/lists.

**Tech Stack:** Rust (worker 0.7, sqlx-d1), Leptos 0.7 CSR, D1/SQLite, gloo-net

---

## File Structure

### New files
- `crates/api/migrations/0002_tags.sql` — Tags schema
- `crates/api/src/handlers/tags.rs` — Tag CRUD + assignment handlers
- `crates/frontend/src/components/tag_badge.rs` — Tag badge display component
- `crates/frontend/src/components/tag_selector.rs` — Tag picker dropdown
- `crates/frontend/src/pages/tags.rs` — Tag management page

### Modified files
- `crates/shared/src/lib.rs` — Tag types, TagCategory enum, DTOs
- `crates/api/src/router.rs` — Pass user_id via `Router::with_data()`, add tag routes
- `crates/api/src/handlers/mod.rs` — Add `pub mod tags`
- `crates/api/src/handlers/lists.rs` — Change `RouteContext<()>` → `RouteContext<String>`
- `crates/api/src/handlers/items.rs` — Change `RouteContext<()>` → `RouteContext<String>`
- `crates/frontend/src/api.rs` — Tag API client functions
- `crates/frontend/src/components/mod.rs` — Register new components
- `crates/frontend/src/components/list_card.rs` — List type labels + tag badges
- `crates/frontend/src/components/item_row.rs` — Tag badges + tag selector
- `crates/frontend/src/pages/home.rs` — List type selector, tag filter
- `crates/frontend/src/pages/list.rs` — Item tag display + assignment
- `crates/frontend/src/pages/mod.rs` — Register tags page
- `crates/frontend/src/app.rs` — Add `/tags` route
- `crates/frontend/src/components/nav.rs` — Add "Tagi" link
- `crates/frontend/style/main.css` — Tag styles

---

### Task 1: Database Migration

**Files:**
- Create: `crates/api/migrations/0002_tags.sql`

- [ ] **Step 1: Write migration SQL**

```sql
-- Tags table
CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#888888',
    category TEXT NOT NULL DEFAULT 'custom',
    parent_tag_id TEXT REFERENCES tags(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Junction: items <-> tags
CREATE TABLE item_tags (
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_id)
);

-- Junction: lists <-> tags
CREATE TABLE list_tags (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (list_id, tag_id)
);

CREATE INDEX idx_tags_user ON tags(user_id);
CREATE INDEX idx_tags_user_cat ON tags(user_id, category);
CREATE INDEX idx_item_tags_item ON item_tags(item_id);
CREATE INDEX idx_item_tags_tag ON item_tags(tag_id);
CREATE INDEX idx_list_tags_list ON list_tags(list_id);
CREATE INDEX idx_list_tags_tag ON list_tags(tag_id);
```

- [ ] **Step 2: Apply migration locally**

Run: `just migrate-local`
Expected: Migration applied successfully

- [ ] **Step 3: Commit**

```bash
git add crates/api/migrations/0002_tags.sql
git commit -m "feat: add tags database schema (migration 0002)"
```

---

### Task 2: Shared Types for Tags

**Files:**
- Modify: `crates/shared/src/lib.rs`

- [ ] **Step 1: Add TagCategory enum and Tag struct**

Add after existing types:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagCategory {
    Context,
    Priority,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub category: TagCategory,
    pub parent_tag_id: Option<String>,
    pub created_at: String,
}
```

- [ ] **Step 2: Add Tag DTOs**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: String,
    pub category: TagCategory,
    pub parent_tag_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub category: Option<TagCategory>,
    pub parent_tag_id: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagAssignment {
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTagLink {
    pub item_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTagLink {
    pub list_id: String,
    pub tag_id: String,
}
```

- [ ] **Step 3: Verify compilation**

Run: `just check`
Expected: Compiles without errors

- [ ] **Step 4: Commit**

```bash
git add crates/shared/src/lib.rs
git commit -m "feat: add Tag types and DTOs to shared crate"
```

---

### Task 3: Pass user_id Through Router + Update Existing Handlers

**Files:**
- Modify: `crates/api/src/router.rs`
- Modify: `crates/api/src/handlers/lists.rs`
- Modify: `crates/api/src/handlers/items.rs`

- [ ] **Step 1: Modify router to use `Router::with_data(user_id)`**

In `router.rs`, change auth block and router creation:

```rust
// Replace:
//   if let Err(e) = auth::validate_session(&req).await { ... }
//   let router = Router::new();
// With:
    let user_id = match auth::validate_session(&req).await {
        Ok(uid) => uid,
        Err(e) => {
            let body = serde_json::json!({ "error": e.to_string() });
            return Ok(Response::from_json(&body)?
                .with_status(401)
                .with_headers(cors));
        }
    };

    let router = Router::with_data(user_id);
```

- [ ] **Step 2: Update list handlers — change `RouteContext<()>` to `RouteContext<String>`**

In `lists.rs`, replace all `ctx: RouteContext<()>` with `ctx: RouteContext<String>`. Five functions: `list_all`, `create`, `get_one`, `update`, `delete`.

- [ ] **Step 3: Update item handlers — same change**

In `items.rs`, replace all `ctx: RouteContext<()>` with `ctx: RouteContext<String>`. Four functions: `list_all`, `create`, `update`, `delete`.

- [ ] **Step 4: Verify compilation**

Run: `just check`
Expected: Compiles without errors

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/router.rs crates/api/src/handlers/lists.rs crates/api/src/handlers/items.rs
git commit -m "refactor: pass user_id through Router::with_data for handler access"
```

---

### Task 4: Tag CRUD API Handlers

**Files:**
- Create: `crates/api/src/handlers/tags.rs`
- Modify: `crates/api/src/handlers/mod.rs`
- Modify: `crates/api/src/router.rs`

- [ ] **Step 1: Add `pub mod tags` to `handlers/mod.rs`**

- [ ] **Step 2: Create `tags.rs` with CRUD handlers**

User ID is accessed via `ctx.data.as_str()` (since data is `String`). Follow existing handler patterns exactly.

```rust
use kartoteka_shared::*;
use wasm_bindgen::JsValue;
use worker::*;

/// GET /api/tags
pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.as_str();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare("SELECT id, user_id, name, color, category, parent_tag_id, created_at FROM tags WHERE user_id = ?1 ORDER BY category, name")
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let tags = result.results::<Tag>()?;
    Response::from_json(&tags)
}

/// POST /api/tags
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: CreateTagRequest = req.json().await?;
    let id = uuid::Uuid::new_v4().to_string();
    let category_str = serde_json::to_value(&body.category)
        .map_err(|e| Error::from(e.to_string()))?
        .as_str()
        .unwrap_or("custom")
        .to_string();

    let parent_val: JsValue = match &body.parent_tag_id {
        Some(p) => p.as_str().into(),
        None => JsValue::NULL,
    };

    let d1 = ctx.env.d1("DB")?;
    d1.prepare("INSERT INTO tags (id, user_id, name, color, category, parent_tag_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .bind(&[id.clone().into(), user_id.into(), body.name.into(), body.color.into(), category_str.into(), parent_val])?
        .run()
        .await?;

    let tag = d1
        .prepare("SELECT id, user_id, name, color, category, parent_tag_id, created_at FROM tags WHERE id = ?1")
        .bind(&[id.into()])?
        .first::<Tag>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create tag"))?;

    Ok(Response::from_json(&tag)?.with_status(201))
}

/// PUT /api/tags/:id
pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.as_str();
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?.to_string();
    let body: UpdateTagRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    // Verify ownership
    let existing = d1.prepare("SELECT id FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if existing.is_none() {
        return Response::error("Not found", 404);
    }

    if let Some(name) = &body.name {
        d1.prepare("UPDATE tags SET name = ?1 WHERE id = ?2")
            .bind(&[name.clone().into(), id.clone().into()])?
            .run().await?;
    }
    if let Some(color) = &body.color {
        d1.prepare("UPDATE tags SET color = ?1 WHERE id = ?2")
            .bind(&[color.clone().into(), id.clone().into()])?
            .run().await?;
    }
    if let Some(category) = &body.category {
        let cat_str = serde_json::to_value(category)
            .map_err(|e| Error::from(e.to_string()))?
            .as_str().unwrap_or("custom").to_string();
        d1.prepare("UPDATE tags SET category = ?1 WHERE id = ?2")
            .bind(&[cat_str.into(), id.clone().into()])?
            .run().await?;
    }
    if let Some(parent) = &body.parent_tag_id {
        let parent_val: JsValue = match parent {
            Some(p) => p.as_str().into(),
            None => JsValue::NULL,
        };
        d1.prepare("UPDATE tags SET parent_tag_id = ?1 WHERE id = ?2")
            .bind(&[parent_val, id.clone().into()])?
            .run().await?;
    }

    let tag = d1
        .prepare("SELECT id, user_id, name, color, category, parent_tag_id, created_at FROM tags WHERE id = ?1")
        .bind(&[id.into()])?
        .first::<Tag>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&tag)
}

/// DELETE /api/tags/:id
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.as_str();
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?;
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.into(), user_id.into()])?
        .run().await?;
    Ok(Response::empty()?.with_status(204))
}
```

- [ ] **Step 3: Add tag assignment handlers to `tags.rs`**

```rust
/// POST /api/items/:item_id/tags — assign tag to item
pub async fn assign_to_item(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let item_id = ctx.param("item_id").ok_or_else(|| Error::from("Missing item_id"))?.to_string();
    let body: TagAssignment = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)")
        .bind(&[item_id.into(), body.tag_id.into()])?
        .run().await?;
    Ok(Response::empty()?.with_status(204))
}

/// DELETE /api/items/:item_id/tags/:tag_id
pub async fn remove_from_item(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let item_id = ctx.param("item_id").ok_or_else(|| Error::from("Missing item_id"))?;
    let tag_id = ctx.param("tag_id").ok_or_else(|| Error::from("Missing tag_id"))?;
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM item_tags WHERE item_id = ?1 AND tag_id = ?2")
        .bind(&[item_id.into(), tag_id.into()])?
        .run().await?;
    Ok(Response::empty()?.with_status(204))
}

/// POST /api/lists/:list_id/tags — assign tag to list
pub async fn assign_to_list(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let list_id = ctx.param("list_id").ok_or_else(|| Error::from("Missing list_id"))?.to_string();
    let body: TagAssignment = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("INSERT OR IGNORE INTO list_tags (list_id, tag_id) VALUES (?1, ?2)")
        .bind(&[list_id.into(), body.tag_id.into()])?
        .run().await?;
    Ok(Response::empty()?.with_status(204))
}

/// DELETE /api/lists/:list_id/tags/:tag_id
pub async fn remove_from_list(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let list_id = ctx.param("list_id").ok_or_else(|| Error::from("Missing list_id"))?;
    let tag_id = ctx.param("tag_id").ok_or_else(|| Error::from("Missing tag_id"))?;
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM list_tags WHERE list_id = ?1 AND tag_id = ?2")
        .bind(&[list_id.into(), tag_id.into()])?
        .run().await?;
    Ok(Response::empty()?.with_status(204))
}

/// GET /api/tag-links/items — all item-tag links for user's items
pub async fn all_item_tag_links(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.as_str();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare("SELECT it.item_id, it.tag_id FROM item_tags it JOIN items i ON i.id = it.item_id JOIN lists l ON l.id = i.list_id JOIN tags t ON t.id = it.tag_id WHERE t.user_id = ?1")
        .bind(&[user_id.into()])?
        .all().await?;
    let links = result.results::<ItemTagLink>()?;
    Response::from_json(&links)
}

/// GET /api/tag-links/lists — all list-tag links for user's lists
pub async fn all_list_tag_links(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.as_str();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare("SELECT lt.list_id, lt.tag_id FROM list_tags lt JOIN tags t ON t.id = lt.tag_id WHERE t.user_id = ?1")
        .bind(&[user_id.into()])?
        .all().await?;
    let links = result.results::<ListTagLink>()?;
    Response::from_json(&links)
}
```

- [ ] **Step 4: Register routes in `router.rs`**

Add after existing routes in the router chain:

```rust
// Tags CRUD
.get_async("/api/tags", tags::list_all)
.post_async("/api/tags", tags::create)
.put_async("/api/tags/:id", tags::update)
.delete_async("/api/tags/:id", tags::delete)
// Tag assignments
.post_async("/api/items/:item_id/tags", tags::assign_to_item)
.delete_async("/api/items/:item_id/tags/:tag_id", tags::remove_from_item)
.post_async("/api/lists/:list_id/tags", tags::assign_to_list)
.delete_async("/api/lists/:list_id/tags/:tag_id", tags::remove_from_list)
// Tag link queries
.get_async("/api/tag-links/items", tags::all_item_tag_links)
.get_async("/api/tag-links/lists", tags::all_list_tag_links)
```

Update import: `use crate::handlers::{items, lists, tags};`

- [ ] **Step 5: Verify compilation**

Run: `just check`
Expected: Compiles without errors

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/handlers/tags.rs crates/api/src/handlers/mod.rs crates/api/src/router.rs
git commit -m "feat: add tag CRUD and assignment API endpoints"
```

---

### Task 5: Frontend API Client for Tags

**Files:**
- Modify: `crates/frontend/src/api.rs`

- [ ] **Step 1: Add tag API functions**

Follow existing `fetch_lists`, `create_list` patterns:

```rust
// Tag CRUD
pub async fn fetch_tags() -> Result<Vec<Tag>, String> {
    get(&format!("{API_BASE}/tags"))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())
}

pub async fn create_tag(req: &CreateTagRequest) -> Result<Tag, String> {
    post_json(&format!("{API_BASE}/tags"), req).await
}

pub async fn update_tag(id: &str, req: &UpdateTagRequest) -> Result<Tag, String> {
    put_json(&format!("{API_BASE}/tags/{id}"), req).await
}

pub async fn delete_tag(id: &str) -> Result<(), String> {
    del(&format!("{API_BASE}/tags/{id}"))
        .send().await.map_err(|e| e.to_string())?;
    Ok(())
}

// Tag assignments
pub async fn assign_tag_to_item(item_id: &str, tag_id: &str) -> Result<(), String> {
    post_json::<serde_json::Value>(&format!("{API_BASE}/items/{item_id}/tags"), &TagAssignment { tag_id: tag_id.to_string() }).await?;
    Ok(())
}

pub async fn remove_tag_from_item(item_id: &str, tag_id: &str) -> Result<(), String> {
    del(&format!("{API_BASE}/items/{item_id}/tags/{tag_id}"))
        .send().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn assign_tag_to_list(list_id: &str, tag_id: &str) -> Result<(), String> {
    post_json::<serde_json::Value>(&format!("{API_BASE}/lists/{list_id}/tags"), &TagAssignment { tag_id: tag_id.to_string() }).await?;
    Ok(())
}

pub async fn remove_tag_from_list(list_id: &str, tag_id: &str) -> Result<(), String> {
    del(&format!("{API_BASE}/lists/{list_id}/tags/{tag_id}"))
        .send().await.map_err(|e| e.to_string())?;
    Ok(())
}

// Tag link bulk queries
pub async fn fetch_list_tag_links() -> Result<Vec<ListTagLink>, String> {
    get(&format!("{API_BASE}/tag-links/lists"))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())
}

pub async fn fetch_item_tag_links() -> Result<Vec<ItemTagLink>, String> {
    get(&format!("{API_BASE}/tag-links/items"))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())
}
```

Note: `assign_tag_to_item` uses `post_json` which expects a JSON response, but the endpoint returns 204 empty. Need to use the raw `Request::post` pattern instead:

```rust
pub async fn assign_tag_to_item(item_id: &str, tag_id: &str) -> Result<(), String> {
    let body = serde_json::to_string(&TagAssignment { tag_id: tag_id.to_string() })
        .map_err(|e| e.to_string())?;
    gloo_net::http::Request::post(&format!("{API_BASE}/items/{item_id}/tags"))
        .headers(auth_headers())
        .body(body).map_err(|e| e.to_string())?
        .send().await.map_err(|e| e.to_string())?;
    Ok(())
}
```

Same pattern for `assign_tag_to_list`.

- [ ] **Step 2: Verify compilation**

Run: `just check`

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/api.rs
git commit -m "feat: add tag API client functions to frontend"
```

---

### Task 6: Tag UI Components + CSS

**Files:**
- Create: `crates/frontend/src/components/tag_badge.rs`
- Create: `crates/frontend/src/components/tag_selector.rs`
- Modify: `crates/frontend/src/components/mod.rs`
- Modify: `crates/frontend/style/main.css`

- [ ] **Step 1: Create TagBadge component**

`tag_badge.rs`:
```rust
use kartoteka_shared::Tag;
use leptos::prelude::*;

#[component]
pub fn TagBadge(tag: Tag, #[prop(optional)] on_remove: Option<Callback<String>>) -> impl IntoView {
    let tag_id = tag.id.clone();
    view! {
        <span class="tag-badge" style=format!("background: {}; color: white;", tag.color)>
            {&tag.name}
            {if let Some(on_remove) = on_remove {
                let tid = tag_id.clone();
                view! {
                    <button class="tag-remove" on:click=move |_| on_remove.run(tid.clone())>"x"</button>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </span>
    }
}
```

- [ ] **Step 2: Create TagSelector component**

`tag_selector.rs`:
```rust
use kartoteka_shared::Tag;
use leptos::prelude::*;

#[component]
pub fn TagSelector(
    all_tags: Vec<Tag>,
    selected_tag_ids: Vec<String>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    let (open, set_open) = signal(false);

    view! {
        <div class="tag-selector">
            <button class="btn btn-sm" on:click=move |_| set_open.update(|v| *v = !*v)>
                "+"
            </button>
            <div class="tag-selector-dropdown" style:display=move || if open.get() { "block" } else { "none" }>
                {all_tags.iter().map(|tag| {
                    let is_selected = selected_tag_ids.contains(&tag.id);
                    let color = tag.color.clone();
                    let name = tag.name.clone();
                    let tid = tag.id.clone();
                    view! {
                        <label class="tag-option" style=format!("border-left: 3px solid {color};")>
                            <input
                                type="checkbox"
                                checked=is_selected
                                on:change=move |_| on_toggle.run(tid.clone())
                            />
                            {name}
                        </label>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
```

- [ ] **Step 3: Register in `components/mod.rs`**

Add:
```rust
pub mod tag_badge;
pub mod tag_selector;
```

- [ ] **Step 4: Add CSS**

Append to `style/main.css`:

```css
/* Tag badges */
.tag-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.1rem 0.45rem;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 500;
    white-space: nowrap;
}

.tag-remove {
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    padding: 0;
    font-size: 0.65rem;
    opacity: 0.7;
}

.tag-remove:hover {
    opacity: 1;
}

.tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-top: 0.5rem;
}

/* Tag selector */
.tag-selector {
    position: relative;
    display: inline-block;
}

.tag-selector-dropdown {
    position: absolute;
    right: 0;
    top: calc(100% + 0.25rem);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    min-width: 180px;
    max-height: 240px;
    overflow-y: auto;
    z-index: 50;
    padding: 0.5rem;
    animation: fadeIn 0.1s ease-out;
}

.tag-option {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    font-size: 0.85rem;
    cursor: pointer;
    border-radius: 0.25rem;
}

.tag-option:hover {
    background: var(--bg);
}

.btn-sm {
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
}

/* List type selector */
select {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    padding: 0.5rem 0.75rem;
    font-size: 0.9rem;
}

/* Tag filter bar */
.tag-filter-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-bottom: 1rem;
}

.tag-filter-bar .tag-badge {
    cursor: pointer;
    opacity: 0.5;
    transition: opacity 0.15s;
}

.tag-filter-bar .tag-badge.active {
    opacity: 1;
}

/* Tag management */
.tag-management {
    margin-top: 2rem;
}

.tag-group {
    margin-bottom: 1.5rem;
}

.tag-group h4 {
    font-size: 0.85rem;
    color: var(--text-muted);
    margin-bottom: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.tag-form {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 1rem;
}

.tag-form input[type="color"] {
    width: 2rem;
    height: 2rem;
    border: none;
    border-radius: 0.25rem;
    cursor: pointer;
    background: none;
    padding: 0;
}

.tag-edit-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0;
}
```

- [ ] **Step 5: Verify compilation**

Run: `just check`

- [ ] **Step 6: Commit**

```bash
git add crates/frontend/src/components/tag_badge.rs crates/frontend/src/components/tag_selector.rs crates/frontend/src/components/mod.rs crates/frontend/style/main.css
git commit -m "feat: add TagBadge and TagSelector components with CSS"
```

---

### Task 7: List Type UI + Home Page Tag Filter

**Files:**
- Modify: `crates/frontend/src/components/list_card.rs`
- Modify: `crates/frontend/src/pages/home.rs`

- [ ] **Step 1: Improve ListCard with proper type labels and tag badges**

Replace `list_card.rs` content:

```rust
use kartoteka_shared::{List, ListType, Tag};
use leptos::prelude::*;
use crate::components::tag_badge::TagBadge;

fn list_type_label(lt: &ListType) -> &'static str {
    match lt {
        ListType::Shopping => "Zakupy",
        ListType::Packing => "Pakowanie",
        ListType::Project => "Projekt",
        ListType::Custom => "Lista",
    }
}

fn list_type_icon(lt: &ListType) -> &'static str {
    match lt {
        ListType::Shopping => "\u{1F6D2}",
        ListType::Packing => "\u{1F9F3}",
        ListType::Project => "\u{1F4CB}",
        ListType::Custom => "\u{1F4DD}",
    }
}

#[component]
pub fn ListCard(list: List, #[prop(default = vec![])] tags: Vec<Tag>) -> impl IntoView {
    let href = format!("/lists/{}", list.id);
    let icon = list_type_icon(&list.list_type);
    let label = list_type_label(&list.list_type);

    view! {
        <a href=href style="text-decoration: none; color: inherit;">
            <div class="card">
                <h3>{&list.name}</h3>
                <span class="meta">{icon} " " {label}</span>
                {if !tags.is_empty() {
                    view! {
                        <div class="tag-list">
                            {tags.into_iter().map(|t| view! { <TagBadge tag=t/> }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </a>
    }
}
```

- [ ] **Step 2: Add list type selector and tag filter to home page**

In `home.rs`:
- Add `ListType` selector (`<select>`) to the create-list form
- Fetch tags and list-tag-links
- Add tag filter bar above list grid
- Pass relevant tags to each ListCard

Key additions:
```rust
// Signals
let (new_list_type, set_new_list_type) = signal(ListType::Custom);
let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

// Resources
let tags_resource = LocalResource::new(|| api::fetch_tags());
let list_tag_links = LocalResource::new(|| api::fetch_list_tag_links());

// In create handler:
let req = CreateListRequest { name, list_type: new_list_type.get() };

// List type <select> in view:
// <select on:change=move |ev| { ... parse and set_new_list_type ... }>

// Tag filter bar: render all tags as clickable badges, filter lists by active tag
```

- [ ] **Step 3: Verify it works**

Run: `just dev`
Test: Create lists with different types, verify icons/labels appear correctly

- [ ] **Step 4: Commit**

```bash
git add crates/frontend/src/components/list_card.rs crates/frontend/src/pages/home.rs
git commit -m "feat: add list type selector, proper labels/icons, tag filter on home page"
```

---

### Task 8: Tag Management Page

**Files:**
- Create: `crates/frontend/src/pages/tags.rs`
- Modify: `crates/frontend/src/pages/mod.rs`
- Modify: `crates/frontend/src/app.rs`
- Modify: `crates/frontend/src/components/nav.rs`

- [ ] **Step 1: Create tag management page**

`tags.rs` — full CRUD for tags grouped by category:

```rust
use kartoteka_shared::*;
use leptos::prelude::*;
use crate::api;
use crate::components::tag_badge::TagBadge;

#[component]
pub fn TagsPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p>"Zaloguj sie"</p> }.into_any();
    }

    let tags = RwSignal::new(Vec::<Tag>::new());
    let (loading, set_loading) = signal(true);
    let (new_name, set_new_name) = signal(String::new());
    let (new_color, set_new_color) = signal("#e94560".to_string());
    let (new_category, set_new_category) = signal(TagCategory::Custom);

    // Fetch tags
    leptos::task::spawn_local(async move {
        if let Ok(fetched) = api::fetch_tags().await {
            tags.set(fetched);
        }
        set_loading.set(false);
    });

    let on_create = move |_| {
        let name = new_name.get();
        if name.is_empty() { return; }
        let color = new_color.get();
        let category = new_category.get();
        set_new_name.set(String::new());
        leptos::task::spawn_local(async move {
            let req = CreateTagRequest {
                name, color, category,
                parent_tag_id: None,
            };
            if let Ok(tag) = api::create_tag(&req).await {
                tags.update(|t| t.push(tag));
            }
        });
    };

    let on_delete = Callback::new(move |tag_id: String| {
        tags.update(|t| t.retain(|tag| tag.id != tag_id));
        leptos::task::spawn_local(async move {
            let _ = api::delete_tag(&tag_id).await;
        });
    });

    // Render with category groups: Kontekst, Priorytet, Własne
    view! {
        <h2>"Tagi"</h2>
        // Create form: name input + color picker + category select + button
        // Then grouped tag lists with delete buttons
        // ... (full implementation during execution)
    }.into_any()
}
```

- [ ] **Step 2: Register page**

In `pages/mod.rs` add: `pub mod tags;`

In `app.rs` add route: `<Route path=path!("/tags") view=TagsPage/>`

In `nav.rs` add link: `<a href="/tags" class="user-menu-item">"Tagi"</a>` in the user menu dropdown

- [ ] **Step 3: Verify it works**

Run: `just dev`
Test: Navigate to /tags, create tags with different categories/colors, delete tags

- [ ] **Step 4: Commit**

```bash
git add crates/frontend/src/pages/tags.rs crates/frontend/src/pages/mod.rs crates/frontend/src/app.rs crates/frontend/src/components/nav.rs
git commit -m "feat: add tag management page with CRUD"
```

---

### Task 9: Item-Level Tag Assignment

**Files:**
- Modify: `crates/frontend/src/components/item_row.rs`
- Modify: `crates/frontend/src/pages/list.rs`

- [ ] **Step 1: Extend ItemRow to show tag badges and selector**

Add props to `ItemRow`: `all_tags: Vec<Tag>`, `item_tag_ids: Vec<String>`, `on_tag_toggle: Callback<(String, String)>` (item_id, tag_id).

Show tag badges after the title. Add a small TagSelector button.

- [ ] **Step 2: Update list.rs to fetch and manage item tags**

```rust
// Additional state
let all_tags = RwSignal::new(Vec::<Tag>::new());
let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());

// Fetch tags and item-tag-links alongside items
leptos::task::spawn_local(async move {
    if let Ok(fetched) = api::fetch_tags().await {
        all_tags.set(fetched);
    }
    if let Ok(links) = api::fetch_item_tag_links().await {
        item_tag_links.set(links);
    }
});

// Tag toggle callback with optimistic update
let on_tag_toggle = Callback::new(move |(item_id, tag_id): (String, String)| {
    let has_tag = item_tag_links.read().iter().any(|l| l.item_id == item_id && l.tag_id == tag_id);
    if has_tag {
        item_tag_links.update(|links| links.retain(|l| !(l.item_id == item_id && l.tag_id == tag_id)));
        leptos::task::spawn_local(async move {
            let _ = api::remove_tag_from_item(&item_id, &tag_id).await;
        });
    } else {
        item_tag_links.update(|links| links.push(ItemTagLink { item_id: item_id.clone(), tag_id: tag_id.clone() }));
        leptos::task::spawn_local(async move {
            let _ = api::assign_tag_to_item(&item_id, &tag_id).await;
        });
    }
});
```

- [ ] **Step 3: Verify it works**

Run: `just dev`
Test: Open a list, assign tags to items, verify badges appear, toggle tags on/off

- [ ] **Step 4: Commit**

```bash
git add crates/frontend/src/components/item_row.rs crates/frontend/src/pages/list.rs
git commit -m "feat: add tag assignment and display on list items"
```

---

## Verification

After all tasks:

1. `just check` — workspace compiles
2. `just lint` — no clippy warnings, fmt ok
3. `just dev` — start locally and test:
   - Create tags (different categories, colors) at `/tags`
   - Create lists with different types (verify icons/labels on home)
   - Assign tags to lists (home page shows tag badges)
   - Filter lists by tag on home page
   - Open a list, assign tags to items
   - Toggle tags on/off, verify badges update
   - Delete tags, verify cleanup
4. `just deploy` — deploy and smoke test production
