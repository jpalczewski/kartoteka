# Data Model

Source of truth: `crates/shared/src/lib.rs` (Rust types) + `crates/api/migrations/` (DB schema).

---

## Entity Overview

```
User (Hanko-managed, no local table)
 ├── List (1:N)
 │    ├── Item (1:N)
 │    │    └── item_tags (M:N) ──► Tag
 │    └── list_tags (M:N) ──────► Tag
 └── Tag (1:N)
```

---

## Tables

### `lists`

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| `id` | TEXT | PRIMARY KEY | UUID v4 |
| `user_id` | TEXT | NOT NULL | Hanko user ID |
| `name` | TEXT | NOT NULL | |
| `list_type` | TEXT | NOT NULL, DEFAULT `'custom'` | `custom` / `shopping` / `packing` / `project` |
| `created_at` | TEXT | NOT NULL, DEFAULT `datetime('now')` | ISO 8601 |
| `updated_at` | TEXT | NOT NULL, DEFAULT `datetime('now')` | ISO 8601 |

Index: `idx_lists_user_id` on `user_id`.

---

### `items`

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| `id` | TEXT | PRIMARY KEY | UUID v4 |
| `list_id` | TEXT | NOT NULL, FK → `lists(id)` ON DELETE CASCADE | |
| `title` | TEXT | NOT NULL | |
| `description` | TEXT | nullable | |
| `completed` | INTEGER | NOT NULL, DEFAULT `0` | D1 returns as float (`0.0`/`1.0`), deserialized via `bool_from_number` |
| `position` | INTEGER | NOT NULL, DEFAULT `0` | ordering within list |
| `created_at` | TEXT | NOT NULL, DEFAULT `datetime('now')` | |
| `updated_at` | TEXT | NOT NULL, DEFAULT `datetime('now')` | |

Indexes: `idx_items_list_id` on `list_id`, `idx_items_position` on `(list_id, position)`.

---

### `tags`

| Column | Type | Constraints | Notes |
|--------|------|-------------|-------|
| `id` | TEXT | PRIMARY KEY | UUID v4 |
| `user_id` | TEXT | NOT NULL | Hanko user ID |
| `name` | TEXT | NOT NULL | |
| `color` | TEXT | NOT NULL, DEFAULT `'#888888'` | hex color |
| `parent_tag_id` | TEXT | nullable, FK → `tags(id)` ON DELETE SET NULL | for tag hierarchy (unlimited depth) |
| `created_at` | TEXT | NOT NULL, DEFAULT `datetime('now')` | |

Indexes: `idx_tags_user` on `user_id`.

---

### `item_tags` (junction)

| Column | Type | Constraints |
|--------|------|-------------|
| `item_id` | TEXT | NOT NULL, FK → `items(id)` ON DELETE CASCADE |
| `tag_id` | TEXT | NOT NULL, FK → `tags(id)` ON DELETE CASCADE |

PRIMARY KEY: `(item_id, tag_id)`.
Indexes: `idx_item_tags_item`, `idx_item_tags_tag`.

---

### `list_tags` (junction)

| Column | Type | Constraints |
|--------|------|-------------|
| `list_id` | TEXT | NOT NULL, FK → `lists(id)` ON DELETE CASCADE |
| `tag_id` | TEXT | NOT NULL, FK → `tags(id)` ON DELETE CASCADE |

PRIMARY KEY: `(list_id, tag_id)`.
Indexes: `idx_list_tags_list`, `idx_list_tags_tag`.

---

## Rust Types (`crates/shared/src/lib.rs`)

### Domain

```rust
pub enum ListType { Shopping, Packing, Project, Custom }

pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub list_type: ListType,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Item {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,   // deserialized from INTEGER via bool_from_number
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub parent_tag_id: Option<String>,
    pub created_at: String,
}

pub struct ItemTagLink { pub item_id: String, pub tag_id: String }
pub struct ListTagLink { pub list_id: String, pub tag_id: String }
```

### Request DTOs

```rust
pub struct CreateListRequest { pub name: String, pub list_type: ListType }
pub struct UpdateListRequest { pub name: Option<String>, pub list_type: Option<ListType> }

pub struct CreateItemRequest { pub title: String, pub description: Option<String> }
pub struct UpdateItemRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
    pub position: Option<i32>,
}

pub struct CreateTagRequest {
    pub name: String,
    pub color: String,
    pub parent_tag_id: Option<String>,
}
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<Option<String>>,  // Some(None) = clear parent
}

pub struct TagAssignment { pub tag_id: String }
```

---

## Notes

- All IDs are UUID v4 strings.
- Timestamps are TEXT in SQLite ISO 8601 format (`datetime('now')`).
- `completed` is stored as `INTEGER` (0/1) in D1, but D1 may return it as a float — handled by the custom `bool_from_number` deserializer.
- Deleting a `List` cascades to `items` and `list_tags`. Deleting an `Item` cascades to `item_tags`. Deleting a `Tag` cascades to `item_tags` and `list_tags`, sets `parent_tag_id` to NULL on child tags.
- Users are managed entirely by Hanko Cloud — no local `users` table.
