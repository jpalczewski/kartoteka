# Delete List with Confirmation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add delete-list UI with DaisyUI modal confirmation, toast notifications, and redirect — in both the home list view and the detail view.

**Architecture:** Global `ToastContext` via Leptos `provide_context` (survives navigation), a reusable `ConfirmDeleteModal` component (conditionally mounted, fetches item count on mount), delete button in `ListCard` (stop_propagation), and delete logic in `HomePage` (optimistic via RwSignal) and `ListPage` (navigate after delete).

**Tech Stack:** Leptos 0.7 CSR, WASM, DaisyUI 5, gloo-net 0.6, Cloudflare Workers D1

> **Note on testing:** This is a WASM/Leptos CSR project — there is no browser-based unit test runner. Each task's "verify" step is `just check` (Cargo compilation for the whole workspace). Manual browser verification is noted where relevant. Run `just check` after every code change before committing.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `crates/frontend/src/api.rs` | Modify | Fix `delete_list` to check HTTP status |
| `crates/frontend/src/app.rs` | Modify | Define `ToastContext`, `Toast`, `ToastKind`; `provide_context`; render `ToastContainer` |
| `crates/frontend/src/components/toast_container.rs` | Create | Renders active toasts as DaisyUI alert stack |
| `crates/frontend/src/components/confirm_delete_modal.rs` | Create | DaisyUI dialog with item count fetch, confirm/cancel callbacks |
| `crates/frontend/src/components/mod.rs` | Modify | Declare new modules |
| `crates/frontend/src/components/list_card.rs` | Modify | Add `on_delete` prop + trash button |
| `crates/frontend/src/pages/home.rs` | Modify | Promote `lists` to `RwSignal`, add modal + delete logic |
| `crates/frontend/src/pages/list.rs` | Modify | Add modal + delete button in header |

---

## Task 1: Fix `api::delete_list` to check HTTP status

**Files:**
- Modify: `crates/frontend/src/api.rs:98-104`

The current implementation ignores the HTTP response status code — a 404 or 500 from the server is silently treated as success. This must be fixed before building delete logic on top of it.

- [ ] **Step 1: Replace the function body**

In `crates/frontend/src/api.rs`, replace the existing `delete_list` function (lines 98–104):

```rust
pub async fn delete_list(id: &str) -> Result<(), String> {
    let resp = del(&format!("{API_BASE}/lists/{id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(format!("Błąd serwera: {}", resp.status()))
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
just check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/api.rs
git commit -m "fix: check HTTP status in delete_list API call"
```

---

## Task 2: Toast system — types and context in `app.rs`

**Files:**
- Modify: `crates/frontend/src/app.rs`

Define the toast types and `ToastContext` struct, wire up `provide_context`, and render a placeholder for `ToastContainer` (the real component comes in Task 3).

- [ ] **Step 1: Add toast types and context to `app.rs`**

Replace the full content of `crates/frontend/src/app.rs`:

```rust
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::nav::Nav;
use crate::components::toast_container::ToastContainer;
use crate::pages::{home::HomePage, list::ListPage, login::LoginPage, settings::SettingsPage, tags::TagsPage};

#[derive(Clone, Debug, PartialEq)]
pub enum ToastKind {
    Success,
    Error,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u32,
    pub message: String,
    pub kind: ToastKind,
}

#[derive(Clone, Copy)]
pub struct ToastContext {
    pub toasts: RwSignal<Vec<Toast>>,
    next_id: RwSignal<u32>,
}

impl ToastContext {
    pub fn new() -> Self {
        Self {
            toasts: RwSignal::new(Vec::new()),
            next_id: RwSignal::new(0),
        }
    }

    pub fn push(&self, message: String, kind: ToastKind) {
        let id = self.next_id.get();
        self.next_id.update(|n| *n += 1);
        self.toasts.update(|ts| ts.push(Toast { id, message, kind }));

        let toasts = self.toasts;
        set_timeout(
            move || toasts.update(|ts| ts.retain(|t| t.id != id)),
            std::time::Duration::from_millis(3000),
        );
    }

    pub fn dismiss(&self, id: u32) {
        self.toasts.update(|ts| ts.retain(|t| t.id != id));
    }
}

#[component]
pub fn App() -> impl IntoView {
    let toast_ctx = ToastContext::new();
    provide_context(toast_ctx);

    view! {
        <Router>
            <Nav/>
            <ToastContainer/>
            <main class="container">
                <Routes fallback=|| view! { <p>"Nie znaleziono strony"</p> }>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/login") view=LoginPage/>
                    <Route path=path!("/settings") view=SettingsPage/>
                    <Route path=path!("/tags") view=TagsPage/>
                    <Route path=path!("/lists/:id") view=ListPage/>
                </Routes>
            </main>
        </Router>
    }
}
```

