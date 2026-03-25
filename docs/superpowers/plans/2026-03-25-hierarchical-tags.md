# Hierarchical Tags Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove tag categories (Context/Priority/Custom), expose hierarchical tag trees in UI, add recursive filtering on tag detail page.

**Architecture:** Remove `TagCategory` enum and `category` column. Reuse existing `parent_tag_id` for tree structure. Add `WITH RECURSIVE` SQL for descendant filtering. Frontend builds tree from flat `Vec<Tag>` using a shared utility, renders with recursive components.

**Tech Stack:** Rust, Leptos 0.7 CSR, Cloudflare Workers, D1/SQLite, sqlx-d1 0.3, gloo-net 0.6, DaisyUI 5

**Spec:** `docs/superpowers/specs/2026-03-25-hierarchical-tags-design.md`

**Important conventions:**
- D1 returns booleans as float (0.0/1.0) — use `#[serde(deserialize_with = "bool_from_number")]` on all bool fields deserialized from DB. The `bool_from_number` function is in `crates/shared/src/lib.rs`.
- D1 parameters use `JsValue` — use `JsValue::NULL` for `None` values, convert bools to `i32` (1/0).
- Leptos 0.7: `LocalResource` (not `Resource`), `RwSignal`, `Callback`. Futures from `gloo-net` are not `Send`.
- Use `Serialize` on DTOs sent from frontend, `Deserialize` on DTOs received by API.
- `gloo-net` 0.6: `Request::get()` returns `RequestBuilder`, `.body()` returns `Result<Request>`.

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `crates/api/migrations/0005_remove_tag_category.sql` | Drop `category` column and index |
| Modify | `crates/shared/src/lib.rs` | Remove `TagCategory` enum, remove `category` from `Tag`, `CreateTagRequest`, `UpdateTagRequest` |
| Modify | `crates/api/src/handlers/tags.rs` | Remove category from CRUD queries, add recursive `tag_items`, add cycle prevention in `update` |
| Modify | `crates/frontend/src/api.rs` | Add `recursive` param to `fetch_tag_items` |
| Create | `crates/frontend/src/components/tag_tree.rs` | Shared `build_tag_tree` utility + `TagTreeNode` component |
| Modify | `crates/frontend/src/components/mod.rs` | Export `tag_tree` module |
| Modify | `crates/frontend/src/components/tag_selector.rs` | Tree with expand/collapse instead of flat list |
| Modify | `crates/frontend/src/pages/tags.rs` | Tree view, remove category grouping, inline add-child |
| Modify | `crates/frontend/src/pages/tag_detail.rs` | Breadcrumbs, recursive toggle, subtags section |

## Compilation Order Note

Tasks 1-2 remove `TagCategory` from shared types. After these changes, both API and frontend will have compile errors until their callers are updated (Task 3 for API, Tasks 5-7 for frontend). **Do not run `cargo check` on the full workspace between Task 2 and completing Task 3 (API) or Task 5 (frontend).** Check individual crates only after updating their callers.

---

### Task 1: Database Migration

**Files:**
- Create: `crates/api/migrations/0005_remove_tag_category.sql`

- [ ] **Step 1: Create migration file**

```sql
-- Remove tag categories — tags are now just tags, organized by hierarchy
ALTER TABLE tags DROP COLUMN category;
DROP INDEX IF EXISTS idx_tags_user_cat;
```

- [ ] **Step 2: Commit**

```bash
git add crates/api/migrations/0005_remove_tag_category.sql
git commit -m "feat: migration to drop tag category column and index"
```

---

### Task 2: Update Shared Models

**Files:**
- Modify: `crates/shared/src/lib.rs`

- [ ] **Step 1: Delete `TagCategory` enum**

Delete lines 103-109 (the entire `TagCategory` enum with its derives and variants).

- [ ] **Step 2: Update `Tag` struct**

Remove the `category` field. The struct (lines 111-120) becomes:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub parent_tag_id: Option<String>,
    pub created_at: String,
}
```

- [ ] **Step 3: Update `CreateTagRequest`**

Remove `category` field. The struct (lines 122-128) becomes:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: String,
    pub parent_tag_id: Option<String>,
}
```

- [ ] **Step 4: Update `UpdateTagRequest`**

