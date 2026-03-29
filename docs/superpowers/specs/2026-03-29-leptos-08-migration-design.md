# Leptos 0.8 Migration & Frontend Architecture Refactor

**Date:** 2026-03-29
**Status:** Approved
**Related:** Issue #24 (Cloudflare exit)

## Motivation

The frontend needs:
1. **Leptos 0.8 upgrade** — simplified `LocalResource` API, no more `SendWrapper` in public interface
2. **Testable business logic** — today all logic is embedded in Leptos components, untestable without WASM
3. **Type deduplication** — several DTOs duplicated between frontend and API (`PreferencesResponse`, `ErrorResponse`)
4. **Preparation for #24** — decouple from Cloudflare/gloo-net specifics so the Axum rewrite is smoother

## Approach: Inside-Out (Iterative)

Four sequential PRs, each independently shippable. Each step builds on the previous but the codebase is functional after every PR.

## PR 1: Leptos 0.8 Bump

Mechanical upgrade, no logic changes. One production bugfix included.

### Changes
- `Cargo.toml`: `leptos`, `leptos_router`, `leptos_meta` → `0.8`
- Remove `&*result` / `&**s` dereferences on `LocalResource::get()` (0.8 returns `Option<T>` directly)
- Remove explicit `SendWrapper::new()` in `nav.rs` and `oauth_consent.rs`
- Remove `send_wrapper` from dependencies
- Unify `wasm_bindgen_futures::spawn_local` → `leptos::task::spawn_local` everywhere
- Check `leptos-fluent` 0.2 compatibility with Leptos 0.8 (bump if needed)
- Verify `gloo-net` 0.7, `web-sys`, `wasm-bindgen` compatibility with Leptos 0.8
- Optionally: add `--cfg=erase_components` to `RUSTFLAGS` in `Trunk.toml` for faster dev builds
- **Bugfix:** add missing `credentials: include` on `reset_list` in `api/lists.rs` (production auth bug, should not wait for PR 3)

### Not in scope
- No structural refactoring
- No new types
- No logic changes (beyond the credentials bugfix)

## PR 2: Shared Crate Expansion

Split `shared/src/lib.rs` (~440 lines, single file) into modules. Add missing types. Move `date_utils` from frontend.

### New structure

```
crates/shared/src/
  lib.rs            -- public re-exports
  models/
    mod.rs
    container.rs    -- Container, ContainerDetail, ContainerStatus
    item.rs         -- Item, DateItem, DaySummary, DayItems
    list.rs         -- List, ListType, ListFeature, DateField
    tag.rs          -- Tag, ItemTagLink, ListTagLink
    settings.rs     -- UserSetting
  dto/
    mod.rs
    requests.rs     -- Create/Update/Move request DTOs
    responses.rs    -- ItemDetailResponse, ContainerChildrenResponse,
                       PreferencesResponse, ErrorResponse, HomeData (deduplicated)
  deserializers.rs  -- bool_from_number, u32_from_number, features_from_json
  date_utils.rs     -- moved from frontend, js_sys replaced with `now` parameter
  constants.rs      -- FEATURE_*, DATE_TYPE_*, SETTING_*
```