- [ ] **Step 2: Declare `toast_container` module in `mod.rs`**

In `crates/frontend/src/components/mod.rs`, add:

```rust
pub mod toast_container;
```

(full file after edit: `pub mod add_input;`, `pub mod add_item_input;`, `pub mod item_row;`, `pub mod list_card;`, `pub mod nav;`, `pub mod tag_badge;`, `pub mod tag_selector;`, `pub mod toast_container;`)

- [ ] **Step 3: Create a stub `toast_container.rs` so it compiles**

Create `crates/frontend/src/components/toast_container.rs`:

```rust
use leptos::prelude::*;

#[component]
pub fn ToastContainer() -> impl IntoView {
    view! { <div/> }
}
```

- [ ] **Step 4: Verify compilation**

```bash
just check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add crates/frontend/src/app.rs crates/frontend/src/components/mod.rs crates/frontend/src/components/toast_container.rs
git commit -m "feat: add ToastContext and ToastKind to app, stub ToastContainer"
```

---

## Task 3: Implement `ToastContainer` component

**Files:**
- Modify: `crates/frontend/src/components/toast_container.rs`

Renders the DaisyUI toast stack. Reads from `ToastContext`. Each toast is an `alert` with a dismiss button.

- [ ] **Step 1: Implement `ToastContainer`**

Replace the stub in `crates/frontend/src/components/toast_container.rs`:

```rust
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};

#[component]
pub fn ToastContainer() -> impl IntoView {
    let ctx = use_context::<ToastContext>().expect("ToastContext missing");

    view! {
        <div class="toast toast-end z-50">
            {move || ctx.toasts.get().into_iter().map(|toast| {
                let id = toast.id;
                let alert_class = match toast.kind {
                    ToastKind::Success => "alert alert-success",
                    ToastKind::Error => "alert alert-error",
                };
                view! {
                    <div class=alert_class>
                        <span>{toast.message}</span>
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs"
                            on:click=move |_| ctx.dismiss(id)
                        >
                            "✕"
                        </button>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
just check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/components/toast_container.rs
git commit -m "feat: implement ToastContainer with DaisyUI alert stack"
```

---

## Task 4: Implement `ConfirmDeleteModal` component

**Files:**
- Create: `crates/frontend/src/components/confirm_delete_modal.rs`
- Modify: `crates/frontend/src/components/mod.rs`

DaisyUI `<dialog>` modal. Fetches item count on mount (unless `item_count` is provided by caller). Calls `on_confirm` or `on_cancel`.

- [ ] **Step 1: Add `confirm_delete_modal` module to `mod.rs`**

In `crates/frontend/src/components/mod.rs`, add:

```rust
pub mod confirm_delete_modal;
```

- [ ] **Step 2: Create the component**

Create `crates/frontend/src/components/confirm_delete_modal.rs`:

