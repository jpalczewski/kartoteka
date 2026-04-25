# List Management Port — Design

**Status:** draft
**Branch:** `rewrite`
**Related:** v1 `crates/frontend` has richer list management; v2 has most of it wired inline in `pages/list/mod.rs` but four gaps remain.

## Goal

Port four missing list management features from v1 (CSR) to v2 (SSR):

1. **Delete list** from list detail page (currently only available on home)
2. **Confirmation dialogs** for destructive actions (delete, archive, reset)
3. **Deadlines sub-config** — per-list selection of which date fields are active (`has_start_date`, `has_deadline`, `has_hard_deadline`)
4. **Inline edit of list description** (server fn exists, UI does not)

Out of scope: tag bar, inline-edit of other fields, sub-config for other features.

## Current State

`crates/frontend-v2/src/pages/list/mod.rs` already wires: rename (inline), pin, reset, archive, and feature toggles (`quantity`, `deadlines`, `location`) via a dropdown menu (⋮) in the list header. `delete_list` server fn exists and is used from `pages/home.rs`, but is not referenced from the list detail page.

`server_fns/lists.rs::update_list_features(list_id, Vec<String>)` accepts only feature names — it drops per-feature JSON config on each call. v1 stored the deadlines sub-config (3 bool flags) inside `ListFeature.config`. Porting sub-config therefore requires a server fn that preserves or updates config.

## Architecture

### New components

**`crates/frontend-v2/src/components/common/confirm_modal.rs`** — one reusable DaisyUI modal.

Props:
- `open: RwSignal<bool>`
- `title: String`
- `message: String`
- `confirm_label: String` (e.g. "Usuń", "Archiwizuj", "Resetuj")
- `variant: ConfirmVariant` (`Danger` → red button; `Warning` → yellow button)
- `on_confirm: Callback<()>`

Renders `<dialog class="modal">` with backdrop click → close, Escape → close. Button styled by variant. Not a wrapper around `<ConfirmDeleteModal>` from v1 — simpler, generic, reused for all three destructive actions.

**`crates/frontend-v2/src/components/lists/deadlines_config.rs`** — 3-checkbox sub-panel.

Props:
- `list_id: String`
- `config: serde_json::Value` (current deadlines feature config)
- `on_changed: Callback<()>` (parent triggers refresh)

Renders three small checkboxes inline:
- "Data startu" (`has_start_date`, default `false`)
- "Termin" (`has_deadline`, default `true`)
- "Twardy termin" (`has_hard_deadline`, default `false`)

On change, calls `update_feature_config(list_id, "deadlines", new_config)`, then `on_changed`. No optimistic update — refresh after success (same pattern as feature toggles in the page today).

### Modified files

**`crates/frontend-v2/src/pages/list/mod.rs`**
- Add **"🗑 Usuń listę"** item to dropdown (below archive, `text-error`).
- Introduce `pending_confirm: RwSignal<Option<ConfirmAction>>` where `ConfirmAction` enum is local to the page:
  ```rust
  enum ConfirmAction { Delete, Archive, Reset }
  ```
  Destructive dropdown buttons set this signal instead of spawning the action directly. One `<ConfirmModal>` at the bottom of the page binds to it and runs the matching server fn on confirm.
- Add click-to-edit for description using same pattern as name: `editing_description: RwSignal<bool>`, `description_input: RwSignal<String>`, Enter saves via `rename_list(id, current_name, Some(new_desc))`, Escape cancels. Rendered below the list name; when no description exists, show a dimmed "Dodaj opis..." placeholder that triggers edit on click.
- In the dropdown, right after the "Terminy" checkbox, render `<DeadlinesConfig>` when `has_deadlines` is true. Pass current `deadlines` feature config (from `data.list.features`) and an `on_changed` callback that bumps `set_refresh`.

### Backend changes — migrate features to a JSON column on `lists`

The existing `list_features` table is normalized but always used atomically: every read path aggregates rows back into a `features_json` string via a subquery, and every write path deletes all rows and re-inserts. Nothing in the codebase filters lists by feature name at the SQL level. The table's shape is a liability — `replace_features` hardcodes `config='{}'` on each insert, silently wiping any config that was ever stored.

This design collapses the normalized table into a single JSON column on `lists`.

#### Target schema

```sql
ALTER TABLE lists ADD COLUMN features TEXT NOT NULL DEFAULT '{}';
-- After data copy:
DROP TABLE list_features;
```

Shape of `lists.features`:
```json
{
  "deadlines": { "has_start_date": false, "has_deadline": true, "has_hard_deadline": false },
  "quantity":  {}
}
```
Top-level keys are feature names; values are per-feature config objects. Missing key = feature not enabled. Empty object `{}` = feature enabled with defaults.

#### Migration file

`crates/db/migrations/004_lists_features_json.sql` — forward-only, SQLite-compatible:

```sql
ALTER TABLE lists ADD COLUMN features TEXT NOT NULL DEFAULT '{}';

UPDATE lists SET features = (
    SELECT COALESCE(
        json_group_object(lf.feature_name, json(lf.config)),
        '{}'
    )
    FROM list_features lf
    WHERE lf.list_id = lists.id
);

DROP TABLE list_features;
```

