# Item Detail Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a dedicated item detail page with auto-save editing, and refactor shared components out of ItemRow/DateItemRow to reduce duplication.

**Architecture:** New route `/lists/:list_id/items/:id` with a detail page component. API gets a new `GET` endpoint for single item fetch. Four shared components extracted from ItemRow/DateItemRow before building the detail page.

**Tech Stack:** Rust, Leptos 0.7 CSR, worker crate (CF Workers), D1/SQLite, DaisyUI 5, gloo-net 0.6

**Spec:** `docs/superpowers/specs/2026-03-29-item-detail-page-design.md`

---

## File Map

### New files
| File | Purpose |
|---|---|
| `crates/frontend/src/components/items/inline_date_editor_section.rs` | Shared: date type → DateEditor + close button |
| `crates/frontend/src/components/items/date_badge_chips.rs` | Shared: clickable date badge rendering |
| `crates/frontend/src/components/items/quantity_stepper.rs` | Shared: quantity actual/target stepper with progress |
| `crates/frontend/src/components/common/inline_confirm_button.rs` | Shared: button with confirm-on-click + timeout |
| `crates/frontend/src/pages/item_detail.rs` | Item detail page component |

### Modified files
| File | Changes |
|---|---|
| `crates/api/src/handlers/items.rs` | Add `get_one` handler |
| `crates/api/src/router.rs` | Add GET route for single item |
| `crates/frontend/src/api/items.rs` | Add `fetch_item` function |
| `crates/frontend/src/components/items/item_row.rs` | Use shared components, title → link |
| `crates/frontend/src/components/items/date_item_row.rs` | Use shared components |
| `crates/frontend/src/components/items/mod.rs` | Add new module exports |
| `crates/frontend/src/components/common/mod.rs` | Add `inline_confirm_button` module |
| `crates/frontend/src/pages/mod.rs` | Add `item_detail` module |
| `crates/frontend/src/app.rs` | Add route for item detail page |

---

## Task 1: API — GET single item endpoint

**Files:**
- Modify: `crates/api/src/handlers/items.rs`
- Modify: `crates/api/src/router.rs`

- [ ] **Step 1: Add `get_one` handler in `handlers/items.rs`**

Add after the `list_all` function (after line 30):

```rust
pub async fn get_one(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_item_ownership(&d1, &id, &user_id).await? {
        return json_error("item_not_found", 404);
    }

    let query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    let item = d1
        .prepare(&query)
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;
    Response::from_json(&item)
}
```

- [ ] **Step 2: Register route in `router.rs`**

In `router.rs`, add after the existing `.post_async("/api/lists/:list_id/items", items::create)` line:

```rust
.get_async("/api/lists/:list_id/items/:id", items::get_one)
```

Place it before `.put_async("/api/lists/:list_id/items/:id", items::update)` so the GET comes first.