```rust
use leptos::prelude::*;

use crate::api;

#[derive(Clone)]
enum CountState {
    Loading,
    Loaded(usize),
    Error,
}

#[component]
pub fn ConfirmDeleteModal(
    list_name: String,
    list_id: String,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
    #[prop(optional)] item_count: Option<usize>,
) -> impl IntoView {
    let count_state = RwSignal::new(match item_count {
        Some(n) => CountState::Loaded(n),
        None => CountState::Loading,
    });

    // Fetch item count on mount if not provided
    if item_count.is_none() {
        let lid = list_id.clone();
        leptos::task::spawn_local(async move {
            match api::fetch_items(&lid).await {
                Ok(items) => count_state.set(CountState::Loaded(items.len())),
                Err(_) => count_state.set(CountState::Error),
            }
        });
    }

    let list_name = list_name.clone();

    view! {
        <dialog class="modal" open=true>
            <div class="modal-box">
                <h3 class="font-bold text-lg">"Usuń listę"</h3>

                {move || match count_state.get() {
                    CountState::Loading => view! {
                        <p class="py-4">"Wczytywanie szczegółów…"</p>
                    }.into_any(),
                    CountState::Error => view! {
                        <p class="py-4">
                            "Czy na pewno chcesz usunąć listę "
                            <strong>{list_name.clone()}</strong>
                            "? Operacja jest nieodwracalna."
                        </p>
                    }.into_any(),
                    CountState::Loaded(n) => view! {
                        <p class="py-4">
                            "Czy na pewno chcesz usunąć listę "
                            <strong>{list_name.clone()}</strong>
                            "? Zawiera "
                            <strong>{n}</strong>
                            " elementów. Operacja jest nieodwracalna."
                        </p>
                    }.into_any(),
                }}

                <div class="modal-action">
                    <button
                        type="button"
                        class="btn btn-ghost"
                        on:click=move |_| on_cancel.run(())
                    >
                        "Anuluj"
                    </button>
                    <button
                        type="button"
                        class="btn btn-error"
                        on:click=move |_| on_confirm.run(())
                    >
                        "Usuń listę"
                    </button>
                </div>
            </div>
            <div
                class="modal-backdrop"
                on:click=move |_| on_cancel.run(())
            />
        </dialog>
    }
}
```

- [ ] **Step 3: Verify compilation**

```bash
just check
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add crates/frontend/src/components/confirm_delete_modal.rs crates/frontend/src/components/mod.rs
git commit -m "feat: add ConfirmDeleteModal component with item count fetch"
```

---

## Task 5: Add delete button to `ListCard`

**Files:**
- Modify: `crates/frontend/src/components/list_card.rs`

Add optional `on_delete: Option<Callback<String>>` prop. Render trash button in the top-right corner when prop is provided. `stop_propagation` prevents triggering card navigation.

- [ ] **Step 1: Update `ListCard`**

In `crates/frontend/src/components/list_card.rs`, replace the full file:

```rust
use kartoteka_shared::{List, ListType, Tag};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::tag_badge::TagBadge;
use crate::components::tag_selector::TagSelector;

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
pub fn ListCard(
    list: List,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] list_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
    #[prop(optional)] on_delete: Option<Callback<String>>,
) -> impl IntoView {
    let href = format!("/lists/{}", list.id);
    let icon = list_type_icon(&list.list_type);
    let label = list_type_label(&list.list_type);

    let navigate = use_navigate();
    let href_clone = href.clone();

    let assigned_tags: Vec<Tag> = all_tags
        .iter()
        .filter(|t| list_tag_ids.contains(&t.id))
        .cloned()
        .collect();

    let list_id_for_delete = list.id.clone();
    let on_delete_clone = on_delete.clone();

    view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative"
            on:click=move |_| { navigate(&href_clone, Default::default()); }
        >
            // Delete button — positioned absolute, stop_propagation prevents card navigation
            {on_delete_clone.map(|cb| {
                let lid = list_id_for_delete.clone();
                view! {
                    <button
                        type="button"
                        aria-label="Usuń listę"
                        class="btn btn-ghost btn-xs absolute top-2 right-2 opacity-40 hover:opacity-100"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb.run(lid.clone());
                        }
                    >
                        "🗑"
                    </button>
                }
            })}

            <div class="card-body p-4">
                <h3 class="card-title text-base">{list.name.clone()}</h3>
                <span class="text-sm text-base-content/60">{icon} " " {label}</span>
                {if on_tag_toggle.is_some() || !assigned_tags.is_empty() {
                    view! {
                        <div
                            class="tag-list mt-2"
                            on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
                        >
                            {assigned_tags.into_iter().map(|t| {
                                let tid = t.id.clone();
                                let cb = on_tag_toggle.clone();
                                if let Some(c) = cb {
                                    let remove_cb = Callback::new(move |_: String| c.run(tid.clone()));
                                    view! { <TagBadge tag=t on_remove=remove_cb /> }.into_any()
                                } else {
                                    view! { <TagBadge tag=t /> }.into_any()
                                }
                            }).collect::<Vec<_>>()}
                            {on_tag_toggle.map(|cb| view! {
                                <TagSelector
                                    all_tags=all_tags.clone()
                                    selected_tag_ids=list_tag_ids.clone()
                                    on_toggle=cb
                                />
                            })}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </div>
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
just check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/components/list_card.rs
git commit -m "feat: add on_delete prop and trash button to ListCard"
```

