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

**`crates/frontend-v2/src/server_fns/lists.rs`**
- Add `update_feature_config(list_id: String, feature_name: String, config: serde_json::Value) -> Result<(), ServerFnError>`.
- Implementation: load the list, clone its features, find the named feature, replace its `config`, call `domain::lists::set_features` with the updated `Vec<ListFeature>`.
- If the domain layer's `set_features` takes only names, add `domain::lists::set_features_with_config(pool, user_id, list_id, Vec<ListFeature>)` that replaces features including config. Decide at implementation time based on what's already there.

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
