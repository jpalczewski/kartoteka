# Hierarchical Tags + Remove Tag Categories

## Goal

Replace flat tag categories (Context/Priority/Custom) with a single unified tag system supporting arbitrary-depth parent-child hierarchies. Simplify the model while adding more powerful organization.

## Decisions

- **Hierarchy depth:** Unlimited (any tag can be a parent)
- **Tagging:** Flat — you assign a specific tag, no automatic inheritance up/down
- **Filtering:** Propagation by default — filtering by a parent tag returns items from all descendant tags. Toggle on tag detail page to disable.
- **UI pattern:** Tree with indentation (tag picker, tags page, tag detail breadcrumbs)
- **Approach:** Minimal migration — reuse existing `parent_tag_id`, use `WITH RECURSIVE` SQL for propagation

## What Gets Removed

- `TagCategory` enum (`Context`, `Priority`, `Custom`) from shared models
- `category` column from `tags` table (migration)
- `category` field from `Tag`, `CreateTagRequest`, `UpdateTagRequest` structs
- `category_label()` function from frontend
- Category grouping on tags page
- Category dropdown in tag creation form
- Index `idx_tags_user_category`

## Database Migration

```sql
ALTER TABLE tags DROP COLUMN category;
DROP INDEX IF EXISTS idx_tags_user_category;
```

D1 uses SQLite 3.40+ which supports `DROP COLUMN`.

## Shared Models Changes

**Remove:**
```rust
// DELETE entire enum
pub enum TagCategory { Context, Priority, Custom }
```

**Update Tag:**
```rust
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub color: String,
    // REMOVED: pub category: TagCategory,
    pub parent_tag_id: Option<String>,
    pub created_at: String,
}
```

**Update CreateTagRequest:**
```rust
pub struct CreateTagRequest {
    pub name: String,
    pub color: String,
    // REMOVED: pub category: TagCategory,
    pub parent_tag_id: Option<String>,
}
```

**Update UpdateTagRequest:**
```rust
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    // REMOVED: pub category: Option<TagCategory>,
    pub parent_tag_id: Option<Option<String>>,
}
```

## API Changes

### Tags CRUD (`handlers/tags.rs`)

**`list_all`:** Remove `category` from SELECT. Order by `name` only (was `category, name`).

**`create`:** Remove `category` from INSERT and its bind. Remove category serialization logic.

**`update`:** Remove `category` update branch.

**`tag_items` — recursive filtering:**

New query with `WITH RECURSIVE`:
```sql
WITH RECURSIVE tag_tree AS (
    SELECT id FROM tags WHERE id = ?1
    UNION ALL
    SELECT t.id FROM tags t JOIN tag_tree tt ON t.parent_tag_id = tt.id
)
SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position,
       i.quantity, i.actual_quantity, i.unit, i.due_date, i.due_time,
       i.created_at, i.updated_at, l.name as list_name
FROM items i
JOIN item_tags it ON it.item_id = i.id
JOIN tag_tree tt ON it.tag_id = tt.id
JOIN lists l ON l.id = i.list_id
ORDER BY l.name, i.position
```

Query param `?recursive=false` disables propagation (uses simple `WHERE it.tag_id = ?1` like current).

### No changes to:
- `assign_to_item`, `remove_from_item`, `assign_to_list`, `remove_from_list`
- `all_item_tag_links`, `all_list_tag_links`
- `delete` (cascade already handles children via `ON DELETE SET NULL`)

## Frontend: Tags Page (`pages/tags.rs`)

**Remove:**
- `category_label()` function
- Category grouping (iterating over `[Context, Priority, Custom]`)
- Category `<select>` dropdown in creation form

**New structure:**
- Build tree from flat `Vec<Tag>` by grouping on `parent_tag_id`
- Recursive `TagTreeNode` component renders each tag with:
  - Indentation: `pl-4` per level
  - Tag badge (color + name, clickable → tag detail)
  - "+" button → inline add child (sets `parent_tag_id`)
  - "✕" button → delete tag
  - Recursively renders children
- Top-level creation form: color picker + name input (no category)
- Adding child: same form appears inline under parent when "+" clicked

**Tree layout:**
```
🔴 Zakupy                    [+] [✕]
    🟡 Biedronka             [+] [✕]
    🟡 Lidl                  [+] [✕]
🔵 Praca                     [+] [✕]
    🟣 Projekty              [+] [✕]
        🟣 Kartoteka         [+] [✕]
🟢 Pilne                     [+] [✕]
```

## Frontend: Tag Selector (`components/tag_selector.rs`)

**Current:** Flat checkbox list in dropdown.

**New:** Tree with indentation in same dropdown.
- Parent tags with children have expand/collapse arrow (▶/▼)
- Default: top-level visible, children collapsed
- Click arrow → expand/collapse (does NOT toggle tag)
- Click checkbox → assign/remove tag (same as current)
- Indentation: `pl-4` per level, consistent with tags page

```
▼ ☐ Zakupy
    ☑ Biedronka
    ☐ Lidl
▶ ☐ Praca
☑ Pilne
```

## Frontend: Tag Detail Page (`pages/tag_detail.rs`)

**New features:**
- **Breadcrumb path** under tag header: `Zakupy > Biedronka` with links to parent tags
- **Toggle "Uwzględnij podtagi"** (default: on) — controls `?recursive=` param to API
- **Subtags section** below items: list of direct children as clickable links to their detail pages

**Existing behavior unchanged:**
- Items grouped by list name
- Links to view items in original list context

## Frontend API (`api.rs`)

**`fetch_tag_items`:** Add optional `recursive` param:
```rust
pub async fn fetch_tag_items(tag_id: &str, recursive: bool) -> Result<Vec<serde_json::Value>, String>
```

URL: `/api/tags/{id}/items?recursive={true|false}`

## Files Affected

| Action | File |
|--------|------|
| Create | `crates/api/migrations/NNNN_remove_tag_category.sql` |
| Modify | `crates/shared/src/lib.rs` |
| Modify | `crates/api/src/handlers/tags.rs` |
| Modify | `crates/frontend/src/api.rs` |
| Modify | `crates/frontend/src/pages/tags.rs` |
| Modify | `crates/frontend/src/components/tag_selector.rs` |
| Modify | `crates/frontend/src/pages/tag_detail.rs` |

## Not Changed

- `item_tags`, `list_tags` tables — no schema changes
- `tag_badge.rs` — display component unchanged
- `item_row.rs` — tag integration unchanged (flat tagging, display only)
- Tag assign/remove API endpoints — unchanged
- `list_card.rs`, `home.rs`, `list.rs` — no tag-related changes