---

## Task 6: Delete list from `HomePage`

**Files:**
- Modify: `crates/frontend/src/pages/home.rs`

Promote `lists` from `LocalResource` to a writable `RwSignal<Vec<List>>` (same pattern as `list_tag_links`). Add `pending_delete` signal, render `ConfirmDeleteModal`, handle optimistic delete with rollback.

- [ ] **Step 1: Update `home.rs`**

Replace the full content of `crates/frontend/src/pages/home.rs`:

```rust
use kartoteka_shared::{CreateListRequest, List, ListTagLink, ListType, Tag};
use leptos::prelude::*;

use crate::api;
use crate::app::{ToastContext, ToastKind};
use crate::components::add_input::AddInput;
use crate::components::confirm_delete_modal::ConfirmDeleteModal;
use crate::components::list_card::ListCard;

fn parse_list_type(s: &str) -> ListType {
    match s {
        "shopping" => ListType::Shopping,
        "packing" => ListType::Packing,
        "project" => ListType::Project,
        _ => ListType::Custom,
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    // Redirect to login if no Hanko token
    if !api::is_logged_in() {
        if let Some(w) = web_sys::window() {
            let _ = w.location().set_href("/login");
        }
    }

    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (new_list_type, set_new_list_type) = signal(ListType::Custom);
    let (refresh, set_refresh) = signal(0u32);
    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

    // pending_delete: (list_id, list_name) — drives the modal
    let pending_delete = RwSignal::new(Option::<(String, String)>::None);

    // Lists: fetched via LocalResource, kept in writable RwSignal for optimistic updates
    let lists_res = LocalResource::new(move || {
        let _ = refresh.get();
        api::fetch_lists()
    });
    let lists_data = RwSignal::new(Vec::<List>::new());
    Effect::new(move |_| {
        if let Some(data) = lists_res.get() {
            if let Ok(lists) = data.as_deref() {
                lists_data.set(lists.to_vec());
            }
        }
    });

    let tags_res = LocalResource::new(|| api::fetch_tags());
    let links_res = LocalResource::new(move || {
        let _ = refresh.get();
        api::fetch_list_tag_links()
    });

    let list_tag_links = RwSignal::new(Vec::<ListTagLink>::new());
    Effect::new(move |_| {
        if let Some(data) = links_res.get() {
            if let Some(links) = data.as_deref().ok().map(|s| s.to_vec()) {
                list_tag_links.set(links);
            }
        }
    });

    let on_list_tag_toggle = Callback::new(move |(list_id, tag_id): (String, String)| {
        let has_tag = list_tag_links
            .read()
            .iter()
            .any(|l| l.list_id == list_id && l.tag_id == tag_id);
        if has_tag {
            list_tag_links.update(|links| {
                links.retain(|l| !(l.list_id == list_id && l.tag_id == tag_id))
            });
            let lid = list_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_list(&lid, &tid).await;
            });
        } else {
            list_tag_links.update(|links| {
                links.push(ListTagLink {
                    list_id: list_id.clone(),
                    tag_id: tag_id.clone(),
                })
            });
            let lid = list_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_list(&lid, &tid).await;
            });
        }
    });

    let on_create = Callback::new(move |name: String| {
        let list_type = new_list_type.get();
        leptos::task::spawn_local(async move {
            let req = CreateListRequest { name, list_type };
            let _ = api::create_list(&req).await;
            set_refresh.update(|n| *n += 1);
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Twoje listy"</h2>

            // Tag filter bar
            <Suspense fallback=|| view! {}>
                {move || tags_res.get().map(|result| {
                    match &*result {
                        Ok(tags) if !tags.is_empty() => {
                            let tags = tags.clone();
                            view! {
                                <div class="tag-filter-bar">
                                    {tags.into_iter().map(|tag| {
                                        let tid = tag.id.clone();
                                        let tid2 = tag.id.clone();
                                        let tid3 = tag.id.clone();
                                        let color = tag.color.clone();
                                        let name = tag.name.clone();
                                        view! {
                                            <span
                                                class=move || if active_tag_filter.get().as_deref() == Some(tid.as_str()) { "tag-badge active" } else { "tag-badge" }
                                                style=format!("background: {}; color: white; cursor: pointer;", color)
                                                on:click=move |_| {
                                                    if active_tag_filter.get().as_deref() == Some(tid2.as_str()) {
                                                        set_active_tag_filter.set(None);
                                                    } else {
                                                        set_active_tag_filter.set(Some(tid3.clone()));
                                                    }
                                                }
                                            >
                                                {name}
                                            </span>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                        _ => view! {}.into_any()
                    }
                })}
            </Suspense>

            // Create form
            <div class="flex gap-2 mb-4">
                <select class="select select-bordered" on:change=move |ev| set_new_list_type.set(parse_list_type(&event_target_value(&ev)))>
                    <option value="custom">"Lista"</option>
                    <option value="shopping">"Zakupy"</option>
                    <option value="packing">"Pakowanie"</option>
                    <option value="project">"Projekt"</option>
                </select>
                <AddInput placeholder="Nazwa nowej listy..." button_label="Dodaj" on_submit=on_create />
            </div>

            // Delete confirmation modal (conditionally rendered)
            {move || pending_delete.get().map(|(lid, lname)| {
                let lid_confirm = lid.clone();
                view! {
                    <ConfirmDeleteModal
                        list_id=lid
                        list_name=lname
                        on_confirm=Callback::new(move |_| {
                            let lid = lid_confirm.clone();
                            leptos::task::spawn_local(async move {
                                // Optimistic: remove from local signal
                                let removed = lists_data.read().iter().find(|l| l.id == lid).cloned();
                                lists_data.update(|ls| ls.retain(|l| l.id != lid));
                                pending_delete.set(None);

                                match api::delete_list(&lid).await {
                                    Ok(()) => toast.push("Lista usunięta".into(), ToastKind::Success),
                                    Err(e) => {
                                        // Rollback
                                        if let Some(list) = removed {
                                            lists_data.update(|ls| ls.push(list));
                                        }
                                        toast.push(format!("Błąd: {e}"), ToastKind::Error);
                                    }
                                }
                            });
                        })
                        on_cancel=Callback::new(move |_| pending_delete.set(None))
                    />
                }
            })}

            // Lists grid
            {move || {
                let tags_data = tags_res.get();
                let all_tags: Vec<Tag> = tags_data
                    .as_ref()
                    .and_then(|r| r.as_deref().ok())
                    .map(|s| s.to_vec())
                    .unwrap_or_default();
                let all_links = list_tag_links.get();
                let filter = active_tag_filter.get();

                let all_lists = lists_data.get();
                if all_lists.is_empty() {
                    return view! {
                        <div class="text-center text-base-content/50 py-12">"Brak list. Utwórz pierwszą!"</div>
                    }.into_any();
                }

                let filtered_lists: Vec<List> = all_lists
                    .iter()
                    .filter(|l| match &filter {
                        None => true,
                        Some(tag_id) => all_links
                            .iter()
                            .any(|link| link.list_id == l.id && &link.tag_id == tag_id),
                    })
                    .cloned()
                    .collect();

                view! {
                    <div class="flex flex-col gap-3">
                        {filtered_lists.into_iter().map(|list| {
                            let list_id = list.id.clone();
                            let list_name = list.name.clone();
                            let list_tag_ids: Vec<String> = all_links
                                .iter()
                                .filter(|l| l.list_id == list.id)
                                .map(|l| l.tag_id.clone())
                                .collect();
                            let tog = on_list_tag_toggle.clone();
                            let tag_cb = Callback::new(move |tag_id: String| {
                                tog.run((list_id.clone(), tag_id));
                            });
                            let lid = list.id.clone();
                            view! {
                                <ListCard
                                    list
                                    all_tags=all_tags.clone()
                                    list_tag_ids
                                    on_tag_toggle=tag_cb
                                    on_delete=Callback::new(move |_: String| {
                                        pending_delete.set(Some((lid.clone(), list_name.clone())));
                                    })
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
just check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/pages/home.rs
git commit -m "feat: add delete list with confirmation and optimistic update to HomePage"
```