### Key decisions
- **Re-exports from `lib.rs`** — `pub use models::*; pub use dto::*;` etc. so existing imports in `crates/api` continue to work without changes. Flat re-export avoids churn in API crate.
- **`HomeData` fix** — current `HomeData` struct in shared has `pinned: Vec<HomeItem>` and `recent: Vec<HomeItem>`, but the actual API response returns 6 separate fields: `pinned_lists`, `pinned_containers`, `recent_lists`, `recent_containers`, `root_containers`, `root_lists`. Fix: rewrite `HomeData` to match the actual API response shape. Move to `dto/responses.rs` (it's a response DTO, not a domain model). Make frontend use the typed struct instead of `serde_json::Value`.
- **`date_utils` migration** — move pure date math and business logic to shared. Functions using `js_sys::Date` (`get_today_string`, `current_time_hhmm`) get refactored to accept parameters: `get_today_string(year: u32, month: u32, day: u32)` and `current_time_hhmm(hours: u32, minutes: u32)`. `is_overdue` and `is_overdue_for_date_type` also need `today: &str` and `now_time: &str` parameters (they call `current_time_hhmm` internally). Frontend thin wrappers read `js_sys::Date` and pass values. Polish-specific formatting functions (`polish_month_abbr`, `format_polish_date`, etc.) stay in the frontend crate — they are presentation logic, not business logic.
- **New shared types from deduplication:**
  - `PreferencesResponse { locale: String }` — currently duplicated in API and frontend
  - `ErrorResponse { code: String, status: u16 }` — API defines `ErrorResponse`, frontend defines `ErrorBody`; unify
  - `ContainerChildrenResponse { containers: Vec<Container>, lists: Vec<List> }` — currently untyped `serde_json::Value` on both sides

### Tests
- Unit tests for every new module
- `date_utils` gets full coverage (pure Rust, no WASM needed)
- Existing tests in `crates/shared/src/tests/` updated for new module paths

## PR 3: HttpClient Trait & API Layer

Abstract HTTP calls behind a trait. Fix error handling inconsistencies. Enable testing without WASM.

### Design

```rust
// crates/frontend/src/api/client.rs

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub trait HttpClient {
    async fn request(
        &self,
        method: Method,
        url: &str,
        body: Option<&str>,
    ) -> Result<HttpResponse, ApiError>;
}

pub struct GlooClient;
impl HttpClient for GlooClient { /* gloo-net + credentials: include */ }
```

### Changes
- New `client.rs` with `HttpClient` trait and `GlooClient` impl
- All API functions become generic: `pub async fn fetch_lists(client: &impl HttpClient) -> Result<Vec<List>, ApiError>`
- `GlooClient` provided via `provide_context` in `App`; components retrieve with `use_context`
- `ApiError` as proper enum (Network, Http, Parse) instead of `String`

### Design notes
- `HttpClient` uses `async fn` in trait (stable since Rust 1.75). Used with `impl HttpClient` (static dispatch), not `dyn HttpClient`, since async trait methods are not object-safe without boxing. This is fine — WASM-only, no need for dynamic dispatch.
- Tests call API functions directly with a mock client (e.g. `fetch_lists(&mock_client)`), no Leptos context needed.

### Fixes included
- Consistent HTTP status checking in ALL methods (today only `patch_json` checks)
- Uniform error handling across all API functions

### Tests
- Mock-based tests for every API function
- Test: correct URL construction, response parsing, error paths
- Regular `cargo test`, no WASM required

## PR 4: State Transforms

Extract data transformation logic from components into pure functions.

### Design

```rust
// crates/frontend/src/state/transforms.rs

/// Pure functions — immutable input, new value output

pub fn with_item_toggled(items: &[Item], item_id: &str) -> (Vec<Item>, bool) { ... }
// returns (new list, new completed value) — caller needs the value for the API request
pub fn without_item(items: &[Item], item_id: &str) -> Vec<Item> { ... }
pub fn filter_items_by_tag(items: &[Item], tag_id: &str) -> Vec<Item> { ... }
pub fn group_items_by_date(items: &[DateItem]) -> BTreeMap<String, Vec<DateItem>> { ... }
pub fn sorted_by_position<T: HasPosition>(items: &[T]) -> Vec<T> { ... }
```

### Component pattern (after refactor)

```rust
let on_toggle = move |item_id: String| {
    let previous = items.get_untracked();
    items.set(with_item_toggled(&previous, &item_id));
    spawn_local(async move {
        if api::update_item(&client, &item_id, &body).await.is_err() {
            items.set(previous); // rollback = restore old snapshot
        }
    });
};
```

### Key points
- Pure functions: no Leptos, no signals, no async — just data in, data out
- Rollback is trivial: restore the previous snapshot (no need to "reverse" the operation)
- Components become thin glue: read signal → apply transform → set signal + spawn API call
- Rollback added everywhere (today only list-delete on HomePage has it)

### Tests
- Unit tests for all transform functions
- Test edge cases: empty lists, missing IDs, double-toggle idempotency

## Additional Tasks

- **Comment on issue #24** — document the steps taken toward the target architecture
- **Update CLAUDE.md** — remove `SendWrapper` notes, add `HttpClient` trait info, update `LocalResource` docs

## What This Does NOT Cover

- Renaming `shared` → `models` (deferred to #24, mechanical find-replace)
- Changes to `crates/api/` logic (only import paths may change via re-exports)
- SSR/hydration (staying CSR-only)
- New features or UI changes