Remove `category` field. The struct (lines 130-136) becomes:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<Option<String>>,
}
```

- [ ] **Step 5: Verify shared crate compiles**

Run: `cargo check -p kartoteka-shared`
Expected: Compiles OK

- [ ] **Step 6: Commit**

```bash
git add crates/shared/src/lib.rs
git commit -m "feat: remove TagCategory enum and category fields from shared models"
```

---

### Task 3: Update API Handlers

**Files:**
- Modify: `crates/api/src/handlers/tags.rs`

- [ ] **Step 1: Update `list_all` query**

Replace the SELECT query (line 9) — remove `category` from column list and ORDER BY:

```rust
let result = d1
    .prepare("SELECT id, user_id, name, color, parent_tag_id, created_at FROM tags WHERE user_id = ?1 ORDER BY name")
    .bind(&[user_id.into()])?
    .all()
    .await?;
```

- [ ] **Step 2: Update `create` handler**

Remove `category_str` serialization logic (lines 22-26). Remove `category_str.into()` from bind params. Update INSERT query:

```rust
d1.prepare("INSERT INTO tags (id, user_id, name, color, parent_tag_id) VALUES (?1, ?2, ?3, ?4, ?5)")
    .bind(&[id.clone().into(), user_id.into(), body.name.into(), body.color.into(), parent_val])?
    .run()
    .await?;
```

Update the SELECT that re-fetches the created tag:

```rust
let tag = d1
    .prepare("SELECT id, user_id, name, color, parent_tag_id, created_at FROM tags WHERE id = ?1")
    .bind(&[id.into()])?
    .first::<Tag>(None)
    .await?
    .ok_or_else(|| Error::from("Failed to create tag"))?;
```

- [ ] **Step 3: Update `update` handler**

Remove the `if let Some(category)` block (lines 81-90). Update the final SELECT:

```rust
let tag = d1
    .prepare("SELECT id, user_id, name, color, parent_tag_id, created_at FROM tags WHERE id = ?1")
    .bind(&[id.into()])?
    .first::<Tag>(None)
    .await?
    .ok_or_else(|| Error::from("Not found"))?;