---

## Task 7: Delete list from `ListPage`

**Files:**
- Modify: `crates/frontend/src/pages/list.rs`

Add a delete button in the page header. Modal uses the already-loaded `items` count (no extra fetch). After successful delete: toast + navigate to `/`.

- [ ] **Step 1: Update `list.rs`**

In `crates/frontend/src/pages/list.rs`, make the following changes:

**Add imports** (replace existing import block at top):

```rust
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::api;
use crate::app::{ToastContext, ToastKind};
use crate::components::add_item_input::AddItemInput;
use crate::components::confirm_delete_modal::ConfirmDeleteModal;
use crate::components::item_row::ItemRow;
use crate::components::tag_badge::TagBadge;
use crate::components::tag_selector::TagSelector;
use kartoteka_shared::{
    CreateItemRequest, Item, ItemTagLink, ListTagLink, Tag, UpdateItemRequest,
};
```

**Add signals** after the existing signals (after line `let (error, set_error) = signal(...)`):

```rust
let toast = use_context::<ToastContext>().expect("ToastContext missing");
let navigate = use_navigate();
let show_delete = RwSignal::new(false);
```

**Replace the `<h2>` header** in the view (currently `<h2 class="text-2xl font-bold mb-4">"Lista"</h2>`) with:

```rust
<div class="flex items-center justify-between mb-4">
    <h2 class="text-2xl font-bold">"Lista"</h2>
    <button
        type="button"
        class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
        on:click=move |_| show_delete.set(true)
    >
        "🗑 Usuń listę"
    </button>
</div>
```

