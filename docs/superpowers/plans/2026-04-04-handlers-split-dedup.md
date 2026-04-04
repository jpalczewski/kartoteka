# Handlers Split & Deduplication Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `items.rs` (1301 lines) and `lists.rs` (673 lines) into focused submodule directories mirroring the existing `containers/` pattern, and extract one shared helper to deduplicate `fetch_*_ids` functions.

**Architecture:** Pure structural refactoring. No behavior changes. Each large handler file becomes a directory with focused submodules. One new generic helper `fetch_ordered_ids` replaces three near-identical functions across items, lists, and containers.

**Tech Stack:** Rust, worker crate, existing helpers.rs

---

## File Structure

### New files to create

```
crates/api/src/handlers/items/
  mod.rs          — constants, shared private helpers, pub use
  validation.rs   — ItemTemporalState, ItemQuantityState, all validation fns + tests
  crud.rs         — list_all, get_one, create, update, delete
  placement.rs    — move_item, set_placement, reorder
  calendar.rs     — DateFieldSelector, date helpers, by_date, calendar + tests

crates/api/src/handlers/lists/
  mod.rs          — LIST_SELECT, shared private helpers, pub use
  crud.rs         — list_all, create, get_one, update, delete, list_sublists, create_sublist, reset
  features.rs     — add_feature, remove_feature, touch_list_updated_at
  archive.rs      — list_archived, toggle_archive
  placement.rs    — move_list, set_placement, apply_list_placement, fetch_lists_by_ids
  pin.rs          — toggle_pin
```

### Files to delete

```
crates/api/src/handlers/items.rs
crates/api/src/handlers/lists.rs
```

### Files to modify

```
crates/api/src/helpers.rs          — add fetch_ordered_ids
crates/api/src/handlers/containers/mod.rs — replace fetch_container_ids_in_scope with fetch_ordered_ids
crates/api/src/handlers/containers/reorder.rs — update import
```

### Files that stay unchanged

```
crates/api/src/handlers/mod.rs     — `pub mod items;` and `pub mod lists;` work for both file and directory
crates/api/src/router.rs           — all paths like items::list_all, lists::create stay the same via pub use
```

---

### Task 1: Add `fetch_ordered_ids` to helpers.rs

This replaces three near-identical functions:
- `fetch_item_ids_in_list` in items.rs (single WHERE clause)
- `fetch_list_ids_in_scope` in lists.rs (3 match arms with different WHERE clauses)
- `fetch_container_ids_in_scope` in containers/mod.rs (2 match arms with different WHERE clauses)

Each does: run a `SELECT id FROM ... ORDER BY position ASC, created_at ASC`, deserialize rows into `Vec<String>`.

**Files:**
- Modify: `crates/api/src/helpers.rs`

- [ ] **Step 1: Add `fetch_ordered_ids` to helpers.rs**

Add after `apply_positions` function (around line 254):

```rust
pub async fn fetch_ordered_ids(
    d1: &D1Database,
    query: &str,
    params: &[JsValue],
) -> Result<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct IdRow {
        id: String,
    }

    let result = d1.prepare(query).bind(params)?.all().await?;
    Ok(result
        .results::<IdRow>()?
        .into_iter()
        .map(|row| row.id)
        .collect())
}
```

- [ ] **Step 2: Verify build**