Notes:
- `json_group_object` + `json(...)` assembles a keyed object, unwrapping each config string as a JSON value (not nested-string).
- `COALESCE(..., '{}')` handles lists with zero features (subquery returns `NULL`).
- Forward-only, following project convention (`001_init.sql`, `002_*`, `003_oauth.sql` are all non-reversible).
- Runs automatically at boot via `sqlx::migrate!("./migrations")` in `crates/db/src/lib.rs:63`.

Risk assessment: all current configs in the DB are `'{}'` (no code ever wrote anything else), so the migration cannot lose data. Safe to apply on dev and prod without a backup-then-revert plan, though a pre-migration DB snapshot is prudent.

#### Code changes

**`crates/db/src/lists.rs`**
- Remove the `features_json` subquery from SELECT statements (lines 49–51, 208–210). Replace with plain `l.features` column.
- `ListRow` / `ListProjectionRow` types: rename field from `features_json: String` to `features: String` (still a JSON string from SQLite, parsed by domain).
- Rewrite `replace_features`:
  - New signature: `pub async fn set_features(tx: &mut SqliteConnection, list_id: &str, features: &serde_json::Value) -> Result<(), DbError>`
  - Implementation: `UPDATE lists SET features = ? WHERE id = ?` — one atomic write.
  - Callers pass the full desired JSON object. No DELETE + INSERT, no config wiping.
- Remove all references to `list_features` table (confirmed: only the SELECT subqueries, `replace_features`, and the migration file itself).

**`crates/domain/src/lists.rs`** (4 call sites)
- `list_row → List` conversion (~line 103): parse `row.features` instead of `row.features_json`. Change shape from `Vec<ListFeature>` deserialize to object deserialize, then project to the existing `Vec<ListFeature>` keeping the API stable for callers.
- `CreateListRequest` handling (~line 197): build a `serde_json::Value` object from `req.features: Vec<String>` (each key → `{}` config), pass to new `db::lists::set_features`.
- Validation path (~line 223): same deserialize change.
- `set_features` domain wrapper (~line 341): same — build object, call DB.
- Add `update_feature_config(pool, user_id, list_id, feature_name, config) -> Result<(), DomainError>`:
  - Load list via `db::lists::get_one`.
  - Verify user ownership.
  - Parse current `features` JSON, mutate the named key's value, serialize back, `UPDATE lists SET features = ?`.
  - Returns `DomainError::NotFound("feature")` if the feature isn't currently enabled on the list.

**`crates/domain/src/home.rs:24`** — same deserialization fix (parse object, project to `Vec<ListFeature>` for the existing API).

**`crates/domain/src/items.rs:335`** (test helper only) — change test setup from `INSERT INTO list_features` to `UPDATE lists SET features = ?`.

**`crates/frontend-v2/src/server_fns/lists.rs`**
- Add `update_feature_config(list_id: String, feature_name: String, config: serde_json::Value) -> Result<(), ServerFnError>` — thin wrapper around the domain function. Same auth pattern as the other list server fns (AuthSession extractor).

**Shared types** — `kartoteka_shared::ListFeature { feature_name, config }` remains unchanged; only the storage representation changes. DTO/API surface is unaffected.

#### Tests to update

- `crates/db/src/lists.rs` tests at lines 447, 481, 487, 519 — change expectations from `features_json` column/string shape to `features` object shape.
- `crates/domain/src/items.rs:335` — test helper as above.

No new migration tests needed — the migration is a one-shot, forward-only statement run at startup. Existing feature-related domain tests provide regression coverage by continuing to pass against the new schema.

### Convention note

Future migrations follow project convention: `crates/db/migrations/NNN_<snake_name>.sql`, forward-only, run via `sqlx::migrate!("./migrations")`, idempotent where practical (`CREATE TABLE IF NOT EXISTS`, `ALTER TABLE ... ADD COLUMN ... DEFAULT ...`).

### Data flow

Destructive action:
```
Click "Usuń listę"
  → pending_confirm.set(Some(Delete))
  → <ConfirmModal open=..> renders "Usunąć listę X?"
  → Confirm → delete_list(id) → navigate("/")
  → Cancel/Esc → pending_confirm.set(None)
```

Deadlines sub-config:
```
Toggle "Data startu" checkbox
  → build config JSON with new flag
  → update_feature_config(list_id, "deadlines", config)
  → on_changed → set_refresh.update(+1) → data_res refetches
  → item detail now shows/hides the corresponding date field
```

Description edit:
```
Click description
  → editing_description=true, description_input = current
  → type → Enter
  → rename_list(id, name, Some(input)) — reuses existing server fn
  → refresh → display updated
```

### Error handling

All server fn errors routed through existing `ToastContext` (`toast.push(err.to_string(), ToastKind::Error)`). Modal stays open on error so user can retry or cancel.

### Testing

- Manual smoke via `just dev`:
  - Delete a list → confirmation → list gone, redirect to `/`
  - Archive → confirmation → list disappears from home, still accessible via `/archived` if that view exists
  - Reset → confirmation → all items `completed=false`
  - Toggle each deadline sub-flag → open an item → verify the corresponding date field appears/disappears
  - Click description → edit → Enter → description updates; click empty placeholder → can add new description
- No new unit tests (no frontend-v2 test suite for components today).
- Playwright coverage deferred — separate spec if desired.

## Non-goals

- Port `ListTagBar` from v1 (v2 already has `TagList` per list).
- Inline-edit other fields (icon, list_type).
- Sub-config for `quantity` or `location` features (no v1 precedent).
- Optimistic updates for feature config (full refresh is fine given rarity of the action).