- [ ] **Step 3: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/handlers/items.rs crates/api/src/router.rs
git commit -m "feat(api): add GET single item endpoint"
```

---

## Task 2: Frontend API — fetch_item function

**Files:**
- Modify: `crates/frontend/src/api/items.rs`

- [ ] **Step 1: Add `fetch_item` to `api/items.rs`**

Add after the existing `fetch_items` function:

```rust
pub async fn fetch_item(list_id: &str, item_id: &str) -> Result<Item, String> {
    super::get(&format!(
        "{}/lists/{list_id}/items/{item_id}",
        super::API_BASE
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/api/items.rs
git commit -m "feat(frontend): add fetch_item API function"
```

---

## Task 3: Extract InlineConfirmButton component

**Files:**
- Create: `crates/frontend/src/components/common/inline_confirm_button.rs`
- Modify: `crates/frontend/src/components/common/mod.rs`
- Modify: `crates/frontend/src/components/items/date_item_row.rs`

This extracts the confirm-with-timeout delete button from `DateItemRow` (lines 131-148 in `date_item_row.rs`). It is distinct from the existing `ConfirmDeleteModal`.

- [ ] **Step 1: Create `inline_confirm_button.rs`**

Create `crates/frontend/src/components/common/inline_confirm_button.rs`:

```rust
use leptos::prelude::*;

/// Inline button that requires a second click to confirm.
/// First click shows confirm_label for `timeout_ms`, then reverts.
/// Distinct from `ConfirmDeleteModal` which is a modal dialog.
#[component]
pub fn InlineConfirmButton(
    /// Callback when confirmed (second click)
    on_confirm: Callback<()>,
    /// Label shown normally
    #[prop(default = "\u{2715}".to_string())]
    label: String,
    /// Label shown during confirm state
    #[prop(default = "Na pewno?".to_string())]
    confirm_label: String,
    /// CSS class for normal state
    #[prop(default = "btn btn-ghost btn-sm btn-square opacity-60 hover:opacity-100".to_string())]
    class: String,
    /// CSS class for confirm state
    #[prop(default = "btn btn-error btn-sm".to_string())]
    confirm_class: String,
    /// Timeout in ms before reverting to normal state
    #[prop(default = 2500)]
    timeout_ms: u32,
) -> impl IntoView {
    let confirming = RwSignal::new(false);

    view! {
        <button
            type="button"
            class=move || if confirming.get() { confirm_class.clone() } else { class.clone() }
            on:click=move |_| {
                if confirming.get() {
                    on_confirm.run(());
                    confirming.set(false);
                } else {
                    confirming.set(true);
                    set_timeout(
                        move || confirming.set(false),
                        std::time::Duration::from_millis(timeout_ms.into()),
                    );
                }
            }
        >
            {move || if confirming.get() { confirm_label.clone() } else { label.clone() }}
        </button>
    }
}
```

- [ ] **Step 2: Register module in `common/mod.rs`**

Add to `crates/frontend/src/components/common/mod.rs`:

```rust
pub mod inline_confirm_button;
```

- [ ] **Step 3: Refactor `DateItemRow` to use `InlineConfirmButton`**

In `crates/frontend/src/components/items/date_item_row.rs`:

Replace the inline confirm block (the `let confirming = RwSignal::new(false); view! { <button ...` block around lines 131-148) with:

```rust
<InlineConfirmButton on_confirm=Callback::new(move |()| on_delete.run(id_delete.clone())) />
```

Add the import at the top:
```rust
use crate::components::common::inline_confirm_button::InlineConfirmButton;
```

Remove the now-unused `id_delete` clone that was used only in the old block — keep the one used by InlineConfirmButton's callback. Also remove `use gloo_timers::callback::Timeout;` or `set_timeout` import if it becomes unused.

- [ ] **Step 4: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add crates/frontend/src/components/common/inline_confirm_button.rs \
       crates/frontend/src/components/common/mod.rs \
       crates/frontend/src/components/items/date_item_row.rs
git commit -m "refactor: extract InlineConfirmButton from DateItemRow"
```

---

## Task 4: Extract InlineDateEditorSection component

**Files:**
- Create: `crates/frontend/src/components/items/inline_date_editor_section.rs`
- Modify: `crates/frontend/src/components/items/mod.rs`
- Modify: `crates/frontend/src/components/items/item_row.rs`
- Modify: `crates/frontend/src/components/items/date_item_row.rs`

This extracts the identical "match date_type → border color → DateEditor + close button" block that appears in both ItemRow (lines ~240-270) and DateItemRow (lines ~195-225).

- [ ] **Step 1: Create `inline_date_editor_section.rs`**

Create `crates/frontend/src/components/items/inline_date_editor_section.rs`:

```rust
use kartoteka_shared::Item;
use leptos::prelude::*;

use super::date_editor::DateEditor;

/// Inline date editor section: resolves date_type to border color + initial values,
/// renders DateEditor + close button. Used by ItemRow, DateItemRow.
#[component]
pub fn InlineDateEditorSection(
    /// Which date type is being edited: "start", "deadline", or "hard_deadline"
    date_type: String,
    /// The item being edited (for initial date/time values)
    item: Item,
    /// Item ID for the save callback
    item_id: String,
    /// Called with (item_id, date_type, date_value, time_value)
    on_save: Callback<(String, String, String, Option<String>)>,
    /// Called when the close button is clicked
    on_close: Callback<()>,
    /// CSS class for the wrapper div
    #[prop(default = "pl-14 pb-2".to_string())]
    wrapper_class: String,
) -> impl IntoView {
    let (border, init_date, init_time, has_time) = match date_type.as_str() {
        "start" => (
            "border-info",
            item.start_date.clone(),
            item.start_time.clone(),
            true,
        ),
        "hard_deadline" => ("border-error", item.hard_deadline.clone(), None, false),
        _ => (
            "border-warning",
            item.deadline.clone(),
            item.deadline_time.clone(),
            true,
        ),
    };

    let id_for_save = item_id;
    let dt_for_save = date_type;

    view! {
        <div class=wrapper_class>
            <DateEditor
                border_color=border
                initial_date=init_date
                initial_time=init_time
                has_time=has_time
                on_change=Callback::new(move |(date, time): (String, Option<String>)| {
                    on_save.run((id_for_save.clone(), dt_for_save.clone(), date, time));
                })
            />
            <button type="button" class="btn btn-xs btn-ghost mt-1 opacity-50"
                on:click=move |_| on_close.run(())
            >"Zamknij"</button>
        </div>
    }
}
```

- [ ] **Step 2: Register module in `items/mod.rs`**

Add to `crates/frontend/src/components/items/mod.rs`:

```rust
pub mod inline_date_editor_section;
```

- [ ] **Step 3: Refactor ItemRow to use InlineDateEditorSection**

In `crates/frontend/src/components/items/item_row.rs`, replace the inline date editor block (the `move || { let dt = editing_date.get(); if let (Some(dt), Some(on_save)) = ...` block around lines 240-270) with:

```rust
{move || {
    let dt = editing_date.get();
    if let (Some(dt), Some(on_save)) = (dt, on_date_save) {
        view! {
            <InlineDateEditorSection
                date_type=dt
                item=item_for_editor.clone()
                item_id=id_for_editor.clone()
                on_save=on_save
                on_close=Callback::new(move |()| editing_date.set(None))
            />
        }.into_any()
    } else {
        view! {}.into_any()
    }
}}
```

This requires storing a clone of the item for the editor: add `let item_for_editor = item.clone();` near the top of the component (before other clones consume fields).

Add import:
```rust
use super::inline_date_editor_section::InlineDateEditorSection;
```

Remove the now-unused individual date field clones (`item_start_date`, `item_start_time`, etc.) that were only used in the old inline block.

- [ ] **Step 4: Refactor DateItemRow to use InlineDateEditorSection**

In `crates/frontend/src/components/items/date_item_row.rs`, replace the inline date editor block (similar match + DateEditor + close button around lines 195-225) with:

```rust
{move || {
    let dt = editing_date.get();
    if let (Some(dt), Some(on_save)) = (dt, on_date_save) {
        view! {
            <InlineDateEditorSection
                date_type=dt
                item=item_for_editor.clone()
                item_id=id_for_editor.clone()
                on_save=on_save
                on_close=Callback::new(move |()| editing_date.set(None))
                wrapper_class="pl-10 pb-2".to_string()
            />
        }.into_any()
    } else {
        view! {}.into_any()
    }
}}
```

Note the different `wrapper_class` — DateItemRow uses `"pl-10 pb-2"` while ItemRow uses `"pl-14 pb-2"`.

Add `let item_for_editor = item.clone();` near the top. Add import. Remove unused date field clones.

- [ ] **Step 5: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add crates/frontend/src/components/items/inline_date_editor_section.rs \
       crates/frontend/src/components/items/mod.rs \
       crates/frontend/src/components/items/item_row.rs \
       crates/frontend/src/components/items/date_item_row.rs
git commit -m "refactor: extract InlineDateEditorSection from ItemRow and DateItemRow"
```

---

## Task 5: Extract DateBadgeChips component

**Files:**
- Create: `crates/frontend/src/components/items/date_badge_chips.rs`
- Modify: `crates/frontend/src/components/items/mod.rs`
- Modify: `crates/frontend/src/components/items/item_row.rs`
- Modify: `crates/frontend/src/components/items/date_item_row.rs`

Both ItemRow and DateItemRow have the same pattern: iterate over `item_date_badges()` result, render clickable buttons that toggle `editing_date` signal. ItemRow also has ghost chips.

- [ ] **Step 1: Create `date_badge_chips.rs`**

Create `crates/frontend/src/components/items/date_badge_chips.rs`:

```rust
use crate::components::common::date_utils::DateBadge;
use leptos::prelude::*;

/// Renders clickable date badge chips that toggle an editing_date signal.
/// Used by ItemRow and DateItemRow.
#[component]
pub fn DateBadgeChips(
    badges: Vec<DateBadge>,
    editing_date: RwSignal<Option<String>>,
    /// Show ghost chips for unset date types
    #[prop(default = false)]
    ghost_start: bool,
    #[prop(default = false)]
    ghost_deadline: bool,
    #[prop(default = false)]
    ghost_hard: bool,
) -> impl IntoView {
    let has_ghosts = ghost_start || ghost_deadline || ghost_hard;
    if badges.is_empty() && !has_ghosts {
        return view! {}.into_any();
    }

    view! {
        <div class="flex gap-1 flex-wrap shrink-0">
            {badges.into_iter().map(|b| {
                let dt = b.date_type.to_string();
                view! {
                    <button type="button" class=format!("{} cursor-pointer", b.css)
                        on:click=move |_| {
                            let current = editing_date.get();
                            if current.as_deref() == Some(dt.as_str()) {
                                editing_date.set(None);
                            } else {
                                editing_date.set(Some(dt.clone()));
                            }
                        }
                    >{b.label}</button>
                }
            }).collect::<Vec<_>>()}
            {if ghost_start {
                view! {
                    <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                        on:click=move |_| editing_date.set(Some("start".into()))
                    >"+\u{1F4C5}"</button>
                }.into_any()
            } else { view! {}.into_any() }}
            {if ghost_deadline {
                view! {
                    <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                        on:click=move |_| editing_date.set(Some("deadline".into()))
                    >"+\u{23F0}"</button>
                }.into_any()
            } else { view! {}.into_any() }}
            {if ghost_hard {
                view! {
                    <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                        on:click=move |_| editing_date.set(Some("hard_deadline".into()))
                    >"+\u{1F6A8}"</button>
                }.into_any()
            } else { view! {}.into_any() }}
        </div>
    }.into_any()
}
```

- [ ] **Step 2: Register in `items/mod.rs`**

Add: `pub mod date_badge_chips;`

- [ ] **Step 3: Refactor ItemRow to use DateBadgeChips**

In `item_row.rs`, replace the date badges + ghost chips rendering block (the large `if date_badges.is_empty() && !has_ghosts` block around lines 66-145) with:

```rust
<DateBadgeChips
    badges=date_badges
    editing_date=editing_date
    ghost_start=ghost_start
    ghost_deadline=ghost_deadline
    ghost_hard=ghost_hard
/>
```

Add import: `use super::date_badge_chips::DateBadgeChips;`

Keep the ghost chip computation logic (cfg_start, cfg_deadline, cfg_hard, ghost_start, etc.) — only the rendering moves to the component.

- [ ] **Step 4: Refactor DateItemRow to use DateBadgeChips**

In `date_item_row.rs`, replace the secondary badges rendering (inside the `has_secondary` block) with `DateBadgeChips`. DateItemRow doesn't have ghost chips, so all ghost props stay `false` (default).

The DateItemRow secondary badges block also contains TagList in the same div — keep TagList, just replace the badge iteration with:

```rust
<DateBadgeChips badges=secondary_badges editing_date=editing_date />
```

- [ ] **Step 5: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add crates/frontend/src/components/items/date_badge_chips.rs \
       crates/frontend/src/components/items/mod.rs \
       crates/frontend/src/components/items/item_row.rs \
       crates/frontend/src/components/items/date_item_row.rs
git commit -m "refactor: extract DateBadgeChips from ItemRow and DateItemRow"
```

---

## Task 6: Extract QuantityStepper component

**Files:**
- Create: `crates/frontend/src/components/items/quantity_stepper.rs`
- Modify: `crates/frontend/src/components/items/mod.rs`
- Modify: `crates/frontend/src/components/items/item_row.rs`

- [ ] **Step 1: Create `quantity_stepper.rs`**

Create `crates/frontend/src/components/items/quantity_stepper.rs`:

```rust
use leptos::prelude::*;

/// Quantity stepper: -/+ buttons, actual/target display, progress bar.
/// Used by ItemRow and ItemDetailPage.
#[component]
pub fn QuantityStepper(
    target: i32,
    initial_actual: i32,
    unit: String,
    on_change: Callback<i32>,
) -> impl IntoView {
    let actual = RwSignal::new(initial_actual);

    view! {
        <div class="flex flex-col items-center gap-0.5">
            <div class="flex items-center gap-1">
                <button type="button" class="btn btn-xs btn-circle btn-ghost"
                    on:click=move |_| {
                        let new_val = (actual.get() - 1).max(0);
                        actual.set(new_val);
                        on_change.run(new_val);
                    }
                >"\u{2212}"</button>
                <span class="text-sm font-mono">
                    {move || actual.get()} " / " {target} " " {unit.clone()}
                </span>
                <button type="button" class="btn btn-xs btn-circle btn-ghost"
                    on:click=move |_| {
                        let new_val = actual.get() + 1;
                        actual.set(new_val);
                        on_change.run(new_val);
                    }
                >"+"</button>
            </div>
            <progress class="progress progress-primary w-20 h-1"
                value=move || actual.get().to_string()
                max=target.to_string()
            />
        </div>
    }
}
```

- [ ] **Step 2: Register in `items/mod.rs`**

Add: `pub mod quantity_stepper;`

- [ ] **Step 3: Refactor ItemRow to use QuantityStepper**

In `item_row.rs`, replace the inline quantity stepper block (the `if show_stepper { ... }` block around lines 155-195) with:

```rust
{if show_stepper {
    let id_for_stepper = id.clone();
    view! {
        <QuantityStepper
            target=target_qty
            initial_actual=item.actual_quantity.unwrap_or(0)
            unit=unit_label.clone()
            on_change=Callback::new(move |new_val: i32| {
                if let Some(cb) = on_quantity_change {
                    cb.run((id_for_stepper.clone(), new_val));
                }
            })
        />
    }.into_any()
} else {
    view! {}.into_any()
}}
```

Add import: `use super::quantity_stepper::QuantityStepper;`

Remove the now-unused `id_dec`, `id_inc`, `cb_dec`, `cb_inc`, `actual` signal, and inline stepper code.

- [ ] **Step 4: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add crates/frontend/src/components/items/quantity_stepper.rs \
       crates/frontend/src/components/items/mod.rs \
       crates/frontend/src/components/items/item_row.rs
git commit -m "refactor: extract QuantityStepper from ItemRow"
```

---

## Task 7: Replace ItemRow inline description with EditableDescription

**Files:**
- Modify: `crates/frontend/src/components/items/item_row.rs`

- [ ] **Step 1: Refactor ItemRow description**

In `item_row.rs`, replace the expandable description block (the `if expanded.get() { ... } else { ... }` block around lines 270-310) with `EditableDescription`.

The current pattern: expand button toggles `expanded` signal, shows textarea on blur save. Replace with:

Keep the expand/collapse toggle button. When expanded, show `EditableDescription`:

```rust
{move || {
    if expanded.get() {
        let id_desc = id_for_desc.clone();
        view! {
            <div class="pl-14 pb-2">
                <EditableDescription
                    value=Some(description_text.get())
                    on_save=Callback::new(move |new_val: Option<String>| {
                        let text = new_val.clone().unwrap_or_default();
                        description_text.set(text);
                        if let Some(cb) = on_description_save {
                            cb.run((id_desc.clone(), new_val.unwrap_or_default()));
                        }
                    })
                />
            </div>
        }.into_any()
    } else {
        let desc = description_text.get();
        if desc.is_empty() {
            view! {}.into_any()
        } else {
            view! {
                <p class="pl-14 pb-1 text-sm text-base-content/60">{desc}</p>
            }.into_any()
        }
    }
}}
```

Add import: `use crate::components::common::editable_description::EditableDescription;`

Remove the raw textarea code that was there before.

- [ ] **Step 2: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/components/items/item_row.rs
git commit -m "refactor: use EditableDescription in ItemRow"
```

---

## Task 8: Make ItemRow title a link to detail page

**Files:**
- Modify: `crates/frontend/src/components/items/item_row.rs`

- [ ] **Step 1: Change title span to anchor link**

In `item_row.rs`, the title is currently:

```rust
<span class=title_class>{item.title}</span>
```

Replace with:

```rust
<a href=format!("/lists/{}/items/{}", item.list_id, id) class=format!("{title_class} hover:text-primary transition-colors no-underline")>
    {item.title.clone()}
</a>
```

The `item.list_id` is available on the `Item` struct. Make sure `item.title` is cloned before `item` is consumed by other uses — may need to reorder clones at the top of the function. Store `let item_title = item.title.clone();` and `let item_list_id = item.list_id.clone();` early.

- [ ] **Step 2: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/components/items/item_row.rs
git commit -m "feat: make ItemRow title a link to detail page"
```

---

## Task 9: Item detail page component

**Files:**
- Create: `crates/frontend/src/pages/item_detail.rs`
- Modify: `crates/frontend/src/pages/mod.rs`
- Modify: `crates/frontend/src/app.rs`

This is the main new page. It fetches the item + list (for name and features), then renders a form with auto-save.

- [ ] **Step 1: Create `item_detail.rs`**

Create `crates/frontend/src/pages/item_detail.rs`:

```rust
use crate::api::{delete_item, fetch_item, fetch_list, update_item};
use crate::app::{ToastContext, ToastKind};
use crate::components::common::breadcrumbs::Breadcrumbs;
use crate::components::common::editable_description::EditableDescription;
use crate::components::common::editable_title::EditableTitle;
use crate::components::common::inline_confirm_button::InlineConfirmButton;
use crate::components::common::loading::Loading;
use crate::components::items::date_editor::DateEditor;
use crate::components::items::quantity_stepper::QuantityStepper;
use kartoteka_shared::*;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// Helper: spawn an update_item call with toast feedback.
fn spawn_save(list_id: String, item_id: String, toast: ToastContext, req: UpdateItemRequest) {
    leptos::task::spawn_local(async move {
        match update_item(&list_id, &item_id, &req).await {
            Ok(_) => toast.push("Zapisano".into(), ToastKind::Success),
            Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
        }
    });
}

#[component]
pub fn ItemDetailPage() -> impl IntoView {
    let params = use_params_map();
    let toast = expect_context::<ToastContext>();
    let navigate = leptos_router::hooks::use_navigate();

    let list_id =
        move || params.with(|p| p.get("list_id").unwrap_or_default().to_string());
    let item_id =
        move || params.with(|p| p.get("id").unwrap_or_default().to_string());

    // Fetch item
    let item_resource = LocalResource::new(move || {
        let lid = list_id();
        let iid = item_id();
        async move { fetch_item(&lid, &iid).await }
    });

    // Fetch list (for name in breadcrumbs + features)
    let list_resource = LocalResource::new(move || {
        let lid = list_id();
        async move { fetch_list(&lid).await }
    });

    view! {
        <Suspense fallback=move || view! { <Loading /> }>
            {move || {
                let item_result = item_resource.get().map(|r| (*r).clone());
                let list_result = list_resource.get().map(|r| (*r).clone());

                match (item_result, list_result) {
                    (Some(Ok(item)), Some(Ok(list))) => {
                        let lid = list.id.clone();
                        let list_name = list.name.clone();
                        let features = list.features.clone();

                        // Deadlines config
                        let deadlines_config = features
                            .iter()
                            .find(|f| f.name == FEATURE_DEADLINES)
                            .map(|f| f.config.clone())
                            .unwrap_or(serde_json::Value::Null);
                        let has_quantity = features.iter().any(|f| f.name == FEATURE_QUANTITY);

                        let cfg_start = deadlines_config.get("has_start_date")
                            .and_then(|v| v.as_bool()).unwrap_or(false);
                        let cfg_deadline = deadlines_config.get("has_deadline")
                            .and_then(|v| v.as_bool()).unwrap_or(false);
                        let cfg_hard = deadlines_config.get("has_hard_deadline")
                            .and_then(|v| v.as_bool()).unwrap_or(false);

                        // Breadcrumbs (list name as link, item title as plain text below)
                        let crumbs = vec![
                            (list_name, format!("/lists/{lid}")),
                        ];
                        let item_title_for_crumb = item.title.clone();

                        // Completed toggle
                        let completed = RwSignal::new(item.completed);

                        // Delete
                        let lid_for_delete = list_id();
                        let iid_for_delete = item_id();
                        let toast_del = toast.clone();
                        let nav = navigate.clone();

                        view! {
                            // Breadcrumbs with item title as non-linked final crumb
                            <div class="breadcrumbs text-sm mb-4">
                                <ul>
                                    <li><a href="/">"Home"</a></li>
                                    {crumbs.into_iter().map(|(label, href)| {
                                        view! { <li><a href=href>{label}</a></li> }
                                    }).collect::<Vec<_>>()}
                                    <li>{item_title_for_crumb}</li>
                                </ul>
                            </div>

                            // Completed checkbox + title
                            <div class="flex items-center gap-3 mb-4">
                                <input
                                    type="checkbox"
                                    class="checkbox checkbox-secondary checkbox-lg"
                                    checked=item.completed
                                    on:change=move |_| {
                                        let new_val = !completed.get();
                                        completed.set(new_val);
                                        spawn_save(list_id(), item_id(), toast.clone(), UpdateItemRequest {
                                            completed: Some(new_val),
                                            ..Default::default()
                                        });
                                    }
                                />
                                <EditableTitle
                                    value=item.title.clone()
                                    on_save=Callback::new(move |new_title: String| {
                                        spawn_save(list_id(), item_id(), toast.clone(), UpdateItemRequest {
                                            title: Some(new_title),
                                            ..Default::default()
                                        });
                                    })
                                />
                            </div>

                            // Description
                            <EditableDescription
                                value=item.description.clone()
                                on_save=Callback::new(move |new_desc: Option<String>| {
                                    spawn_save(list_id(), item_id(), toast.clone(), UpdateItemRequest {
                                        description: new_desc,
                                        ..Default::default()
                                    });
                                })
                            />

                            // Dates section
                            {if cfg_start {
                                view! {
                                    <div class="mb-4">
                                        <label class="label text-sm font-medium">"Data rozpoczęcia"</label>
                                        <DateEditor
                                            border_color="border-info"
                                            initial_date=item.start_date.clone()
                                            initial_time=item.start_time.clone()
                                            has_time=true
                                            on_change=Callback::new(move |(date, time): (String, Option<String>)| {
                                                let (d, t) = if date.is_empty() {
                                                    (Some(None), Some(None))
                                                } else {
                                                    (Some(Some(date)), time.map(Some))
                                                };
                                                spawn_save(list_id(), item_id(), toast.clone(), UpdateItemRequest {
                                                    start_date: d,
                                                    start_time: t,
                                                    ..Default::default()
                                                });
                                            })
                                        />
                                    </div>
                                }.into_any()
                            } else { view! {}.into_any() }}

                            {if cfg_deadline {
                                view! {
                                    <div class="mb-4">
                                        <label class="label text-sm font-medium">"Termin"</label>
                                        <DateEditor
                                            border_color="border-warning"
                                            initial_date=item.deadline.clone()
                                            initial_time=item.deadline_time.clone()
                                            has_time=true
                                            on_change=Callback::new(move |(date, time): (String, Option<String>)| {
                                                let (d, t) = if date.is_empty() {
                                                    (Some(None), Some(None))
                                                } else {
                                                    (Some(Some(date)), time.map(Some))
                                                };
                                                spawn_save(list_id(), item_id(), toast.clone(), UpdateItemRequest {
                                                    deadline: d,
                                                    deadline_time: t,
                                                    ..Default::default()
                                                });
                                            })
                                        />
                                    </div>
                                }.into_any()
                            } else { view! {}.into_any() }}

                            {if cfg_hard {
                                view! {
                                    <div class="mb-4">
                                        <label class="label text-sm font-medium">"Twardy termin"</label>
                                        <DateEditor
                                            border_color="border-error"
                                            initial_date=item.hard_deadline.clone()
                                            initial_time=None::<String>
                                            has_time=false
                                            on_change=Callback::new(move |(date, _time): (String, Option<String>)| {
                                                let d = if date.is_empty() {
                                                    Some(None)
                                                } else {
                                                    Some(Some(date))
                                                };
                                                spawn_save(list_id(), item_id(), toast.clone(), UpdateItemRequest {
                                                    hard_deadline: d,
                                                    ..Default::default()
                                                });
                                            })
                                        />
                                    </div>
                                }.into_any()
                            } else { view! {}.into_any() }}

                            // Quantity section
                            {if has_quantity && item.quantity.is_some() {
                                let target = item.quantity.unwrap_or(0);
                                let unit = item.unit.clone().unwrap_or_default();
                                view! {
                                    <div class="mb-4">
                                        <label class="label text-sm font-medium">"Ilość"</label>
                                        <QuantityStepper
                                            target=target
                                            initial_actual=item.actual_quantity.unwrap_or(0)
                                            unit=unit
                                            on_change=Callback::new(move |new_val: i32| {
                                                spawn_save(list_id(), item_id(), toast.clone(), UpdateItemRequest {
                                                    actual_quantity: Some(new_val),
                                                    ..Default::default()
                                                });
                                            })
                                        />
                                    </div>
                                }.into_any()
                            } else { view! {}.into_any() }}

                            // Delete button
                            <div class="mt-8 pt-4 border-t border-base-300">
                                <InlineConfirmButton
                                    on_confirm=Callback::new(move |()| {
                                        let lid = lid_for_delete.clone();
                                        let iid = iid_for_delete.clone();
                                        let toast = toast_del.clone();
                                        let nav = nav.clone();
                                        leptos::task::spawn_local(async move {
                                            match delete_item(&lid, &iid).await {
                                                Ok(_) => {
                                                    toast.push("Usunięto".into(), ToastKind::Success);
                                                    nav(&format!("/lists/{lid}"), Default::default());
                                                }
                                                Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
                                            }
                                        });
                                    })
                                    label="Usuń element".to_string()
                                    confirm_label="Na pewno usunąć?".to_string()
                                    class="btn btn-error btn-outline btn-sm".to_string()
                                    confirm_class="btn btn-error btn-sm".to_string()
                                />
                            </div>
                        }.into_any()
                    }
                    (Some(Err(e)), _) | (_, Some(Err(e))) => {
                        view! { <p class="text-error">{format!("Błąd: {e}")}</p> }.into_any()
                    }
                    _ => view! { <Loading /> }.into_any(),
                }
            }}
        </Suspense>
    }
}
```

**Important notes:**
- `UpdateItemRequest` needs `Default` derive — add it in step 2.
- The `spawn_save` helper function avoids the need to clone closures (closures are not `Clone`). Each callback calls `spawn_save` directly with fresh `list_id()`, `item_id()`, `toast.clone()`.
- Breadcrumbs: we render inline HTML instead of using the `Breadcrumbs` component, because the component renders all crumbs as `<a>` links, but the spec requires the last crumb (item title) to be plain text (current page). The existing `Breadcrumbs` component is not modified.
- Toast: use `toast.push(msg, ToastKind::Success/Error)` — there are no `toast.success()`/`toast.error()` shorthand methods.

- [ ] **Step 2: Check if UpdateItemRequest has Default**

In `crates/shared/src/lib.rs`, the `UpdateItemRequest` struct (line 286) currently derives `Debug, Clone, Serialize, Deserialize`. It needs `Default` added:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateItemRequest {
```

- [ ] **Step 3: Register module in `pages/mod.rs`**

Add to `crates/frontend/src/pages/mod.rs`:

```rust
pub mod item_detail;
```

- [ ] **Step 4: Add route in `app.rs`**

In `crates/frontend/src/app.rs`, add after the `"/lists/:id"` route:

```rust
<Route path=path!("/lists/:list_id/items/:id") view=ItemDetailPage/>
```

Add the import at the top (in the existing use block for pages):

```rust
use pages::item_detail::ItemDetailPage;
```

- [ ] **Step 5: Verify compilation**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add crates/shared/src/lib.rs \
       crates/frontend/src/pages/item_detail.rs \
       crates/frontend/src/pages/mod.rs \
       crates/frontend/src/app.rs
git commit -m "feat: add item detail page with auto-save"
```

---

## Task 10: Smoke test & final verification

- [ ] **Step 1: Run full check**

Run: `just check`
Expected: Compiles without errors.

- [ ] **Step 2: Run tests**

Run: `just test`
Expected: All existing tests pass.

- [ ] **Step 3: Run linter**

Run: `just lint`
Expected: No clippy warnings or fmt issues.

- [ ] **Step 4: Fix any issues**

If any step above fails, fix the issue and re-run.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: address lint/test issues from item detail page"
```