Run: `cd /Users/erxyi/Projekty/kartoteka && cargo check -p kartoteka-api 2>&1 | tail -5`
Expected: compiles (new function is unused for now, that's fine)

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/helpers.rs
git commit -m "refactor: add generic fetch_ordered_ids helper"
```

---

### Task 2: Create items/ directory — mod.rs

**Files:**
- Create: `crates/api/src/handlers/items/mod.rs`

- [ ] **Step 1: Create directory**

```bash
mkdir -p crates/api/src/handlers/items
```

- [ ] **Step 2: Write mod.rs**

This file contains:
- `mod` declarations for all submodules
- `pub use` re-exports for every public handler (must match what router.rs expects)
- Constants `ITEM_COLS`, `DATE_ITEM_COLS`, `MAX_ITEM_TITLE_LENGTH`
- Shared private helpers used by multiple submodules: `list_archived_response`, `check_item_features`

```rust
mod calendar;
mod crud;
mod placement;
mod validation;

pub use calendar::{by_date, calendar};
pub use crud::{create, delete, get_one, list_all, update};
pub use placement::{move_item, reorder, set_placement};

use crate::error::json_error;
use kartoteka_shared::*;
use worker::*;

pub(super) const ITEM_COLS: &str = "id, list_id, title, description, completed, position, quantity, actual_quantity, unit, start_date, start_time, deadline, deadline_time, hard_deadline, created_at, updated_at";

pub(super) const DATE_ITEM_COLS: &str = "i.id, i.list_id, i.title, i.description, i.completed, i.position, \
    i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, \
    i.created_at, i.updated_at, l.name as list_name, l.list_type";

pub(super) const MAX_ITEM_TITLE_LENGTH: usize = 255;

pub(super) fn list_archived_response() -> worker::Result<Response> {
    json_error("list_archived", 409)
}

pub(super) fn check_item_features(
    feature_names: &[String],
    has_date_field: bool,
    has_quantity_field: bool,
) -> worker::Result<Option<Response>> {
    if has_date_field && !feature_names.iter().any(|f| f == FEATURE_DEADLINES) {
        return Ok(Some(
            Response::from_json(&serde_json::json!({
                "error": "feature_required",
                "feature": "deadlines",
                "message": "This list does not have the 'deadlines' feature enabled. Enable it in list settings or retry without date fields."
            }))
            .map(|r| r.with_status(422))?,
        ));
    }
    if has_quantity_field && !feature_names.iter().any(|f| f == FEATURE_QUANTITY) {
        return Ok(Some(
            Response::from_json(&serde_json::json!({
                "error": "feature_required",
                "feature": "quantity",
                "message": "This list does not have the 'quantity' feature enabled. Enable it in list settings or retry without quantity fields."
            }))
            .map(|r| r.with_status(422))?,
        ));
    }
    Ok(None)
}
```

Note: do NOT delete `items.rs` yet — both will exist temporarily causing a compile error. That's expected; we'll delete items.rs in Task 6 after all submodules are created.

---

### Task 3: Create items/validation.rs

**Files:**
- Create: `crates/api/src/handlers/items/validation.rs`

- [ ] **Step 1: Write validation.rs**

This file contains all item validation logic. Move from `items.rs`:
- Structs: `ItemTemporalState` (lines 18-55), `ItemQuantityState` (lines 57-86)
- Functions: `apply_patch_field` (line 94), `validation_field` (line 102), `normalize_title` (line 109), `validate_item_quantity_state` (line 126), `derive_completed_from_quantity_state` (line 138), `validate_date_field` (line 149), `validate_time_field` (line 168), `validate_item_temporal_state` (line 184)
- Tests: `normalize_title_rejects_titles_longer_than_255_chars`, `quantity_validation_rejects_non_positive_quantity_and_negative_actual`, `derived_completion_uses_quantity_and_actual_quantity` (lines 1217-1264)

Imports needed:

```rust
use kartoteka_shared::*;

use super::MAX_ITEM_TITLE_LENGTH;
```

All functions and structs should be `pub(super)` (used by crud.rs but not outside items module).

The `#[cfg(test)] mod tests` block at the bottom should include only the 3 validation tests listed above (not the calendar test).

---

### Task 4: Create items/crud.rs

**Files:**
- Create: `crates/api/src/handlers/items/crud.rs`

- [ ] **Step 1: Write crud.rs**

Move from `items.rs`:
- Handlers: `list_all` (line 364), `get_one` (line 383), `create` (line 435), `update` (line 567), `delete` (line 722)

Imports needed:

```rust
use crate::error::{json_error, validation_error};
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

use super::validation::*;
use super::{ITEM_COLS, check_item_features, list_archived_response};
```

All 5 handlers remain `pub async fn` (re-exported by mod.rs).

---

### Task 5: Create items/placement.rs

**Files:**
- Create: `crates/api/src/handlers/items/placement.rs`

- [ ] **Step 1: Write placement.rs**

Move from `items.rs`:
- Handlers: `move_item` (line 745), `set_placement` (line 813), `reorder` (line 901)
- Helper used only here: the inline `MoveItemRequest` struct inside `move_item` stays inline
- Replace `fetch_item_ids_in_list` with `fetch_ordered_ids` from helpers.

Imports needed:

```rust
use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

use super::{ITEM_COLS, list_archived_response};
```

Replace usages of `fetch_item_ids_in_list(&d1, &list_id)` with:

```rust
fetch_ordered_ids(
    &d1,
    "SELECT id FROM items WHERE list_id = ?1 ORDER BY position ASC, created_at ASC",
    &[list_id.clone().into()],
)
```

There are 3 call sites in this file:
1. `reorder` — `fetch_item_ids_in_list(&d1, &list_id)`
2. `set_placement` — `fetch_item_ids_in_list(&d1, &body.source_list_id)`
3. `set_placement` — `fetch_item_ids_in_list(&d1, &body.target_list_id)`

---

### Task 6: Create items/calendar.rs

**Files:**
- Create: `crates/api/src/handlers/items/calendar.rs`

- [ ] **Step 1: Write calendar.rs**

Move from `items.rs`:
- Enum: `DateFieldSelector` (line 88)
- Functions: `parse_date_field_selector` (line 220), `query_param_with_alias` (line 234), `parse_required_query_date` (line 245), `relevant_date_for_item` (line 272), `keep_item_for_day` (line 286), `date_key_in_range` (line 313), `filter_day_summaries` (line 327)
- Handlers: `by_date` (line 932), `calendar` (line 1040)
- Test: `all_selector_uses_start_date_date_type` (line 1267-1300)

Imports needed:

```rust
use crate::error::validation_error;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

use super::DATE_ITEM_COLS;
```

Private helper functions (`parse_date_field_selector`, `query_param_with_alias`, etc.) remain private to this file. The `validation_field` function is needed here too — since it's also used in validation.rs, move it to mod.rs as `pub(super)` and import from `super::` in both files.

**Important:** `validation_field` is used in both `validation.rs` and `calendar.rs`. Place it in `mod.rs`:

```rust
pub(super) fn validation_field(field: &str, code: &str) -> ValidationFieldError {
    ValidationFieldError {
        field: field.to_string(),
        code: code.to_string(),
    }
}
```

Then import as `use super::validation_field;` in both calendar.rs and validation.rs.

---

### Task 7: Delete items.rs, verify build + tests

**Files:**
- Delete: `crates/api/src/handlers/items.rs`

- [ ] **Step 1: Delete old file**

```bash
rm crates/api/src/handlers/items.rs
```

- [ ] **Step 2: Verify build**

Run: `cargo check -p kartoteka-api 2>&1 | tail -20`
Expected: compiles successfully. If errors, fix imports/visibility.

- [ ] **Step 3: Run tests**

Run: `cargo test -p kartoteka-api 2>&1 | tail -20`
Expected: all existing tests pass (4 tests from items — 3 validation + 1 calendar)

- [ ] **Step 4: Commit**

```bash
git add -A crates/api/src/handlers/items/ && git add crates/api/src/handlers/items.rs
git commit -m "refactor: split items handler into submodules"
```

---

### Task 8: Create lists/ directory — mod.rs

**Files:**
- Create: `crates/api/src/handlers/lists/mod.rs`

- [ ] **Step 1: Create directory**

```bash
mkdir -p crates/api/src/handlers/lists
```

- [ ] **Step 2: Write mod.rs**

This file contains:
- `mod` declarations and `pub use` re-exports
- `LIST_SELECT` constant
- Shared private helpers used across submodules: `placement_filter`, `ensure_parent_list_target`, `list_has_sublists`, `create_list_from_request`

```rust
mod archive;
mod crud;
mod features;
mod pin;
mod placement;

pub use archive::{list_archived, toggle_archive};
pub use crud::{create, create_sublist, delete, get_one, list_all, list_sublists, reset, update};
pub use features::{add_feature, remove_feature};
pub use pin::toggle_pin;
pub use placement::{move_list, set_placement};

use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use wasm_bindgen::JsValue;
use worker::*;

pub(super) const LIST_SELECT: &str = "\
    SELECT l.id, l.user_id, l.name, l.description, l.list_type, \
    l.parent_list_id, l.position, l.archived, l.container_id, l.pinned, l.last_opened_at, \
    l.created_at, l.updated_at, \
    COALESCE((SELECT json_group_array(json_object('name', lf.feature_name, 'config', json(lf.config))) \
    FROM list_features lf WHERE lf.list_id = l.id), '[]') as features \
    FROM lists l";

pub(super) fn placement_filter(
    parent_list_id: Option<&str>,
    container_id: Option<&str>,
) -> (&'static str, Vec<JsValue>) {
    match (parent_list_id, container_id) {
        (Some(parent_id), None) => ("parent_list_id = ?1", vec![parent_id.into()]),
        (None, Some(container_id)) => (
            "parent_list_id IS NULL AND container_id = ?1",
            vec![container_id.into()],
        ),
        (None, None) => ("parent_list_id IS NULL AND container_id IS NULL", vec![]),
        (Some(_), Some(_)) => unreachable!("validated earlier"),
    }
}

pub(super) async fn ensure_parent_list_target(
    d1: &D1Database,
    parent_id: &str,
    user_id: &str,
) -> Result<bool> {
    Ok(d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2 AND parent_list_id IS NULL")
        .bind(&[parent_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?
        .is_some())
}

pub(super) async fn list_has_sublists(d1: &D1Database, list_id: &str) -> Result<bool> {
    Ok(d1
        .prepare("SELECT 1 FROM lists WHERE parent_list_id = ?1 LIMIT 1")
        .bind(&[list_id.into()])?
        .first::<serde_json::Value>(None)
        .await?
        .is_some())
}

pub(super) async fn create_list_from_request(
    d1: &D1Database,
    user_id: &str,
    body: CreateListRequest,
) -> Result<Response> {
    if let Err(code) = body.validate_placement() {
        return json_error(code, 400);
    }

    if let Some(ref parent_id) = body.parent_list_id
        && !ensure_parent_list_target(d1, parent_id, user_id).await?
    {
        return json_error("list_not_found", 404);
    }

    if let Some(ref container_id) = body.container_id
        && !check_ownership(d1, "containers", container_id, user_id).await?
    {
        return json_error("container_not_found", 404);
    }

    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let list_type_str = serde_json::to_value(&body.list_type)
        .map_err(|e| Error::from(e.to_string()))?
        .as_str()
        .unwrap_or("custom")
        .to_string();
    let (filter, params) =
        placement_filter(body.parent_list_id.as_deref(), body.container_id.as_deref());
    let position = next_position(d1, "lists", filter, &params).await?;
    let parent_val = opt_str_to_js(&body.parent_list_id);
    let container_val = opt_str_to_js(&body.container_id);

    d1.prepare(
        "INSERT INTO lists (id, user_id, name, list_type, parent_list_id, container_id, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&[
        id.clone().into(),
        user_id.into(),
        body.name.clone().into(),
        list_type_str.into(),
        parent_val,
        container_val,
        position.into(),
    ])?
    .run()
    .await?;

    let features = body
        .features
        .unwrap_or_else(|| body.list_type.default_features());
    for feature in &features {
        let config_str = feature.config.to_string();
        d1.prepare("INSERT INTO list_features (list_id, feature_name, config) VALUES (?1, ?2, ?3)")
            .bind(&[
                id.clone().into(),
                feature.name.clone().into(),
                config_str.into(),
            ])?
            .run()
            .await?;
    }

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create list"))?;

    Ok(Response::from_json(&list)?.with_status(201))
}
```

---

### Task 9: Create lists/crud.rs

**Files:**
- Create: `crates/api/src/handlers/lists/crud.rs`

- [ ] **Step 1: Write crud.rs**

Move from `lists.rs`:
- Handlers: `list_all` (line 267), `create` (line 282), `get_one` (line 290), `update` (line 315), `delete` (line 374), `list_sublists` (line 387), `create_sublist` (line 478), `reset` (line 447)

Imports needed:

```rust
use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

use super::{LIST_SELECT, create_list_from_request, ensure_parent_list_target};
```

All handlers remain `pub async fn`.

---

### Task 10: Create lists/features.rs

**Files:**
- Create: `crates/api/src/handlers/lists/features.rs`

- [ ] **Step 1: Write features.rs**

Move from `lists.rs`:
- Helper: `touch_list_updated_at` (line 53)
- Handlers: `add_feature` (line 501), `remove_feature` (line 549)

Imports needed:

```rust
use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

use super::LIST_SELECT;
```

`touch_list_updated_at` can be private to this file (only used by features).

---

### Task 11: Create lists/archive.rs

**Files:**
- Create: `crates/api/src/handlers/lists/archive.rs`

- [ ] **Step 1: Write archive.rs**

Move from `lists.rs`:
- Handlers: `list_archived` (line 408), `toggle_archive` (line 423)

Imports needed:

```rust
use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::List;
use tracing::instrument;
use worker::*;

use super::LIST_SELECT;
```

---

### Task 12: Create lists/placement.rs

**Files:**
- Create: `crates/api/src/handlers/lists/placement.rs`

- [ ] **Step 1: Write placement.rs**

Move from `lists.rs`:
- Helpers: `fetch_lists_by_ids` (line 61), `apply_list_placement` (line 131)
- Handlers: `move_list` (line 581), `set_placement` (line 605)

Replace `fetch_list_ids_in_scope` with inline calls to `fetch_ordered_ids`. The old function had 3 match arms:

```rust
// Was: fetch_list_ids_in_scope(&d1, &user_id, parent_list_id, container_id)
// Becomes (inside set_placement):
let current_ids = match (body.parent_list_id.as_deref(), body.container_id.as_deref()) {
    (Some(parent_id), None) => {
        fetch_ordered_ids(
            &d1,
            "SELECT id FROM lists WHERE user_id = ?1 AND parent_list_id = ?2 ORDER BY position ASC, created_at ASC",
            &[user_id.clone().into(), parent_id.into()],
        ).await?
    }
    (None, Some(cid)) => {
        fetch_ordered_ids(
            &d1,
            "SELECT id FROM lists WHERE user_id = ?1 AND parent_list_id IS NULL AND container_id = ?2 AND archived = 0 ORDER BY position ASC, created_at ASC",
            &[user_id.clone().into(), cid.into()],
        ).await?
    }
    (None, None) => {
        fetch_ordered_ids(
            &d1,
            "SELECT id FROM lists WHERE user_id = ?1 AND parent_list_id IS NULL AND container_id IS NULL AND archived = 0 ORDER BY position ASC, created_at ASC",
            &[user_id.clone().into()],
        ).await?
    }
    (Some(_), Some(_)) => unreachable!("validated earlier"),
};
```

**Alternative (cleaner):** Keep `fetch_list_ids_in_scope` as a private function in this file but have it delegate to `fetch_ordered_ids` internally. This preserves readability:

```rust
async fn fetch_list_ids_in_scope(
    d1: &D1Database,
    user_id: &str,
    parent_list_id: Option<&str>,
    container_id: Option<&str>,
) -> Result<Vec<String>> {
    match (parent_list_id, container_id) {
        (Some(parent_id), None) => {
            fetch_ordered_ids(
                d1,
                "SELECT id FROM lists \
                 WHERE user_id = ?1 AND parent_list_id = ?2 \
                 ORDER BY position ASC, created_at ASC",
                &[user_id.into(), parent_id.into()],
            ).await
        }
        (None, Some(cid)) => {
            fetch_ordered_ids(
                d1,
                "SELECT id FROM lists \
                 WHERE user_id = ?1 AND parent_list_id IS NULL AND container_id = ?2 AND archived = 0 \
                 ORDER BY position ASC, created_at ASC",
                &[user_id.into(), cid.into()],
            ).await
        }
        (None, None) => {
            fetch_ordered_ids(
                d1,
                "SELECT id FROM lists \
                 WHERE user_id = ?1 AND parent_list_id IS NULL AND container_id IS NULL AND archived = 0 \
                 ORDER BY position ASC, created_at ASC",
                &[user_id.into()],
            ).await
        }
        (Some(_), Some(_)) => unreachable!("validated earlier"),
    }
}
```

Use this cleaner approach.

Imports needed:

```rust
use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

use super::{LIST_SELECT, ensure_parent_list_target, list_has_sublists, placement_filter};
```

---

### Task 13: Create lists/pin.rs

**Files:**
- Create: `crates/api/src/handlers/lists/pin.rs`

- [ ] **Step 1: Write pin.rs**

Move from `lists.rs`:
- Handler: `toggle_pin` (line 652)

```rust
use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::List;
use tracing::instrument;
use worker::*;

use super::LIST_SELECT;

#[instrument(skip_all, fields(action = "toggle_list_pin", list_id = tracing::field::Empty))]
pub async fn toggle_pin(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    if toggle_bool_field(&d1, "lists", "pinned", &id, &user_id)
        .await?
        .is_none()
    {
        return json_error("list_not_found", 404);
    }

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}
```

---

### Task 14: Delete lists.rs, verify build + tests

**Files:**
- Delete: `crates/api/src/handlers/lists.rs`

- [ ] **Step 1: Delete old file**

```bash
rm crates/api/src/handlers/lists.rs
```

- [ ] **Step 2: Verify build**

Run: `cargo check -p kartoteka-api 2>&1 | tail -20`
Expected: compiles successfully

- [ ] **Step 3: Run all tests**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add -A crates/api/src/handlers/lists/ && git add crates/api/src/handlers/lists.rs
git commit -m "refactor: split lists handler into submodules"
```

---

### Task 15: Migrate containers/ to use fetch_ordered_ids

**Files:**
- Modify: `crates/api/src/handlers/containers/mod.rs` — remove `fetch_container_ids_in_scope`
- Modify: `crates/api/src/handlers/containers/reorder.rs` — use `fetch_ordered_ids` instead

- [ ] **Step 1: Update containers/reorder.rs**

Replace import:
```rust
// Old:
use super::{CONTAINER_SELECT, fetch_container_ids_in_scope};
// New:
use super::CONTAINER_SELECT;
```

Replace the call site in `reorder`:
```rust
// Old:
let current_ids =
    fetch_container_ids_in_scope(&d1, &user_id, body.parent_container_id.as_deref()).await?;

// New:
let current_ids = match body.parent_container_id.as_deref() {
    Some(parent_id) => {
        fetch_ordered_ids(
            &d1,
            "SELECT id FROM containers \
             WHERE user_id = ?1 AND parent_container_id = ?2 \
             ORDER BY position ASC, created_at ASC",
            &[user_id.clone().into(), parent_id.into()],
        ).await?
    }
    None => {
        fetch_ordered_ids(
            &d1,
            "SELECT id FROM containers \
             WHERE user_id = ?1 AND parent_container_id IS NULL \
             ORDER BY position ASC, created_at ASC",
            &[user_id.clone().into()],
        ).await?
    }
};
```

- [ ] **Step 2: Remove `fetch_container_ids_in_scope` from containers/mod.rs**

Delete the entire `fetch_container_ids_in_scope` function (lines 19-57 of containers/mod.rs).

- [ ] **Step 3: Verify build + tests**

Run: `cargo check -p kartoteka-api && cargo test --workspace 2>&1 | tail -20`
Expected: compiles, all tests pass

- [ ] **Step 4: Run lint**

Run: `cargo clippy -p kartoteka-api -- -D warnings 2>&1 | tail -20`
Expected: no warnings

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/handlers/containers/
git commit -m "refactor: migrate containers to fetch_ordered_ids helper"
```

---

### Task 16: Final verification

- [ ] **Step 1: Full CI check**

Run: `just ci`
Expected: all checks pass (fmt, clippy, test, machete, deny)

- [ ] **Step 2: Verify no leftover files**

```bash
ls crates/api/src/handlers/items.rs 2>&1  # should not exist
ls crates/api/src/handlers/lists.rs 2>&1  # should not exist
ls crates/api/src/handlers/items/     # should list: mod.rs, validation.rs, crud.rs, placement.rs, calendar.rs
ls crates/api/src/handlers/lists/     # should list: mod.rs, crud.rs, features.rs, archive.rs, placement.rs, pin.rs
ls crates/api/src/handlers/containers/ # should list: mod.rs, crud.rs, home.rs, pin.rs, reorder.rs, children.rs
```