**Add the modal** after the `<div class="container...">` opening tag (before the tag management block):

```rust
// Delete confirmation modal
{move || {
    if show_delete.get() {
        let lid = list_id();
        let item_count = items.read().len();
        Some(view! {
            <ConfirmDeleteModal
                list_id=lid.clone()
                list_name="Lista".to_string()
                item_count=Some(item_count)
                on_confirm=Callback::new(move |_| {
                    let lid = lid.clone();
                    let nav = navigate.clone();
                    leptos::task::spawn_local(async move {
                        match api::delete_list(&lid).await {
                            Ok(()) => {
                                toast.push("Lista usunięta".into(), ToastKind::Success);
                                nav("/", Default::default());
                            }
                            Err(e) => {
                                toast.push(format!("Błąd: {e}"), ToastKind::Error);
                                show_delete.set(false);
                            }
                        }
                    });
                })
                on_cancel=Callback::new(move |_| show_delete.set(false))
            />
        })
    } else {
        None
    }
}}
```

- [ ] **Step 2: Verify compilation**

```bash
just check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/pages/list.rs
git commit -m "feat: add delete list with confirmation and redirect to ListPage"
```

---

## Task 8: Manual verification

- [ ] **Step 1: Start local dev environment**

```bash
just dev
```

Open browser at `http://localhost:8080`.

- [ ] **Step 2: Verify toast system**

Temporarily trigger a toast from the browser console is not possible with WASM — use the delete flow below to verify toasts appear and auto-dismiss after 3 seconds.

- [ ] **Step 3: Verify delete from home page**

1. Create a test list with a few items via the UI
2. On the home page, verify the 🗑 trash icon appears in the top-right corner of each list card (subtle opacity)
3. Click the trash icon — verify the modal opens with the list name and item count
4. Click [Anuluj] — modal closes, list remains
5. Click 🗑 again, click [Usuń listę] — list disappears immediately (optimistic), toast "Lista usunięta" appears, auto-dismisses after 3s

- [ ] **Step 4: Verify delete from detail page**

1. Open a list's detail page
2. Verify "🗑 Usuń listę" button in the header
3. Click it — modal opens with item count (no loading spinner since count is passed directly)
4. Click [Anuluj] — modal closes
5. Click the button again, click [Usuń listę] — redirected to `/`, toast "Lista usunięta" visible

- [ ] **Step 5: Verify error handling**

Stop the local API (`Ctrl+C` on `just dev-api`) while the frontend is running. Attempt to delete a list — verify error toast appears and the list card is restored (rollback on home page).