```

- [ ] **Step 4: Add cycle prevention to `update` handler**

In the `if let Some(parent) = &body.parent_tag_id` block, before the UPDATE, add cycle detection. A tag cannot be its own ancestor:

```rust
if let Some(parent) = &body.parent_tag_id {
    if let Some(new_parent_id) = parent {
        // Self-reference check first (no DB call needed)
        if new_parent_id == &id {
            return Response::error("Cannot set parent: tag cannot be its own parent", 400);
        }
        // Cycle prevention: check if new parent is a descendant of this tag.
        // Walk DOWN from this tag's children — if new_parent_id appears among descendants, it's a cycle.
        let cycle_check = d1
            .prepare(
                "WITH RECURSIVE descendants AS ( \
                 SELECT id FROM tags WHERE parent_tag_id = ?1 \
                 UNION ALL \
                 SELECT t.id FROM tags t JOIN descendants d ON t.parent_tag_id = d.id \
                 ) SELECT 1 FROM descendants WHERE id = ?2 LIMIT 1"
            )
            .bind(&[JsValue::from(id.as_str()), JsValue::from(new_parent_id.as_str())])?
            .first::<serde_json::Value>(None)
            .await?;
        if cycle_check.is_some() {
            return Response::error("Cannot set parent: would create a cycle", 400);
        }
    }

    let parent_val: JsValue = match parent {
        Some(p) => p.as_str().into(),
        None => JsValue::NULL,
    };
    d1.prepare("UPDATE tags SET parent_tag_id = ?1 WHERE id = ?2")
        .bind(&[parent_val, id.clone().into()])?
        .run()
        .await?;
}
```

Note: `?1` = tag being edited, `?2` = proposed new parent. The CTE walks down from the tag's children. If the proposed parent appears among descendants, setting it would create a cycle.

- [ ] **Step 5: Update `tag_items` with recursive query**

Replace the entire `tag_items` function body. Read `recursive` query param from URL (default `true`):

```rust
/// GET /api/tags/:id/items
pub async fn tag_items(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let tag_id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify tag belongs to user
    let tag_check = d1
        .prepare("SELECT id FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[tag_id.clone().into(), user_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if tag_check.is_none() {
        return Response::error("Not found", 404);
    }

    // Check recursive param (default: true)
    let url = req.url()?;
    let recursive = url
        .query_pairs()
        .find(|(k, _)| k == "recursive")
        .map(|(_, v)| v != "false")
        .unwrap_or(true);

    let rows = if recursive {
        d1.prepare(
            "WITH RECURSIVE tag_tree AS ( \
             SELECT id FROM tags WHERE id = ?1 AND user_id = ?2 \
             UNION ALL \
             SELECT t.id FROM tags t JOIN tag_tree tt ON t.parent_tag_id = tt.id WHERE t.user_id = ?2 \
             ) \
             SELECT DISTINCT i.id, i.list_id, i.title, i.description, i.completed, i.position, \
             i.quantity, i.actual_quantity, i.unit, i.due_date, i.due_time, \
             i.created_at, i.updated_at, l.name as list_name \
             FROM items i \
             JOIN item_tags it ON it.item_id = i.id \
             JOIN tag_tree tt ON it.tag_id = tt.id \
             JOIN lists l ON l.id = i.list_id \
             ORDER BY l.name, i.position"
        )
        .bind(&[tag_id.into(), user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?
    } else {
        d1.prepare(
            "SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position, \
             i.quantity, i.actual_quantity, i.unit, i.due_date, i.due_time, \
             i.created_at, i.updated_at, l.name as list_name \
             FROM items i \
             JOIN item_tags it ON it.item_id = i.id \
             JOIN lists l ON l.id = i.list_id \
             WHERE it.tag_id = ?1 \
             ORDER BY l.name, i.position"
        )
        .bind(&[tag_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?
    };

    Response::from_json(&rows)
}
```

- [ ] **Step 6: Verify API crate compiles**

Run: `cargo check -p kartoteka-api`
Expected: Compiles OK

- [ ] **Step 7: Commit**

```bash
git add crates/api/src/handlers/tags.rs
git commit -m "feat: remove category from tag handlers, add recursive filtering and cycle prevention"
```

---

### Task 4: Frontend API + Tag Tree Utility

**Files:**
- Modify: `crates/frontend/src/api.rs`
- Create: `crates/frontend/src/components/tag_tree.rs`
- Modify: `crates/frontend/src/components/mod.rs`

- [ ] **Step 1: Update `fetch_tag_items` signature**

In `crates/frontend/src/api.rs`, replace the `fetch_tag_items` function (lines 248-257):

```rust
pub async fn fetch_tag_items(tag_id: &str, recursive: bool) -> Result<Vec<serde_json::Value>, String> {
    let url = format!("{API_BASE}/tags/{tag_id}/items?recursive={recursive}");
    get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Create `tag_tree.rs` with `build_tag_tree` utility**

Create `crates/frontend/src/components/tag_tree.rs`:

```rust
use kartoteka_shared::Tag;
use std::collections::HashMap;

/// A tag with its children, used for rendering tag trees.
#[derive(Clone, Debug)]
pub struct TagNode {
    pub tag: Tag,
    pub children: Vec<TagNode>,
}

/// Build a tree of TagNodes from a flat Vec<Tag>.
/// Returns only root nodes (tags with no parent or whose parent is not in the list).
/// Children are sorted alphabetically by name within each parent.
pub fn build_tag_tree(tags: &[Tag]) -> Vec<TagNode> {
    let tag_ids: std::collections::HashSet<&str> = tags.iter().map(|t| t.id.as_str()).collect();

    // Group children by parent_tag_id
    let mut children_map: HashMap<&str, Vec<&Tag>> = HashMap::new();
    let mut roots: Vec<&Tag> = Vec::new();

    for tag in tags {
        match &tag.parent_tag_id {
            Some(pid) if tag_ids.contains(pid.as_str()) => {
                children_map.entry(pid.as_str()).or_default().push(tag);
            }
            _ => roots.push(tag),
        }
    }

    // Sort roots alphabetically
    roots.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Sort children within each parent
    for children in children_map.values_mut() {
        children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    fn build_subtree<'a>(tag: &'a Tag, children_map: &HashMap<&str, Vec<&'a Tag>>) -> TagNode {
        let children = children_map
            .get(tag.id.as_str())
            .map(|kids| kids.iter().map(|t| build_subtree(t, children_map)).collect())
            .unwrap_or_default();
        TagNode {
            tag: tag.clone(),
            children,
        }
    }

    roots.iter().map(|t| build_subtree(t, &children_map)).collect()
}

/// Walk ancestors of a tag (from tag up to root). Returns list from root to tag.
pub fn build_breadcrumb(tags: &[Tag], tag_id: &str) -> Vec<Tag> {
    let tag_map: HashMap<&str, &Tag> = tags.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut path = Vec::new();
    let mut current_id = Some(tag_id);

    let mut visited = std::collections::HashSet::new();
    while let Some(id) = current_id {
        if !visited.insert(id.to_string()) {
            break; // cycle protection
        }
        if let Some(tag) = tag_map.get(id) {
            path.push((*tag).clone());
            current_id = tag.parent_tag_id.as_deref();
        } else {
            break;
        }
    }

    path.reverse(); // root first
    path
}
```

- [ ] **Step 3: Export `tag_tree` module**

In `crates/frontend/src/components/mod.rs`, add:

```rust
pub mod tag_tree;
```

- [ ] **Step 4: Commit**

```bash
git add crates/frontend/src/api.rs crates/frontend/src/components/tag_tree.rs crates/frontend/src/components/mod.rs
git commit -m "feat: add tag tree utility, breadcrumb builder, and recursive fetch_tag_items param"
```

---

### Task 5: Tags Page — Tree View

**Files:**
- Modify: `crates/frontend/src/pages/tags.rs`

- [ ] **Step 1: Replace entire TagsPage**

Replace the full content of `crates/frontend/src/pages/tags.rs`. Remove `category_label`, category grouping, category dropdown. Add tree rendering with inline add-child:

```rust
use crate::api;
use crate::components::add_input::AddInput;
use crate::components::tag_badge::TagBadge;
use crate::components::tag_tree::{build_tag_tree, TagNode};
use kartoteka_shared::{CreateTagRequest, Tag};
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn TagsPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj się"</a></p> }.into_any();
    }

    let tags = RwSignal::new(Vec::<Tag>::new());
    let (loading, set_loading) = signal(true);
    let (new_color, set_new_color) = signal("#e94560".to_string());
    // Which parent tag is currently in "add child" mode (None = top-level add only)
    let adding_child_to = RwSignal::new(Option::<String>::None);

    // Initial fetch
    leptos::task::spawn_local(async move {
        if let Ok(fetched) = api::fetch_tags().await {
            tags.set(fetched);
        }
        set_loading.set(false);
    });

    let do_create = Callback::new(move |(name, parent_id): (String, Option<String>)| {
        let color = new_color.get_untracked();
        leptos::task::spawn_local(async move {
            let req = CreateTagRequest {
                name,
                color,
                parent_tag_id: parent_id,
            };
            if let Ok(tag) = api::create_tag(&req).await {
                tags.update(|t| t.push(tag));
            }
            adding_child_to.set(None);
        });
    });

    let on_create_root = Callback::new(move |name: String| {
        do_create.run((name, None));
    });

    let on_delete = Callback::new(move |tag_id: String| {
        tags.update(|t| t.retain(|tag| tag.id != tag_id));
        leptos::task::spawn_local(async move {
            let _ = api::delete_tag(&tag_id).await;
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-6">"Tagi"</h2>

            <div class="flex gap-2 items-center mb-4">
                <input
                    type="color"
                    aria-label="Kolor tagu"
                    class="w-8 h-8 rounded cursor-pointer border-0 p-0"
                    prop:value=move || new_color.get()
                    on:input=move |ev| set_new_color.set(event_target_value(&ev))
                />
                <AddInput placeholder="Nowy tag..." button_label="Dodaj" on_submit=on_create_root />
            </div>

            {move || {
                if loading.get() {
                    return view! { <p>"Wczytywanie..."</p> }.into_any();
                }
                let all_tags = tags.get();
                if all_tags.is_empty() {
                    return view! { <p class="text-center text-base-content/50 py-12">"Brak tagów. Dodaj pierwszy!"</p> }.into_any();
                }

                let tree = build_tag_tree(&all_tags);
                view! {
                    <div>
                        {tree.into_iter().map(|node| {
                            view! {
                                <TagTreeRow
                                    node=node
                                    depth=0
                                    on_delete=on_delete
                                    adding_child_to=adding_child_to
                                    new_color=new_color
                                    do_create=do_create
                                />
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}

#[component]
fn TagTreeRow(
    node: TagNode,
    depth: usize,
    on_delete: Callback<String>,
    adding_child_to: RwSignal<Option<String>>,
    new_color: ReadSignal<String>,
    do_create: Callback<(String, Option<String>)>,
) -> impl IntoView {
    let tag = node.tag;
    let children = node.children;
    let tid = tag.id.clone();
    let tid_link = tag.id.clone();
    let tid_add = tag.id.clone();
    let tid_delete = tag.id.clone();
    let padding = format!("padding-left: {}rem;", depth as f64 * 1.0);

    view! {
        <div>
            <div class="flex items-center gap-2 py-1" style=padding.clone()>
                <A href=format!("/tags/{tid_link}") attr:class="no-underline">
                    <TagBadge tag=tag.clone() />
                </A>
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    title="Dodaj podtag"
                    on:click=move |_| {
                        adding_child_to.set(Some(tid_add.clone()));
                    }
                >"+"</button>
                <button
                    class="btn btn-error btn-xs btn-square"
                    on:click=move |_| on_delete.run(tid_delete.clone())
                >"✕"</button>
            </div>

            // Inline add-child form
            {move || {
                let current = adding_child_to.get();
                if current.as_deref() == Some(&tid) {
                    let tid_for_create = tid.clone();
                    let on_submit_child = Callback::new(move |name: String| {
                        do_create.run((name, Some(tid_for_create.clone())));
                    });
                    let child_padding = format!("padding-left: {}rem;", (depth + 1) as f64 * 1.0);
                    view! {
                        <div class="flex gap-2 items-center py-1" style=child_padding>
                            <AddInput placeholder="Nazwa podtagu..." button_label="Dodaj" on_submit=on_submit_child />
                            <button class="btn btn-ghost btn-xs" on:click=move |_| adding_child_to.set(None)>"✕"</button>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            // Children
            {children.into_iter().map(|child| {
                view! {
                    <TagTreeRow
                        node=child
                        depth=depth + 1
                        on_delete=on_delete
                        adding_child_to=adding_child_to
                        new_color=new_color
                        do_create=do_create
                    />
                }
            }).collect_view()}
        </div>
    }
}
```

- [ ] **Step 2: Verify frontend crate compiles**

Run: `cargo check -p kartoteka-frontend`
Expected: Compiles OK (may need adjustments — see Task 8)

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/pages/tags.rs
git commit -m "feat: tags page with tree view and inline add-child"
```

---

### Task 6: Tag Selector — Tree with Expand/Collapse

**Files:**
- Modify: `crates/frontend/src/components/tag_selector.rs`

- [ ] **Step 1: Replace entire TagSelector**

Replace the full content of `crates/frontend/src/components/tag_selector.rs`:

```rust
use crate::components::tag_tree::{build_tag_tree, TagNode};
use kartoteka_shared::Tag;
use leptos::prelude::*;
use std::collections::HashMap;

#[component]
pub fn TagSelector(
    all_tags: Vec<Tag>,
    selected_tag_ids: Vec<String>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    let (open, set_open) = signal(false);
    let expanded = RwSignal::new(HashMap::<String, bool>::new());
    let tree = build_tag_tree(&all_tags);

    view! {
        <div class="relative">
            <button type="button" class="btn btn-ghost btn-xs btn-square" on:click=move |_| set_open.update(|v| *v = !*v)>
                "＋"
            </button>
            <div
                class="absolute right-0 top-full mt-1 bg-base-200 border border-base-300 rounded-box min-w-44 max-h-60 overflow-y-auto z-50 p-2 shadow-lg"
                style:display=move || if open.get() { "block" } else { "none" }
            >
                {tree.into_iter().map(|node| {
                    view! {
                        <TagSelectorNode
                            node=node
                            depth=0
                            selected_tag_ids=selected_tag_ids.clone()
                            on_toggle=on_toggle
                            expanded=expanded
                        />
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

#[component]
fn TagSelectorNode(
    node: TagNode,
    depth: usize,
    selected_tag_ids: Vec<String>,
    on_toggle: Callback<String>,
    expanded: RwSignal<HashMap<String, bool>>,
) -> impl IntoView {
    let tag = node.tag;
    let children = node.children;
    let has_children = !children.is_empty();
    let is_selected = selected_tag_ids.contains(&tag.id);
    let color = tag.color.clone();
    let name = tag.name.clone();
    let tid = tag.id.clone();
    let tid_toggle = tag.id.clone();
    let tid_expand = tag.id.clone();
    let padding = format!("padding-left: {}rem;", depth as f64 * 0.75);

    view! {
        <div>
            <label
                class="flex items-center gap-1.5 px-2 py-1.5 text-sm rounded cursor-pointer hover:bg-base-300"
                style=format!("{padding} border-left: 3px solid {color};")
            >
                {has_children.then(|| {
                    let tid_e = tid_expand.clone();
                    view! {
                        <button
                            class="btn btn-ghost btn-xs btn-square p-0 min-h-0 h-4 w-4"
                            on:click=move |ev| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                expanded.update(|m| {
                                    let v = m.entry(tid_e.clone()).or_insert(false);
                                    *v = !*v;
                                });
                            }
                        >
                            {move || {
                                let is_expanded = expanded.get().get(&tid_expand).copied().unwrap_or(false);
                                if is_expanded { "▼" } else { "▶" }
                            }}
                        </button>
                    }
                })}
                <input
                    type="checkbox"
                    class="checkbox checkbox-secondary checkbox-xs"
                    checked=is_selected
                    on:change=move |_| on_toggle.run(tid_toggle.clone())
                />
                {name}
            </label>
            {move || {
                let is_expanded = expanded.get().get(&tid).copied().unwrap_or(false);
                if is_expanded && has_children {
                    children.clone().into_iter().map(|child| {
                        view! {
                            <TagSelectorNode
                                node=child
                                depth=depth + 1
                                selected_tag_ids=selected_tag_ids.clone()
                                on_toggle=on_toggle
                                expanded=expanded
                            />
                        }
                    }).collect_view().into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </div>
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/frontend/src/components/tag_selector.rs
git commit -m "feat: tag selector with hierarchical tree and expand/collapse"
```

---

### Task 7: Tag Detail Page — Breadcrumbs, Toggle, Subtags

**Files:**
- Modify: `crates/frontend/src/pages/tag_detail.rs`

- [ ] **Step 1: Replace entire TagDetailPage**

Replace the full content of `crates/frontend/src/pages/tag_detail.rs`:

```rust
use crate::api;
use crate::components::tag_badge::TagBadge;
use crate::components::tag_tree::build_breadcrumb;
use kartoteka_shared::Tag;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use std::collections::BTreeMap;

#[component]
pub fn TagDetailPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj sie"</a></p> }.into_any();
    }

    let params = use_params_map();
    let tag_id = move || params.read().get("id").unwrap_or_default();

    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let tag = RwSignal::new(Option::<Tag>::None);
    let items = RwSignal::new(Vec::<serde_json::Value>::new());
    let (loading, set_loading) = signal(true);
    let (recursive, set_recursive) = signal(true);

    // Fetch data reactively — re-runs when tag_id changes (e.g. navigating between subtags)
    let _resource = LocalResource::new(move || {
        let tid = tag_id();
        let rec = recursive.get();
        async move {
            if let Ok(tags) = api::fetch_tags().await {
                tag.set(tags.iter().find(|t| t.id == tid).cloned());
                all_tags.set(tags);
            }
            if let Ok(fetched) = api::fetch_tag_items(&tid, rec).await {
                items.set(fetched);
            }
            set_loading.set(false);
        }
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            {move || {
                if loading.get() {
                    return view! { <p>"Wczytywanie..."</p> }.into_any();
                }
                match tag.get() {
                    None => view! { <p>"Nie znaleziono tagu"</p> }.into_any(),
                    Some(t) => {
                        let color = t.color.clone();
                        let tags_for_breadcrumb = all_tags.get();
                        let breadcrumb = build_breadcrumb(&tags_for_breadcrumb, &t.id);
                        let all_items = items.get();

                        // Direct children of this tag
                        let children: Vec<Tag> = tags_for_breadcrumb.iter()
                            .filter(|tag| tag.parent_tag_id.as_deref() == Some(&t.id))
                            .cloned()
                            .collect();

                        // Group items by list_name
                        let mut groups: BTreeMap<(String, String), Vec<serde_json::Value>> = BTreeMap::new();
                        for item in all_items {
                            let list_name = item.get("list_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("(bez listy)")
                                .to_string();
                            let list_id = item.get("list_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            groups.entry((list_id, list_name)).or_default().push(item);
                        }

                        view! {
                            <div>
                                // Breadcrumb
                                {if breadcrumb.len() > 1 {
                                    view! {
                                        <div class="text-sm text-base-content/50 mb-2 flex items-center gap-1">
                                            {breadcrumb.iter().enumerate().map(|(i, bt)| {
                                                let is_last = i == breadcrumb.len() - 1;
                                                let bt_id = bt.id.clone();
                                                let bt_name = bt.name.clone();
                                                if is_last {
                                                    view! { <span class="font-semibold">{bt_name}</span> }.into_any()
                                                } else {
                                                    view! {
                                                        <span>
                                                            <A href=format!("/tags/{bt_id}") attr:class="link link-hover">{bt_name}</A>
                                                            " > "
                                                        </span>
                                                    }.into_any()
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}

                                // Tag header
                                <h2 class="text-2xl font-bold mb-4 flex items-center gap-2">
                                    <span
                                        class="inline-block w-4 h-4 rounded-full"
                                        style=format!("background: {color}")
                                    ></span>
                                    {t.name}
                                </h2>

                                // Recursive toggle
                                <label class="flex items-center gap-2 cursor-pointer mb-4">
                                    <input
                                        type="checkbox"
                                        class="toggle toggle-sm toggle-primary"
                                        prop:checked=move || recursive.get()
                                        on:change=move |_| set_recursive.update(|v| *v = !*v)
                                    />
                                    <span class="text-sm">"Uwzględnij podtagi"</span>
                                </label>

                                // Items grouped by list
                                {if groups.is_empty() {
                                    view! {
                                        <p class="text-center text-base-content/50 py-12">
                                            "Brak elementów z tym tagiem"
                                        </p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            {groups.into_iter().map(|((list_id, list_name), group_items)| {
                                                view! {
                                                    <div class="mb-6">
                                                        <h4 class="text-sm font-semibold uppercase tracking-wide mb-2 text-base-content/70">
                                                            <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                                                                {list_name}
                                                            </A>
                                                        </h4>
                                                        {group_items.into_iter().map(|item| {
                                                            let title = item.get("title")
                                                                .and_then(|v| v.as_str())
                                                                .unwrap_or("")
                                                                .to_string();
                                                            let completed = item.get("completed")
                                                                .map(|v| v.as_f64().unwrap_or(0.0) != 0.0 || v.as_bool().unwrap_or(false))
                                                                .unwrap_or(false);
                                                            view! {
                                                                <div class="flex items-center gap-2 py-1 pl-2">
                                                                    <span class=if completed { "text-base-content/40" } else { "" }>
                                                                        {if completed { "\u{2611}" } else { "\u{2610}" }}
                                                                    </span>
                                                                    <span class=if completed { "line-through text-base-content/40" } else { "" }>
                                                                        {title}
                                                                    </span>
                                                                </div>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }}

                                // Subtags section
                                {if !children.is_empty() {
                                    view! {
                                        <div class="mt-8">
                                            <h3 class="text-xs text-base-content/50 uppercase tracking-wider mb-2">"Podtagi"</h3>
                                            <div class="flex flex-wrap gap-2">
                                                {children.into_iter().map(|child| {
                                                    let child_id = child.id.clone();
                                                    view! {
                                                        <A href=format!("/tags/{child_id}") attr:class="no-underline">
                                                            <TagBadge tag=child />
                                                        </A>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
    .into_any()
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/frontend/src/pages/tag_detail.rs
git commit -m "feat: tag detail with breadcrumbs, recursive toggle, and subtags section"
```

---

### Task 8: Full Workspace Verification

- [ ] **Step 1: Check full workspace compiles**

Run: `cargo check`
Expected: All crates compile without errors

- [ ] **Step 2: Fix any remaining compile errors**

Common issues:
- `do_create` closure may need explicit type annotations or `move` adjustments
- `TagSelectorNode` recursive component may need `#[allow(unconditional_recursion)]` or boxing
- Missing imports for `HashMap` or `BTreeMap`
- `collect_view()` vs `.collect::<Vec<_>>()` depending on Leptos 0.7 version
- The `Effect::new` return pattern — may need adjustment per Leptos 0.7 API

- [ ] **Step 3: Commit fixes**

```bash
git add -A
git commit -m "fix: resolve compile errors from hierarchical tags implementation"
```

---

### Task 9: Update Documentation

**Files:**
- Modify: `docs/model/data-model.md` (if it exists and references TagCategory)

- [ ] **Step 1: Update data model docs**

Remove `TagCategory` references from `docs/model/data-model.md`. Update `Tag`, `CreateTagRequest`, `UpdateTagRequest` to match new structs (without `category` fields).

- [ ] **Step 2: Commit**

```bash
git add docs/model/data-model.md
git commit -m "docs: update data model to reflect removed tag categories"
```
