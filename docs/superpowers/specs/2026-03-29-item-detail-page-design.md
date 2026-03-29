# Item Detail Page — Design Spec

**Issue**: #38
**Date**: 2026-03-29
**Status**: Approved

## Summary

Dedicated detail page for viewing and editing a single item. Replaces the need to edit everything inline in the crowded `ItemRow`. Auto-save on blur for all fields.

## Routing & Navigation

- New route: `path!("/lists/:list_id/items/:id")` — params read via `use_params_map()` as `"list_id"` and `"id"`
- Does not conflict with existing `path!("/lists/:id")` — the static segment `items` disambiguates
- In `ItemRow`: item title becomes an `<a>` link to the detail page
- Detail page shows breadcrumbs: Home > [List name (link to /lists/:list_id)] > Item title (no link, current page)
- Back navigation returns to `/lists/:list_id`

## API

### New endpoint: `GET /api/lists/:list_id/items/:id`

- New handler `get_one` in `handlers/items.rs`
- New route `.get_async("/api/lists/:list_id/items/:id", items::get_one)` in `router.rs`
- Returns a single `Item` (existing struct, no changes)
- Ownership check via existing `check_item_ownership` helper
- Returns 404 JSON error if not found

### New frontend function

- `fetch_item(list_id: &str, item_id: &str) -> Result<Item, String>` in `api/items.rs`

### Existing (no changes)

- `PUT /api/lists/:list_id/items/:id` — used for auto-save updates

## Detail Page Layout

Full-page form with auto-save on blur (consistent with existing `EditableDescription` / `EditableTitle` patterns). Components taking snapshot `String` values (not signals) must be wrapped in reactive closures (`move || view! { ... }`) so they re-render when the item resource updates after save.

### Header
- Breadcrumbs (Home > List name > Item title)
- Completed checkbox (toggle, same behavior as in ItemRow)

### Fields (vertical layout, full width)
1. **Title** — `EditableTitle` component (existing, from `components/common/`)
2. **Description** — `EditableDescription` component (existing, from `components/common/`)
3. **Dates section** — One `DateEditor` per enabled date type, always visible (not behind badge clicks). Border colors: start → `border-info`, deadline → `border-warning`, hard_deadline → `border-error`. Each editor fires `on_change` immediately (same as existing behavior); auto-save calls `update_item` on each change. Date types: start_date/time, deadline/time, hard_deadline.
4. **Quantity section** — New `QuantityStepper` component. Visible only when list has `quantity` feature enabled. Shows: quantity input, actual_quantity stepper, unit.

### Actions
- **Delete** — `InlineConfirmButton` component. After deletion, redirect to `/lists/:list_id`.

### Out of scope (MVP)
- Tags (will add in follow-up)
- Notes/comments
- Attachments
- Activity/change history
- Per-item reminder overrides (#32 — blocked by this issue)

## Refactoring

Extract shared components from existing `ItemRow` and `DateItemRow` to reduce duplication and reuse in the detail page.

### New shared components

| Component | Extracted from | Reused in |
|---|---|---|
| `InlineDateEditorSection` | ItemRow + DateItemRow (~30 lines each, identical) | ItemRow, DateItemRow, DetailPage |
| `DateBadgeChips` | ItemRow + DateItemRow (identical badge click/toggle pattern) | ItemRow, DateItemRow |
| `QuantityStepper` | ItemRow (~30 lines inline) | ItemRow, DetailPage |
| `InlineConfirmButton` | DateItemRow (confirm with 2500ms timeout pattern, distinct from existing `ConfirmDeleteModal`) | ItemRow, DateItemRow, DetailPage |

### Existing components to reuse more broadly

| Component | Current usage | New usage |
|---|---|---|
| `EditableDescription` | List/Container pages | ItemRow (replace inline textarea), DetailPage |
| `EditableTitle` | List/Container pages | DetailPage |
| `DateEditor` | ItemRow, DateItemRow | DetailPage (directly, not via InlineDateEditorSection) |

### ItemRow changes
- Title `<span>` becomes `<a>` link to detail page
- Inline description textarea replaced with `EditableDescription`
- Inline quantity stepper replaced with `QuantityStepper`
- Inline date editor block replaced with `InlineDateEditorSection`
- Date badge rendering replaced with `DateBadgeChips`

### DateItemRow changes
- Inline date editor block replaced with `InlineDateEditorSection`
- Date badge rendering replaced with `DateBadgeChips`
- Confirm delete button replaced with `InlineConfirmButton`

## Auto-save behavior

Each field sends `update_item` PUT on blur with only the changed field (using `UpdateItemRequest` with `Option` fields — unchanged fields sent as `None`). Dates fire on every `DateEditor` change (not on blur, since `DateEditor` has no blur concept). Success shows a toast confirmation; errors show error toast. Consistent with existing blur-save patterns in the app.
