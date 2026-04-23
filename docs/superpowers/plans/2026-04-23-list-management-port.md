# List Management Port — Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port four missing list management features from v1 (CSR) to v2 (SSR): delete from list detail, confirmation dialogs for destructive actions, deadlines sub-config (3 date-field flags), and inline list-description edit. Along the way, collapse the normalized `list_features` table into a single `lists.features` JSON column.

**Architecture:** Schema change first (new column, data copy, drop old table), then bottom-up code refactor (db → domain → server_fn → frontend-v2 components → page wiring).

**Tech Stack:** SQLite via sqlx (bundled), Rust domain layer, Leptos SSR with server functions, DaisyUI modals.

**Spec:** `docs/superpowers/specs/2026-04-23-list-management-port-design.md`

---

## Task 1: Add migration `004_lists_features_json.sql`

**Files:**
- Create: `crates/db/migrations/004_lists_features_json.sql`

- [ ] **Step 1: Write the migration**

```sql
-- crates/db/migrations/004_lists_features_json.sql
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

- [ ] **Step 2: Verify migration applies cleanly**

Run: `cargo test -p kartoteka-db --lib -- --ignored` to exercise `test_pool()` which applies migrations.
Expected: migration runs, tests still compile (they may fail later — that's Task 2+).

If `cargo test` fails on migration SQL itself (syntax, FK), fix the migration before moving on. Do **not** proceed to Task 2 with a broken migration.

- [ ] **Step 3: Commit**

```bash
git add crates/db/migrations/004_lists_features_json.sql
git commit -m "feat(db): migrate list features to JSON column on lists"
```

---

## Task 2: Refactor `db::lists` — row types, SELECT queries, `set_features`

**Files:**
- Modify: `crates/db/src/lists.rs`

- [ ] **Step 1: Rename row field `features_json` → `features`**

Both `ListRow` (around line 22) and `ListProjectionRow` (around line 41) have `pub features_json: String`. Rename to `pub features: String`. This field now comes directly from the `lists.features` column (no subquery).

- [ ] **Step 2: Remove the `FEATURES_SUBQUERY` constant and its usages**

Delete the `FEATURES_SUBQUERY` const (around line 48). Every `SELECT l.*, {FEATURES_SUBQUERY} FROM lists l` becomes `SELECT l.* FROM lists l` — the new `features` column is already included in `l.*`.

Search for every `format!(...)` with `FEATURES_SUBQUERY` and remove the placeholder. For the multi-line SELECT at ~line 208, remove the subquery lines and the ` as features_json` alias; keep `l.*`.

- [ ] **Step 3: Rewrite `replace_features` as `set_features`**

Replace the existing function (lines 270–294) with:

```rust
/// Replace the full features JSON for a list. Caller must be in a transaction.
#[tracing::instrument(skip(tx))]
pub async fn set_features(
    tx: &mut SqliteConnection,
    list_id: &str,
    features: &serde_json::Value,
) -> Result<(), DbError> {
    let json = serde_json::to_string(features)
        .map_err(|e| DbError::Sqlx(sqlx::Error::Decode(Box::new(e))))?;
    sqlx::query("UPDATE lists SET features = ? WHERE id = ?")
        .bind(json)
        .bind(list_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(())
}
```

- [ ] **Step 4: Run `cargo check -p kartoteka-db`**

Expected: compiles. Domain crate will **not** compile yet — that's Task 3.

- [ ] **Step 5: Commit**

```bash
git add crates/db/src/lists.rs
git commit -m "refactor(db): read lists.features column, rewrite set_features as UPDATE"
```

---

## Task 3: Update `domain::lists` deserialization and callers

**Files:**
- Modify: `crates/domain/src/lists.rs`

- [ ] **Step 1: Update List row → List conversion (~line 103)**

Current code:
```rust
let features: Vec<ListFeature> = serde_json::from_str(&row.features_json)
    .map_err(...)?;
```

New code (parse object, project to Vec):
```rust
let features_obj: serde_json::Map<String, serde_json::Value> =
    serde_json::from_str(&row.features)
        .map_err(|e| DomainError::Internal(e.to_string()))?;
let features: Vec<ListFeature> = features_obj
    .into_iter()
    .map(|(name, config)| ListFeature { feature_name: name, config })
    .collect();
```

Apply the same change at the validation path around line 223 (`serde_json::from_str(&current.features_json)`).

- [ ] **Step 2: Update `create` feature write (~line 197)**

Current:
```rust
if !req.features.is_empty() {
    db::lists::replace_features(&mut tx, &list_id, &req.features).await?;
}
```

New:
```rust
if !req.features.is_empty() {
    let obj: serde_json::Map<String, serde_json::Value> = req.features
        .iter()
        .map(|name| (name.clone(), serde_json::json!({})))
        .collect();
    db::lists::set_features(&mut tx, &list_id, &serde_json::Value::Object(obj)).await?;
}
```

- [ ] **Step 3: Update `set_features` wrapper (~line 341)**

Analogous: build JSON object from `req.features: Vec<String>`, call `db::lists::set_features`.

- [ ] **Step 4: Run `cargo check -p kartoteka-domain`**

Expected: compiles. `domain::home` may still have the old field name — Task 4 handles that. If compilation fails for `home.rs`, move to Task 4 before committing.

- [ ] **Step 5: Commit (if domain compiles in isolation; else fold into Task 4 commit)**

```bash
git add crates/domain/src/lists.rs
git commit -m "refactor(domain): parse lists.features as JSON object, write via set_features"
```

---

## Task 4: Update remaining `features_json` consumers

**Files:**
- Modify: `crates/domain/src/home.rs`
- Modify: `crates/domain/src/items.rs` (test helper only)

- [ ] **Step 1: Fix `domain/home.rs:24`**

Change `row.features_json` → `row.features`, swap deserialization from `Vec<ListFeature>` to object-then-project (same pattern as Task 3 Step 1).

- [ ] **Step 2: Fix test helper in `domain/items.rs:335`**

Current:
```rust
for feature in features {
    sqlx::query("INSERT INTO list_features (list_id, feature_name) VALUES (?, ?)")
        .bind(&list_id)
        .bind(feature)
        .execute(pool)
        .await
        .unwrap();
}
```

New:
```rust
let obj: serde_json::Map<String, serde_json::Value> = features
    .iter()
    .map(|f| (f.to_string(), serde_json::json!({})))
    .collect();
let json = serde_json::to_string(&serde_json::Value::Object(obj)).unwrap();
sqlx::query("UPDATE lists SET features = ? WHERE id = ?")
    .bind(json)
    .bind(&list_id)
    .execute(pool)
    .await
    .unwrap();
```

- [ ] **Step 3: Run `cargo check --workspace`**

Expected: workspace compiles clean.

- [ ] **Step 4: Commit**

```bash
git add crates/domain/src/home.rs crates/domain/src/items.rs
git commit -m "refactor(domain): update home + items test helper for features JSON column"
```

---

## Task 5: Update `db::lists` tests

**Files:**
- Modify: `crates/db/src/lists.rs` (tests at lines 447, 481, 487, 519)

- [ ] **Step 1: Fix assertions**

At line 447 (`assert_eq!(row.features_json, "[]")`): change to `assert_eq!(row.features, "{}")` (default empty object).

At lines 481 and 519 (`replace_features(&mut tx, &id, &["deadlines".into(), ...])`): replace with direct calls to the new `set_features`, passing a JSON object:

```rust
let feats = serde_json::json!({"deadlines": {}, "quantity": {}});
set_features(&mut tx, &id, &feats).await.unwrap();
```

At line 487 (`let features: Vec<serde_json::Value> = serde_json::from_str(&row.features_json).unwrap()`): change to parse object and assert keys, e.g.:

```rust
let obj: serde_json::Map<String, serde_json::Value> =
    serde_json::from_str(&row.features).unwrap();
assert!(obj.contains_key("deadlines"));
assert!(obj.contains_key("quantity"));
```

- [ ] **Step 2: Run db tests**

Run: `cargo test -p kartoteka-db --lib`
Expected: PASS.

- [ ] **Step 3: Run full test sweep**

Run: `cargo test --workspace --exclude kartoteka-frontend --exclude kartoteka-api`
Expected: PASS (confirms Tasks 1–5 integrated cleanly).

- [ ] **Step 4: Commit**

```bash
git add crates/db/src/lists.rs
git commit -m "test(db): update list features tests for JSON column"
```

---

## Task 6: Add `domain::lists::update_feature_config` with tests

**Files:**
- Modify: `crates/domain/src/lists.rs`

- [ ] **Step 1: Write the failing test**

Add after the existing tests:

```rust
#[tokio::test]
async fn update_feature_config_sets_single_feature() {
    let pool = kartoteka_db::test_helpers::test_pool().await;
    let uid = kartoteka_db::test_helpers::create_test_user(&pool).await;

    let list = create(
        &pool,
        &uid,
        &CreateListRequest {
            name: "L".into(),
            list_type: Some("checklist".into()),
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: None,
            features: vec!["deadlines".into()],
        },
    )
    .await
    .unwrap();

    let new_cfg = serde_json::json!({"has_start_date": true, "has_deadline": true});
    update_feature_config(&pool, &uid, &list.id, "deadlines", new_cfg.clone())
        .await
        .unwrap();

    let reloaded = get_one(&pool, &list.id, &uid).await.unwrap().unwrap();
    let deadlines_feat = reloaded
        .features
        .iter()
        .find(|f| f.feature_name == "deadlines")
        .unwrap();
    assert_eq!(deadlines_feat.config, new_cfg);
}

#[tokio::test]
async fn update_feature_config_rejects_disabled_feature() {
    let pool = kartoteka_db::test_helpers::test_pool().await;
    let uid = kartoteka_db::test_helpers::create_test_user(&pool).await;

    let list = create(
        &pool,
        &uid,
        &CreateListRequest {
            name: "L".into(),
            list_type: Some("checklist".into()),
            icon: None, description: None, container_id: None,
            parent_list_id: None, features: vec![],
        },
    )
    .await
    .unwrap();

    let err = update_feature_config(
        &pool,
        &uid,
        &list.id,
        "deadlines",
        serde_json::json!({}),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p kartoteka-domain update_feature_config`
Expected: FAIL with "cannot find function `update_feature_config`".

- [ ] **Step 3: Implement `update_feature_config`**

Add to `crates/domain/src/lists.rs`:

```rust
#[tracing::instrument(skip(pool, config))]
pub async fn update_feature_config(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    feature_name: &str,
    config: serde_json::Value,
) -> Result<(), DomainError> {
    let current = db::lists::get_one(pool, list_id, user_id)
        .await?
        .ok_or_else(|| DomainError::NotFound("list".into()))?;
    let mut obj: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&current.features)
            .map_err(|e| DomainError::Internal(e.to_string()))?;
    if !obj.contains_key(feature_name) {
        return Err(DomainError::NotFound("feature".into()));
    }
    obj.insert(feature_name.to_string(), config);
    let mut tx = pool.begin().await?;
    db::lists::set_features(&mut tx, list_id, &serde_json::Value::Object(obj)).await?;
    tx.commit().await?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p kartoteka-domain update_feature_config`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/lists.rs
git commit -m "feat(domain): add update_feature_config for per-feature JSON config"
```

---

## Task 7: Add `update_feature_config` server fn

**Files:**
- Modify: `crates/frontend-v2/src/server_fns/lists.rs`

- [ ] **Step 1: Add the server fn**

Add near the end of the file, after `reset_list`:

```rust
#[server(UpdateFeatureConfig, "/api")]
pub async fn update_feature_config(
    list_id: String,
    feature_name: String,
    config: serde_json::Value,
) -> Result<(), ServerFnError> {
    use axum_login::AuthSession;
    let auth = leptos_axum::extract::<AuthSession<crate::auth::Backend>>().await?;
    let user = auth.user.ok_or_else(|| ServerFnError::new("unauthorized"))?;
    let pool = crate::server_fns::pool()?;
    kartoteka_domain::lists::update_feature_config(
        &pool,
        &user.id,
        &list_id,
        &feature_name,
        config,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))
}
```

(Exact auth extractor pattern: mirror what `rename_list` uses in the same file. Do **not** invent — copy from an existing fn.)

- [ ] **Step 2: Run `cargo check -p kartoteka-frontend-v2`**

If using `cargo leptos build` from CLAUDE.md: run `cargo leptos build` instead.
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend-v2/src/server_fns/lists.rs
git commit -m "feat(server_fns): add update_feature_config"
```

---

## Task 8: Add `ConfirmModal` component

**Files:**
- Create: `crates/frontend-v2/src/components/common/confirm_modal.rs`
- Modify: `crates/frontend-v2/src/components/common/mod.rs`

- [ ] **Step 1: Write the component**

```rust
// crates/frontend-v2/src/components/common/confirm_modal.rs
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ConfirmVariant {
    Danger,
    Warning,
}

#[component]
pub fn ConfirmModal(
    #[prop(into)] open: Signal<bool>,
    title: String,
    message: String,
    confirm_label: String,
    variant: ConfirmVariant,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let btn_class = match variant {
        ConfirmVariant::Danger => "btn btn-error",
        ConfirmVariant::Warning => "btn btn-warning",
    };
    view! {
        {move || open.get().then(|| {
            let on_confirm = on_confirm;
            let on_cancel = on_cancel;
            view! {
                <div class="modal modal-open" role="dialog">
                    <div class="modal-box">
                        <h3 class="font-bold text-lg">{title.clone()}</h3>
                        <p class="py-4">{message.clone()}</p>
                        <div class="modal-action">
                            <button
                                type="button"
                                class="btn btn-ghost"
                                on:click=move |_| on_cancel.run(())
                            >"Anuluj"</button>
                            <button
                                type="button"
                                class=btn_class
                                on:click=move |_| on_confirm.run(())
                            >{confirm_label.clone()}</button>
                        </div>
                    </div>
                    <div class="modal-backdrop" on:click=move |_| on_cancel.run(())></div>
                </div>
            }
        })}
    }
}
```

- [ ] **Step 2: Register the module**

In `crates/frontend-v2/src/components/common/mod.rs` add:

```rust
pub mod confirm_modal;
```

- [ ] **Step 3: Run `cargo leptos build`**

Expected: compiles.

- [ ] **Step 4: Commit**

```bash
git add crates/frontend-v2/src/components/common/confirm_modal.rs crates/frontend-v2/src/components/common/mod.rs
git commit -m "feat(frontend-v2): add ConfirmModal component"
```

---

## Task 9: Wire confirm modal for Delete / Archive / Reset in list page

**Files:**
- Modify: `crates/frontend-v2/src/pages/list/mod.rs`

- [ ] **Step 1: Add `PendingAction` enum and signal at page scope**

Inside `ListPage`, above the main view:

```rust
#[derive(Clone)]
enum PendingAction {
    Delete,
    Archive,
    Reset,
}
let pending: RwSignal<Option<PendingAction>> = RwSignal::new(None);
```

Also import `delete_list` at the top of the file from `server_fns::lists`.

- [ ] **Step 2: Change existing Archive and Reset dropdown buttons to set `pending`**

Find the Archive button (around line 438 — `data-testid="action-archive"`). Replace its `on:click` body from spawning `archive_list(...)` directly to:

```rust
on:click=move |_| pending.set(Some(PendingAction::Archive))
```

Same for Reset (around line 361, `data-testid="action-reset"`): set `PendingAction::Reset`.

- [ ] **Step 3: Add a new Delete button at the bottom of the dropdown**

After the Archive `<li>`, add:

```rust
<li>
    <button
        type="button"
        class="text-error"
        data-testid="action-delete"
        on:click=move |_| pending.set(Some(PendingAction::Delete))
    >
        "🗑 Usuń listę"
    </button>
</li>
```

- [ ] **Step 4: Render `<ConfirmModal>` bound to `pending`**

At the end of the page view (before the closing `</div>` that wraps the whole list page), add:

```rust
{move || pending.get().map(|action| {
    let (title, message, confirm_label, variant) = match action.clone() {
        PendingAction::Delete => (
            "Usunąć listę?".to_string(),
            format!("Lista '{}' zostanie trwale usunięta wraz z elementami.", list_name.clone()),
            "Usuń".to_string(),
            ConfirmVariant::Danger,
        ),
        PendingAction::Archive => (
            "Zarchiwizować listę?".to_string(),
            "Listę można będzie przywrócić później.".to_string(),
            "Archiwizuj".to_string(),
            ConfirmVariant::Warning,
        ),
        PendingAction::Reset => (
            "Zresetować ukończone elementy?".to_string(),
            "Wszystkie elementy zostaną oznaczone jako niewykonane.".to_string(),
            "Resetuj".to_string(),
            ConfirmVariant::Warning,
        ),
    };
    let nav_for_confirm = navigate.clone();
    let on_confirm = Callback::new(move |_| {
        let lid = list_id();
        let action = action.clone();
        let nav = nav_for_confirm.clone();
        leptos::task::spawn_local(async move {
            let result = match action {
                PendingAction::Delete => delete_list(lid).await.map(|_| Some("/")),
                PendingAction::Archive => archive_list(lid).await.map(|_| Some("/")),
                PendingAction::Reset => reset_list(lid).await.map(|_| None),
            };
            match result {
                Ok(Some(path)) => nav(path, Default::default()),
                Ok(None) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
        pending.set(None);
    });
    let on_cancel = Callback::new(move |_| pending.set(None));
    view! {
        <ConfirmModal
            open=Signal::derive(move || pending.get().is_some())
            title=title
            message=message
            confirm_label=confirm_label
            variant=variant
            on_confirm=on_confirm
            on_cancel=on_cancel
        />
    }
})}
```

Import `ConfirmModal` and `ConfirmVariant` at the top:
```rust
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
```

- [ ] **Step 4.1: Build and smoke-test**

Run: `cargo leptos build` then `just dev`.
Open `/list/<some-id>`, open dropdown (⋮), click Delete → confirmation appears → confirm → redirected to `/`, list is gone.
Repeat for Archive (redirects to `/`) and Reset (stays, items uncompleted).

- [ ] **Step 5: Commit**

```bash
git add crates/frontend-v2/src/pages/list/mod.rs
git commit -m "feat(frontend-v2): confirmation dialog for delete/archive/reset list"
```

---

## Task 10: Add `DeadlinesConfig` component

**Files:**
- Create: `crates/frontend-v2/src/components/lists/deadlines_config.rs`
- Modify: `crates/frontend-v2/src/components/lists/mod.rs`

- [ ] **Step 1: Write the component**

```rust
// crates/frontend-v2/src/components/lists/deadlines_config.rs
use leptos::prelude::*;
use crate::app::{ToastContext, ToastKind};
use crate::server_fns::lists::update_feature_config;

#[component]
pub fn DeadlinesConfig(
    list_id: String,
    config: serde_json::Value,
    on_changed: Callback<()>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let get_bool = |k: &str, default: bool| -> bool {
        config.get(k).and_then(|v| v.as_bool()).unwrap_or(default)
    };
    let has_start = RwSignal::new(get_bool("has_start_date", false));
    let has_deadline = RwSignal::new(get_bool("has_deadline", true));
    let has_hard = RwSignal::new(get_bool("has_hard_deadline", false));

    let list_id_shared = list_id;
    let save = move || {
        let cfg = serde_json::json!({
            "has_start_date": has_start.get(),
            "has_deadline": has_deadline.get(),
            "has_hard_deadline": has_hard.get(),
        });
        let lid = list_id_shared.clone();
        let toast = toast.clone();
        let on_changed = on_changed;
        leptos::task::spawn_local(async move {
            match update_feature_config(lid, "deadlines".into(), cfg).await {
                Ok(_) => on_changed.run(()),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="flex flex-col gap-1 pl-6 py-1 text-xs">
            <label class="label cursor-pointer gap-2 justify-start py-0">
                <input
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    prop:checked=has_start
                    on:change={
                        let save = save.clone();
                        move |ev| { has_start.set(event_target_checked(&ev)); save(); }
                    }
                />
                <span>"Data startu"</span>
            </label>
            <label class="label cursor-pointer gap-2 justify-start py-0">
                <input
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    prop:checked=has_deadline
                    on:change={
                        let save = save.clone();
                        move |ev| { has_deadline.set(event_target_checked(&ev)); save(); }
                    }
                />
                <span>"Termin"</span>
            </label>
            <label class="label cursor-pointer gap-2 justify-start py-0">
                <input
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    prop:checked=has_hard
                    on:change={
                        let save = save.clone();
                        move |ev| { has_hard.set(event_target_checked(&ev)); save(); }
                    }
                />
                <span>"Twardy termin"</span>
            </label>
        </div>
    }
}
```

Note: `save` closure clones `list_id_shared` each call because v2 patterns favor cloning `String` over `Arc` for short-lived values. Match `use event_target_checked` import — import it from `leptos::prelude::event_target_checked` if the top of the file doesn't already bring it.

- [ ] **Step 2: Register the module**

In `crates/frontend-v2/src/components/lists/mod.rs` add:

```rust
pub mod deadlines_config;
```

- [ ] **Step 3: Build**

Run: `cargo leptos build`
Expected: compiles.

- [ ] **Step 4: Commit**

```bash
git add crates/frontend-v2/src/components/lists/deadlines_config.rs crates/frontend-v2/src/components/lists/mod.rs
git commit -m "feat(frontend-v2): DeadlinesConfig sub-panel component"
```

---

## Task 11: Wire `DeadlinesConfig` into list page dropdown

**Files:**
- Modify: `crates/frontend-v2/src/pages/list/mod.rs`

- [ ] **Step 1: Import the component**

Add at top:
```rust
use crate::components::lists::deadlines_config::DeadlinesConfig;
```

- [ ] **Step 2: Extract the current deadlines config from the data**

Inside the main data block where `current_features` is built (around line 276–283), add:

```rust
let deadlines_cfg = data.list.features
    .iter()
    .find(|f| f.feature_name == FEATURE_DEADLINES)
    .map(|f| f.config.clone())
    .unwrap_or_else(|| serde_json::json!({}));
```

- [ ] **Step 3: Render `DeadlinesConfig` below the "Terminy" checkbox**

In the dropdown (around line 410–420), just below the Terminy `<li>` entry, add:

```rust
{move || has_deadlines.then(|| {
    let lid = list_id();
    view! {
        <li>
            <DeadlinesConfig
                list_id=lid
                config=deadlines_cfg.clone()
                on_changed=Callback::new(move |_| set_refresh.update(|n| *n += 1))
            />
        </li>
    }
})}
```

Note: `has_deadlines` is already a `bool` computed from features — check whether it needs to be wrapped in a `Signal` for reactivity here. If the dropdown re-renders on data refresh (it does, since `current_features` is inside the `data_res` Suspense), this works without reactive wrapping.

- [ ] **Step 4: Build and smoke-test**

Run: `cargo leptos build` then verify in browser:
- Open `/list/<id>`, open dropdown, enable "Terminy" if not enabled.
- Three sub-checkboxes appear.
- Toggle one, check an item detail page — the corresponding date field appears/disappears.

- [ ] **Step 5: Commit**

```bash
git add crates/frontend-v2/src/pages/list/mod.rs
git commit -m "feat(frontend-v2): deadlines sub-config in list dropdown"
```

---

## Task 12: Inline edit for list description

**Files:**
- Modify: `crates/frontend-v2/src/pages/list/mod.rs`

- [ ] **Step 1: Add editing state**

Alongside `editing_name` (near the top of `ListPage`):
```rust
let (editing_desc, set_editing_desc) = signal(false);
let (desc_input, set_desc_input) = signal(String::new());
```

- [ ] **Step 2: Replace the description render**

Find the current description render (around line 463):
```rust
{list_description.map(|desc| view! {
    <p class="text-base-content/60 mb-4">{desc}</p>
})}
```

Replace with click-to-edit (mirrors the name-edit pattern):

```rust
{move || if editing_desc.get() {
    let lid = list_id();
    let current_name = list_name.clone();
    view! {
        <input
            type="text"
            class="input input-bordered text-sm w-full mb-4"
            data-testid="list-desc-input"
            prop:value=desc_input
            on:input=move |ev| set_desc_input.set(event_target_value(&ev))
            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                if ev.key() == "Enter" {
                    let lid2 = lid.clone();
                    let name = current_name.clone();
                    let new_desc = desc_input.get_untracked();
                    set_editing_desc.set(false);
                    leptos::task::spawn_local(async move {
                        match rename_list(lid2, name, Some(new_desc)).await {
                            Ok(_) => set_refresh.update(|n| *n += 1),
                            Err(e) => toast.push(e.to_string(), ToastKind::Error),
                        }
                    });
                } else if ev.key() == "Escape" {
                    set_editing_desc.set(false);
                }
            }
        />
    }.into_any()
} else {
    let desc_for_click = list_description.clone().unwrap_or_default();
    let desc_display = list_description.clone().unwrap_or_else(|| "Dodaj opis...".to_string());
    let is_empty = list_description.is_none();
    let class = if is_empty {
        "text-base-content/40 italic mb-4 cursor-pointer hover:underline decoration-dotted"
    } else {
        "text-base-content/60 mb-4 cursor-pointer hover:underline decoration-dotted"
    };
    view! {
        <p
            class=class
            data-testid="list-desc"
            on:click=move |_| {
                set_desc_input.set(desc_for_click.clone());
                set_editing_desc.set(true);
            }
        >
            {desc_display}
        </p>
    }.into_any()
}}
```

Note: the current `list_description.clone()` variable is captured by the outer closure. Verify it exists in scope at the replacement site (it does — already used for the old render).

- [ ] **Step 3: Build and smoke-test**

Run: `cargo leptos build` and verify:
- List without description: shows "Dodaj opis..." dimmed. Click → input appears. Type → Enter → saved, displayed.
- List with description: click description → input pre-filled. Edit → Enter → saved.
- Escape cancels without saving.

- [ ] **Step 4: Commit**

```bash
git add crates/frontend-v2/src/pages/list/mod.rs
git commit -m "feat(frontend-v2): inline edit for list description"
```

---

## Task 13: Final sweep — full test + manual smoke

- [ ] **Step 1: Workspace test**

Run: `cargo test --workspace --exclude kartoteka-frontend --exclude kartoteka-api`
Expected: all PASS.

- [ ] **Step 2: Lint + format**

Run: `just ci`
Expected: clean.

- [ ] **Step 3: Manual smoke checklist**

With `just dev` running, verify end-to-end on at least one real list:
- Create a list → open detail page → dropdown shows rename, pin, reset, feature toggles, archive, delete.
- Click Delete → confirmation modal → cancel works, confirm deletes and redirects to `/`.
- Click Archive → warning modal → confirm archives (list disappears from `/`).
- Click Reset on a list with completed items → warning modal → confirm resets all.
- Toggle Terminy → sub-panel appears with 3 checkboxes. Flip each → open an item of this list → corresponding date field toggles visibility.
- Click list description → edit → Enter saves. Clear description by deleting text and pressing Enter.
- Lists created pre-migration still render correctly (features list is intact).

- [ ] **Step 4: Commit any fixes, then ready for review**

```bash
git status  # should be clean
```
