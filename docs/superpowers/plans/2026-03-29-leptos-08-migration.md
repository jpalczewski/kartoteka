# Leptos 0.8 Migration & Frontend Architecture Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade Leptos 0.7→0.8, extract testable business logic from components, deduplicate shared types, and abstract the HTTP layer behind a trait.

**Architecture:** Four sequential PRs (each independently shippable): (1) Leptos 0.8 bump, (2) shared crate expansion + date_utils migration, (3) HttpClient trait + API layer refactor, (4) state transforms extraction. Each PR builds on the previous.

**Tech Stack:** Leptos 0.8 CSR, gloo-net 0.7, serde, wasm-bindgen, Trunk

**Spec:** `docs/superpowers/specs/2026-03-29-leptos-08-migration-design.md`

---

## File Structure Overview

### PR 1 files (modify only)
- `crates/frontend/Cargo.toml` — bump leptos deps, remove `send_wrapper`
- `crates/frontend/src/components/nav.rs` — remove SendWrapper, fix deref
- `crates/frontend/src/pages/oauth_consent.rs` — remove SendWrapper, fix spawn_local
- `crates/frontend/src/pages/login.rs` — fix spawn_local
- `crates/frontend/src/pages/signup.rs` — fix spawn_local
- `crates/frontend/src/api/mod.rs` — fix spawn_local in logout()
- `crates/frontend/src/api/lists.rs` — fix missing credentials on reset_list

### PR 2 files (create + modify)
- Create: `crates/shared/src/models/mod.rs`
- Create: `crates/shared/src/models/container.rs`
- Create: `crates/shared/src/models/item.rs`
- Create: `crates/shared/src/models/list.rs`
- Create: `crates/shared/src/models/tag.rs`
- Create: `crates/shared/src/models/settings.rs`
- Create: `crates/shared/src/dto/mod.rs`
- Create: `crates/shared/src/dto/requests.rs`
- Create: `crates/shared/src/dto/responses.rs`
- Create: `crates/shared/src/deserializers.rs`
- Create: `crates/shared/src/constants.rs`
- Create: `crates/shared/src/date_utils.rs`
- Modify: `crates/shared/src/lib.rs` — replace contents with re-exports
- Modify: `crates/shared/src/tests/` — update imports
- Modify: `crates/frontend/src/components/common/date_utils.rs` — thin wrappers over shared

### PR 3 files (create + modify)
- Create: `crates/frontend/src/api/client.rs` — HttpClient trait + GlooClient
- Modify: `crates/frontend/src/api/mod.rs` — rewrite helpers to use HttpClient
- Modify: `crates/frontend/src/api/lists.rs` — use HttpClient
- Modify: `crates/frontend/src/api/items.rs` — use HttpClient
- Modify: `crates/frontend/src/api/containers.rs` — use HttpClient
- Modify: `crates/frontend/src/api/tags.rs` — use HttpClient
- Modify: `crates/frontend/src/api/settings.rs` — use HttpClient
- Modify: `crates/frontend/src/api/preferences.rs` — use HttpClient
- Modify: `crates/frontend/src/app.rs` — provide_context GlooClient
- Create: `crates/frontend/tests/api_tests.rs` — mock-based API tests

### PR 4 files (create + modify)
- Create: `crates/frontend/src/state/mod.rs`
- Create: `crates/frontend/src/state/transforms.rs`
- Create: `crates/frontend/src/state/transforms_test.rs`
- Modify: `crates/frontend/src/components/items/item_actions.rs` — use transforms
- Modify: `crates/frontend/src/pages/home.rs` — use transforms
- Modify: `crates/frontend/src/pages/today.rs` — use transforms
- Modify: various pages — apply snapshot+rollback pattern

---

## PR 1: Leptos 0.8 Bump

### Task 1.1: Update Cargo.toml dependencies

**Files:**
- Modify: `crates/frontend/Cargo.toml`

- [ ] **Step 1: Check leptos-fluent compatibility**

Run: `cargo search leptos-fluent --limit 5`

Check if `leptos-fluent` 0.2 supports Leptos 0.8 or if a bump is needed. Also check `leptos_meta`.

- [ ] **Step 2: Update dependency versions**

In `crates/frontend/Cargo.toml`, change:
```toml
# FROM:
leptos = { version = "0.7", features = ["csr"] }
leptos_router = "0.7"
leptos-fluent = "0.2"
leptos_meta = "0.7"
# ...
send_wrapper = "0.6"

# TO:
leptos = { version = "0.8", features = ["csr"] }
leptos_router = "0.8"
leptos-fluent = "0.2"       # or bumped version if needed
leptos_meta = "0.8"
# REMOVE send_wrapper line entirely
```

- [ ] **Step 3: Run cargo check to verify compilation**

Run: `cd /Users/erxyi/Projekty/kartoteka && cargo check -p kartoteka-frontend --target wasm32-unknown-unknown 2>&1 | head -50`

Expected: Compilation errors related to SendWrapper (we'll fix those next). If there are OTHER errors (version incompatibilities), resolve them first before proceeding.

- [ ] **Step 4: Commit dependency changes**

```bash
git add crates/frontend/Cargo.toml Cargo.lock
git commit -m "chore: bump leptos 0.7 -> 0.8, remove send_wrapper dep"
```

### Task 1.2: Remove SendWrapper from nav.rs

**Files:**
- Modify: `crates/frontend/src/components/nav.rs:3,11,29`

- [ ] **Step 1: Remove SendWrapper import and usage**

In `crates/frontend/src/components/nav.rs`:

Line 3: Remove `use send_wrapper::SendWrapper;`

Line 11: Change:
```rust
// FROM:
let session_res = LocalResource::new(|| async { SendWrapper::new(api::get_session().await) });
// TO:
let session_res = LocalResource::new(|| api::get_session());
```

Line 29: Change:
```rust
// FROM:
match &**s {
// TO:
match s.as_ref() {
```
Note: In Leptos 0.8, `LocalResource::get()` returns `Option<T>` directly. The `s` in the closure from `.map(|s| ...)` is the inner `T` (which is `Option<SessionInfo>`), so we use `s.as_ref()` to get `Option<&SessionInfo>` for pattern matching.

- [ ] **Step 2: Verify nav.rs compiles**

Run: `cargo check -p kartoteka-frontend --target wasm32-unknown-unknown 2>&1 | grep -E "error|warning" | head -20`

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/components/nav.rs
git commit -m "refactor: remove SendWrapper from Nav component (Leptos 0.8)"
```

### Task 1.3: Remove SendWrapper from oauth_consent.rs + fix spawn_local

**Files:**
- Modify: `crates/frontend/src/pages/oauth_consent.rs:3-4,29`

- [ ] **Step 1: Fix imports and SendWrapper usage**

In `crates/frontend/src/pages/oauth_consent.rs`:

Line 3: Remove `use send_wrapper::SendWrapper;`
Line 4: Change `use wasm_bindgen_futures::spawn_local;` to `use leptos::task::spawn_local;`

Line 29: Change:
```rust
// FROM:
let session = LocalResource::new(move || SendWrapper::new(api::get_session()));
// TO:
let session = LocalResource::new(move || api::get_session());
```

Also update any `&**s` or `&*s` dereference patterns on `session.get()` to direct access.

- [ ] **Step 2: Verify compiles**

Run: `cargo check -p kartoteka-frontend --target wasm32-unknown-unknown 2>&1 | grep "error" | head -10`

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/pages/oauth_consent.rs
git commit -m "refactor: remove SendWrapper from OAuthConsentPage, unify spawn_local"
```

### Task 1.4: Unify spawn_local in login.rs and signup.rs

**Files:**
- Modify: `crates/frontend/src/pages/login.rs:16`
- Modify: `crates/frontend/src/pages/signup.rs:17`

- [ ] **Step 1: Fix login.rs**

In `crates/frontend/src/pages/login.rs` line 16, change:
```rust
// FROM:
wasm_bindgen_futures::spawn_local(async move {
// TO:
leptos::task::spawn_local(async move {
```

- [ ] **Step 2: Fix signup.rs**

In `crates/frontend/src/pages/signup.rs` line 17, change:
```rust
// FROM:
wasm_bindgen_futures::spawn_local(async move {
// TO:
leptos::task::spawn_local(async move {
```

- [ ] **Step 3: Fix api/mod.rs logout()**

In `crates/frontend/src/api/mod.rs` line 187, change:
```rust
// FROM:
wasm_bindgen_futures::spawn_local(async {
// TO:
leptos::task::spawn_local(async {
```

- [ ] **Step 4: Verify no remaining wasm_bindgen_futures::spawn_local**

Run: `grep -r "wasm_bindgen_futures::spawn_local" crates/frontend/src/`

Expected: No matches.

- [ ] **Step 5: Commit**

```bash
git add crates/frontend/src/pages/login.rs crates/frontend/src/pages/signup.rs crates/frontend/src/api/mod.rs
git commit -m "refactor: unify spawn_local to leptos::task::spawn_local everywhere"
```

### Task 1.5: Fix reset_list missing credentials

**Files:**
- Modify: `crates/frontend/src/api/lists.rs:26-38`

- [ ] **Step 1: Add credentials to reset_list**

In `crates/frontend/src/api/lists.rs`, replace the `reset_list` function (lines 26-38):

```rust
pub async fn reset_list(id: &str) -> Result<(), String> {
    let json = serde_json::to_string(&serde_json::json!({})).map_err(|e| e.to_string())?;
    let resp = Request::post(&format!("{}/lists/{id}/reset", super::API_BASE))
        .headers(super::auth_headers())
        .credentials(web_sys::RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() >= 400 {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}
```

The only change is adding `.credentials(web_sys::RequestCredentials::Include)` after `.headers(...)`.

- [ ] **Step 2: Commit**

```bash
git add crates/frontend/src/api/lists.rs
git commit -m "fix: add missing credentials: include on reset_list (production auth bug)"
```

### Task 1.6: Final verification and cleanup

- [ ] **Step 1: Full workspace check**

Run: `cargo check --workspace --target wasm32-unknown-unknown 2>&1 | tail -5`

If frontend doesn't compile for workspace target, try:
Run: `cargo check -p kartoteka-frontend --target wasm32-unknown-unknown`

- [ ] **Step 2: Check wasm-bindgen-futures can be removed from deps**

Run: `grep -r "wasm_bindgen_futures" crates/frontend/src/`

If no remaining usages, remove `wasm-bindgen-futures = "0.4"` from `crates/frontend/Cargo.toml`.

- [ ] **Step 3: Run existing tests**

Run: `cargo test --workspace`

Expected: All existing tests pass (shared + i18n tests).

- [ ] **Step 4: Verify no send_wrapper references remain**

Run: `grep -r "send_wrapper\|SendWrapper" crates/frontend/src/`

Expected: No matches.

- [ ] **Step 5: Commit cleanup if any**

```bash
git add -A && git commit -m "chore: remove unused wasm-bindgen-futures dep"
```

---

## PR 2: Shared Crate Expansion

### Task 2.1: Create deserializers.rs and constants.rs

**Files:**
- Create: `crates/shared/src/deserializers.rs`
- Create: `crates/shared/src/constants.rs`

- [ ] **Step 1: Create deserializers.rs**

Extract from `crates/shared/src/lib.rs` lines 3-10 (`bool_from_number`), lines 131-139 (`features_from_json`), lines 392-398 (`u32_from_number`), and `default_config` (lines 275-277):

```rust
// crates/shared/src/deserializers.rs
use serde::{Deserialize, Deserializer};

pub(crate) fn bool_from_number<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
        _ => Ok(false),
    }
}

pub(crate) fn u32_from_number<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) as u32),
        _ => Ok(0),
    }
}

pub(crate) fn features_from_json<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<crate::models::list::ListFeature>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::String(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom),
        serde_json::Value::Array(_) => serde_json::from_value(v).map_err(serde::de::Error::custom),
        serde_json::Value::Null => Ok(vec![]),
        _ => Ok(vec![]),
    }
}

pub(crate) fn default_config() -> serde_json::Value {
    serde_json::json!({})
}
```

- [ ] **Step 2: Create constants.rs**

Extract from `lib.rs` lines 102-111:

```rust
// crates/shared/src/constants.rs
pub const FEATURE_QUANTITY: &str = "quantity";
pub const FEATURE_DEADLINES: &str = "deadlines";

pub const DATE_TYPE_START: &str = "start";
pub const DATE_TYPE_DEADLINE: &str = "deadline";
pub const DATE_TYPE_HARD_DEADLINE: &str = "hard_deadline";

pub const SETTING_MCP_AUTO_ENABLE_FEATURES: &str = "mcp_auto_enable_features";
```

- [ ] **Step 3: Commit**

```bash
git add crates/shared/src/deserializers.rs crates/shared/src/constants.rs
git commit -m "refactor: extract deserializers and constants from shared lib.rs"
```

### Task 2.2: Create models modules

**Files:**
- Create: `crates/shared/src/models/mod.rs`
- Create: `crates/shared/src/models/container.rs`
- Create: `crates/shared/src/models/item.rs`
- Create: `crates/shared/src/models/list.rs`
- Create: `crates/shared/src/models/tag.rs`
- Create: `crates/shared/src/models/settings.rs`

- [ ] **Step 1: Create models/container.rs**

Extract `ContainerStatus`, `Container`, `ContainerDetail` from `lib.rs` lines 14-50:

```rust
// crates/shared/src/models/container.rs
use serde::{Deserialize, Serialize};
use crate::deserializers::{bool_from_number, u32_from_number};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContainerStatus {
    Active,
    Done,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
    pub position: i32,
    #[serde(deserialize_with = "bool_from_number")]
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDetail {
    #[serde(flatten)]
    pub container: Container,
    #[serde(deserialize_with = "u32_from_number", default)]
    pub completed_items: u32,
    #[serde(deserialize_with = "u32_from_number", default)]
    pub total_items: u32,
    #[serde(deserialize_with = "u32_from_number", default)]
    pub completed_lists: u32,
    #[serde(deserialize_with = "u32_from_number", default)]
    pub total_lists: u32,
}
```

- [ ] **Step 2: Create models/list.rs**

Extract `ListFeature`, `ListType`, `DateField`, `List` from `lib.rs` lines 124-229. Include the `impl` blocks:

```rust
// crates/shared/src/models/list.rs
use serde::{Deserialize, Serialize};
use crate::constants::{FEATURE_QUANTITY, FEATURE_DEADLINES};
use crate::deserializers::{bool_from_number, features_from_json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListFeature {
    pub name: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    Checklist,
    Zakupy,
    Pakowanie,
    Terminarz,
    Custom,
}

impl ListType {
    pub fn default_features(&self) -> Vec<ListFeature> {
        match self {
            Self::Zakupy | Self::Pakowanie => vec![ListFeature {
                name: FEATURE_QUANTITY.into(),
                config: serde_json::json!({"unit_default": "szt"}),
            }],
            Self::Terminarz => vec![ListFeature {
                name: FEATURE_DEADLINES.into(),
                config: serde_json::json!({"has_start_date": false, "has_deadline": true, "has_hard_deadline": false}),
            }],
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DateField {
    StartDate,
    Deadline,
    HardDeadline,
}

impl DateField {
    pub fn column_name(&self) -> &'static str {
        match self {
            Self::StartDate => "start_date",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }

    pub fn time_column_name(&self) -> Option<&'static str> {
        match self {
            Self::StartDate => Some("start_time"),
            Self::Deadline => Some("deadline_time"),
            Self::HardDeadline => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::StartDate => "start",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub list_type: ListType,
    pub parent_list_id: Option<String>,
    pub position: i32,
    #[serde(deserialize_with = "bool_from_number")]
    pub archived: bool,
    #[serde(default, deserialize_with = "features_from_json")]
    pub features: Vec<ListFeature>,
    #[serde(default)]
    pub container_id: Option<String>,
    #[serde(default, deserialize_with = "bool_from_number")]
    pub pinned: bool,
    #[serde(default)]
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl List {
    pub fn has_feature(&self, name: &str) -> bool {
        self.features.iter().any(|f| f.name == name)
    }
}
```

- [ ] **Step 3: Create models/item.rs**

Extract `Item`, `DateItem`, `DaySummary`, `DayItems`, and `From<DateItem> for Item` from `lib.rs`:

```rust
// crates/shared/src/models/item.rs
use serde::{Deserialize, Serialize};
use crate::deserializers::{bool_from_number, u32_from_number};
use super::list::ListType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "bool_from_number")]
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateItem {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "bool_from_number")]
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub list_name: String,
    pub list_type: ListType,
    #[serde(default)]
    pub date_type: Option<String>,
}

impl From<DateItem> for Item {
    fn from(di: DateItem) -> Self {
        Item {
            id: di.id, list_id: di.list_id, title: di.title,
            description: di.description, completed: di.completed,
            position: di.position, quantity: di.quantity,
            actual_quantity: di.actual_quantity, unit: di.unit,
            start_date: di.start_date, start_time: di.start_time,
            deadline: di.deadline, deadline_time: di.deadline_time,
            hard_deadline: di.hard_deadline, created_at: di.created_at,
            updated_at: di.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummary {
    pub date: String,
    #[serde(deserialize_with = "u32_from_number")]
    pub total: u32,
    #[serde(deserialize_with = "u32_from_number")]
    pub completed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayItems {
    pub date: String,
    pub items: Vec<DateItem>,
}
```

- [ ] **Step 4: Create models/tag.rs**

```rust
// crates/shared/src/models/tag.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub parent_tag_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTagLink {
    pub item_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTagLink {
    pub list_id: String,
    pub tag_id: String,
}
```

- [ ] **Step 5: Create models/settings.rs**

```rust
// crates/shared/src/models/settings.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSetting {
    pub key: String,
    pub value: serde_json::Value,
}
```

- [ ] **Step 6: Create models/mod.rs**

```rust
// crates/shared/src/models/mod.rs
pub mod container;
pub mod item;
pub mod list;
pub mod settings;
pub mod tag;

pub use container::*;
pub use item::*;
pub use list::*;
pub use settings::*;
pub use tag::*;
```

- [ ] **Step 7: Commit**

```bash
git add crates/shared/src/models/
git commit -m "refactor: extract shared models into separate modules"
```

### Task 2.3: Create dto modules

**Files:**
- Create: `crates/shared/src/dto/mod.rs`
- Create: `crates/shared/src/dto/requests.rs`
- Create: `crates/shared/src/dto/responses.rs`

- [ ] **Step 1: Create dto/requests.rs**

Extract all request DTOs from `lib.rs`:

```rust
// crates/shared/src/dto/requests.rs
use serde::{Deserialize, Serialize};
use crate::models::{ContainerStatus, ListType, ListFeature};
use crate::deserializers::default_config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub status: Option<ContainerStatus>,
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContainerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<Option<ContainerStatus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveListRequest {
    pub container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveContainerRequest {
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: ListType,
    pub features: Option<Vec<ListFeature>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub list_type: Option<ListType>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfigRequest {
    #[serde(default = "default_config")]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateItemRequest {
    pub title: String,
    pub description: Option<String>,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateItemRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
    pub position: Option<i32>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<Option<String>>,
    pub start_time: Option<Option<String>>,
    pub deadline: Option<Option<String>>,
    pub deadline_time: Option<Option<String>>,
    pub hard_deadline: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub color: String,
    pub parent_tag_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<Option<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergeTagRequest {
    pub target_tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagAssignment {
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertSettingRequest {
    pub value: serde_json::Value,
}
```

- [ ] **Step 2: Create dto/responses.rs**

New types + moved types:

```rust
// crates/shared/src/dto/responses.rs
use serde::{Deserialize, Serialize};
use crate::models::{Item, ListFeature, Container, List};

/// Response from GET /api/lists/:list_id/items/:id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDetailResponse {
    #[serde(flatten)]
    pub item: Item,
    pub list_name: String,
    pub list_features: Vec<ListFeature>,
}

/// Response from GET /api/containers/:id/children
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerChildrenResponse {
    pub containers: Vec<Container>,
    pub lists: Vec<List>,
}

/// Response from GET /api/preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesResponse {
    pub locale: String,
}

/// Request body for PUT /api/preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePreferencesBody {
    pub locale: String,
}

/// Error response body returned by API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    #[serde(default)]
    pub status: u16,
}

/// Response from GET /api/home — matches actual API shape
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeData {
    #[serde(default)]
    pub pinned_lists: Vec<List>,
    #[serde(default)]
    pub pinned_containers: Vec<Container>,
    #[serde(default)]
    pub recent_lists: Vec<List>,
    #[serde(default)]
    pub recent_containers: Vec<Container>,
    #[serde(default)]
    pub root_containers: Vec<Container>,
    #[serde(default)]
    pub root_lists: Vec<List>,
}
```

Note: Check the actual API handler at `crates/api/src/handlers/containers.rs` to verify the exact field names before implementing. The `HomeItem` type can be removed (it was dead code — the API never returned that shape).

- [ ] **Step 3: Create dto/mod.rs**

```rust
// crates/shared/src/dto/mod.rs
pub mod requests;
pub mod responses;

pub use requests::*;
pub use responses::*;
```

- [ ] **Step 4: Commit**

```bash
git add crates/shared/src/dto/
git commit -m "refactor: extract shared DTOs into requests/responses modules"
```

### Task 2.4: Rewrite lib.rs as re-exports

**Files:**
- Modify: `crates/shared/src/lib.rs`

- [ ] **Step 1: Replace lib.rs contents**

```rust
// crates/shared/src/lib.rs

pub mod constants;
pub(crate) mod deserializers;
pub mod dto;
pub mod models;

// Flat re-exports for backward compatibility (no churn in crates/api imports)
pub use constants::*;
pub use dto::*;
pub use models::*;

#[cfg(test)]
mod tests;
```

- [ ] **Step 2: Verify workspace compiles**

Run: `cargo check --workspace 2>&1 | tail -10`

Fix any import issues. The API crate uses `use kartoteka_shared::*;` which should still work via the re-exports.

- [ ] **Step 3: Run existing tests**

Run: `cargo test --workspace`

Expected: All existing tests pass. Some test files may need import path updates if they reference internal types directly.

- [ ] **Step 4: Update test files if needed**

The tests in `crates/shared/src/tests/` use types like `Item`, `List`, etc. Since we re-export everything from `lib.rs`, `use crate::*` should still work. Verify and fix if needed.

- [ ] **Step 5: Commit**

```bash
git add crates/shared/src/lib.rs
git commit -m "refactor: rewrite shared lib.rs as module re-exports"
```

### Task 2.5: Migrate date_utils to shared

**Files:**
- Create: `crates/shared/src/date_utils.rs`
- Modify: `crates/frontend/src/components/common/date_utils.rs`

- [ ] **Step 1: Write failing tests for date_utils in shared**

Create test file first. Key functions to test: `get_today_string`, `parse_date`, `days_in_month`, `date_to_days`, `relative_date`, `is_overdue`, `day_of_week`, `add_days`, `week_range`, `month_grid_range`.

Add tests in `crates/shared/src/date_utils.rs` (inline `#[cfg(test)] mod tests`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_today_string() {
        assert_eq!(get_today_string(2026, 3, 29), "2026-03-29");
        assert_eq!(get_today_string(2026, 1, 5), "2026-01-05");
    }

    #[test]
    fn test_current_time_hhmm() {
        assert_eq!(current_time_hhmm(14, 5), "14:05");
        assert_eq!(current_time_hhmm(0, 0), "00:00");
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(parse_date("2026-03-29"), Some((2026, 3, 29)));
        assert_eq!(parse_date("invalid"), None);
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29); // leap year
        assert_eq!(days_in_month(2026, 1), 31);
    }

    #[test]
    fn test_is_overdue_deadline_past() {
        let item = crate::Item {
            id: "1".into(), list_id: "l1".into(), title: "t".into(),
            description: None, completed: false, position: 0,
            quantity: None, actual_quantity: None, unit: None,
            start_date: None, start_time: None,
            deadline: Some("2026-03-28".into()), deadline_time: None,
            hard_deadline: None,
            created_at: "2026-03-01".into(), updated_at: "2026-03-01".into(),
        };
        assert!(is_overdue(&item, "2026-03-29", "12:00"));
    }

    #[test]
    fn test_is_overdue_completed_not_overdue() {
        let item = crate::Item {
            id: "1".into(), list_id: "l1".into(), title: "t".into(),
            description: None, completed: true, position: 0,
            quantity: None, actual_quantity: None, unit: None,
            start_date: None, start_time: None,
            deadline: Some("2026-03-28".into()), deadline_time: None,
            hard_deadline: None,
            created_at: "2026-03-01".into(), updated_at: "2026-03-01".into(),
        };
        assert!(!is_overdue(&item, "2026-03-29", "12:00"));
    }

    #[test]
    fn test_add_days() {
        assert_eq!(add_days("2026-03-29", 1), "2026-03-30");
        assert_eq!(add_days("2026-03-31", 1), "2026-04-01");
        assert_eq!(add_days("2026-01-01", -1), "2025-12-31");
    }

    #[test]
    fn test_day_of_week() {
        // 2026-03-29 is a Sunday = 0
        assert_eq!(day_of_week("2026-03-29"), 0);
    }

    #[test]
    fn test_relative_date() {
        assert_eq!(relative_date("2026-03-29", "2026-03-29"), "today");
        assert_eq!(relative_date("2026-03-30", "2026-03-29"), "tomorrow");
        assert_eq!(relative_date("2026-03-28", "2026-03-29"), "yesterday");
    }

    #[test]
    fn test_week_range() {
        let (start, end) = week_range("2026-03-29"); // Sunday
        // Week should be Mon-Sun
        assert_eq!(start, "2026-03-23");
        assert_eq!(end, "2026-03-29");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p kartoteka-shared -- date_utils 2>&1 | head -20`

Expected: Compilation error (module doesn't exist yet).

- [ ] **Step 3: Create shared date_utils.rs**

Copy all pure functions from `crates/frontend/src/components/common/date_utils.rs` to `crates/shared/src/date_utils.rs`. Modify:

- `get_today_string()` → `get_today_string(year: u32, month: u32, day: u32) -> String`
- `current_time_hhmm()` → `current_time_hhmm(hours: u32, minutes: u32) -> String`
- `is_overdue(item, today)` → `is_overdue(item, today, now_time)` (add `now_time: &str` param, pass to `is_overdue_for_date_type`)
- `is_overdue_for_date_type(...)` → add `now_time: &str` param instead of calling `current_time_hhmm()`
- `get_today()` — remove (it was just a wrapper around `get_today_string()`)

**Do NOT copy** Polish formatting functions (`polish_month_abbr`, `polish_day_of_week`, `polish_day_of_week_full`, `polish_month_name`, `format_polish_date`). Those stay in the frontend.

Also do NOT copy: `DateBadge` struct and `item_date_badges` — these are presentation logic (badge colors, labels), stay in frontend.

Import `crate::Item` for the `is_overdue`/`is_upcoming` signatures.

Add `pub mod date_utils;` to `lib.rs` and `pub use date_utils::*;` for re-export.

- [ ] **Step 4: Run tests**

Run: `cargo test -p kartoteka-shared -- date_utils -v`

Expected: All tests pass.

- [ ] **Step 5: Update frontend date_utils.rs to use shared**

In `crates/frontend/src/components/common/date_utils.rs`:
- Remove all functions that were moved to shared
- Add thin wrappers for `js_sys` functions:

```rust
use kartoteka_shared::date_utils as shared_dates;

pub fn get_today_string() -> String {
    let d = js_sys::Date::new_0();
    shared_dates::get_today_string(
        d.get_full_year() as u32,
        d.get_month() as u32 + 1,
        d.get_date() as u32,
    )
}

pub fn current_time_hhmm() -> String {
    let d = js_sys::Date::new_0();
    shared_dates::current_time_hhmm(d.get_hours() as u32, d.get_minutes() as u32)
}

// Re-export everything from shared for backward compatibility
pub use kartoteka_shared::date_utils::*;

// Polish formatting functions stay here (presentation logic)
pub fn polish_month_abbr(month: u32) -> &'static str { /* stays */ }
// ... etc
```

- [ ] **Step 6: Verify frontend compiles**

Run: `cargo check -p kartoteka-frontend --target wasm32-unknown-unknown 2>&1 | tail -10`

- [ ] **Step 7: Run all tests**

Run: `cargo test --workspace`

- [ ] **Step 8: Commit**

```bash
git add crates/shared/src/date_utils.rs crates/shared/src/lib.rs crates/frontend/src/components/common/date_utils.rs
git commit -m "feat: migrate date_utils business logic to shared crate with full test coverage"
```

### Task 2.6: Update frontend to use shared response types

**Files:**
- Modify: `crates/frontend/src/api/containers.rs`
- Modify: `crates/frontend/src/api/preferences.rs`
- Modify: `crates/frontend/src/pages/home.rs`

- [ ] **Step 1: Update containers.rs to use HomeData**

In `crates/frontend/src/api/containers.rs`, change `fetch_home()` to return `HomeData` instead of `serde_json::Value`:

```rust
pub async fn fetch_home() -> Result<HomeData, String> {
    super::get(&format!("{}/home", super::API_BASE))
        .send().await.map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())
}
```

Similarly update `fetch_container_children()` to return `ContainerChildrenResponse`.

- [ ] **Step 2: Update home.rs to use typed HomeData**

In `crates/frontend/src/pages/home.rs`, replace all `serde_json::Value` parsing (like `v.get("pinned_lists").and_then(...)`) with direct field access on `HomeData`:

```rust
// FROM:
if let Some(pl) = v.get("pinned_lists").and_then(|x| serde_json::from_value::<Vec<List>>(x.clone()).ok()) {
    pinned_lists.set(pl);
}
// TO:
pinned_lists.set(data.pinned_lists);
```

- [ ] **Step 3: Update preferences.rs to use shared types**

In `crates/frontend/src/api/preferences.rs`:
- Remove local `PreferencesResponse` and `UpdatePreferencesBody` structs
- Import from shared: `use kartoteka_shared::{PreferencesResponse, UpdatePreferencesBody};`

- [ ] **Step 4: Update API error.rs to use shared ErrorResponse**

In `crates/api/src/error.rs`, import `ErrorResponse` from shared instead of defining it locally. The `json_error` function stays in the API crate (it depends on `worker::Response`).

Note: If the API crate's `ErrorResponse` only has `Serialize` but the shared one also needs `Deserialize` (for frontend), make sure the shared type derives both.

- [ ] **Step 5: Verify everything compiles and tests pass**

Run: `cargo check --workspace && cargo test --workspace`

- [ ] **Step 6: Commit**

```bash
git add crates/frontend/src/api/ crates/frontend/src/pages/home.rs crates/api/src/error.rs
git commit -m "refactor: use shared response types, fix HomeData to match actual API shape"
```

---

## PR 3: HttpClient Trait & API Layer

### Task 3.1: Define HttpClient trait and GlooClient

**Files:**
- Create: `crates/frontend/src/api/client.rs`

- [ ] **Step 1: Write the trait and types**

```rust
// crates/frontend/src/api/client.rs
use std::future::Future;

/// HTTP method enum (avoiding dependency on external crate for this)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Simplified HTTP response for API layer
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// Abstract HTTP client trait. Used with static dispatch (`impl HttpClient`).
pub trait HttpClient {
    fn request(
        &self,
        method: Method,
        url: &str,
        body: Option<&str>,
    ) -> impl Future<Output = Result<HttpResponse, String>>;
}

/// Production HTTP client using gloo-net. Sends credentials with every request.
pub struct GlooClient;

impl HttpClient for GlooClient {
    async fn request(
        &self,
        method: Method,
        url: &str,
        body: Option<&str>,
    ) -> Result<HttpResponse, String> {
        use gloo_net::http::Request;

        let headers = super::auth_headers();

        let builder = match method {
            Method::Get => Request::get(url),
            Method::Post => Request::post(url),
            Method::Put => Request::put(url),
            Method::Patch => Request::patch(url),
            Method::Delete => Request::delete(url),
        };

        let builder = builder
            .headers(headers)
            .credentials(web_sys::RequestCredentials::Include);

        let builder = if let Some(b) = body {
            builder.body(b).map_err(|e| e.to_string())?
        } else {
            builder
        };

        let resp = builder.send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| e.to_string())?;

        Ok(HttpResponse { status, body })
    }
}
```

- [ ] **Step 2: Add module to api/mod.rs**

Add `pub mod client;` to `crates/frontend/src/api/mod.rs`.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/api/client.rs crates/frontend/src/api/mod.rs
git commit -m "feat: add HttpClient trait and GlooClient implementation"
```

### Task 3.2: Rewrite API helpers to use HttpClient

**Files:**
- Modify: `crates/frontend/src/api/mod.rs`

- [ ] **Step 1: Add generic helper functions**

Add to `crates/frontend/src/api/mod.rs`:

```rust
use client::{HttpClient, HttpResponse, Method};

/// Parse API response, checking status and deserializing body
pub(crate) fn parse_response<T: serde::de::DeserializeOwned>(
    resp: &HttpResponse,
) -> Result<T, ApiError> {
    if resp.status >= 400 {
        let code = serde_json::from_str::<kartoteka_shared::ErrorResponse>(&resp.body)
            .ok()
            .map(|e| e.code);
        return Err(ApiError::Http {
            status: resp.status,
            code,
        });
    }
    serde_json::from_str(&resp.body).map_err(|e| ApiError::Parse(e.to_string()))
}

/// GET request with deserialized response
pub(crate) async fn api_get<T: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    url: &str,
) -> Result<T, ApiError> {
    let resp = client.request(Method::Get, url, None).await.map_err(|e| ApiError::Network(e))?;
    parse_response(&resp)
}

/// POST with JSON body and deserialized response
pub(crate) async fn api_post<T: serde::de::DeserializeOwned>(
    client: &impl HttpClient,
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, ApiError> {
    let json = serde_json::to_string(body).map_err(|e| ApiError::Parse(e.to_string()))?;
    let resp = client.request(Method::Post, url, Some(&json)).await.map_err(|e| ApiError::Network(e))?;
    parse_response(&resp)
}

// Similar for api_put, api_patch, api_delete...
```

- [ ] **Step 2: Keep old helpers temporarily for backward compatibility**

Don't remove the old `get()`, `post_json()`, etc. yet. The per-domain modules will be migrated one by one in the next tasks.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/api/mod.rs
git commit -m "feat: add generic API helpers using HttpClient trait"
```

### Task 3.3: Migrate per-domain API modules

**Files:**
- Modify: `crates/frontend/src/api/lists.rs`
- Modify: `crates/frontend/src/api/items.rs`
- Modify: `crates/frontend/src/api/containers.rs`
- Modify: `crates/frontend/src/api/tags.rs`
- Modify: `crates/frontend/src/api/settings.rs`
- Modify: `crates/frontend/src/api/preferences.rs`

- [ ] **Step 1: Migrate lists.rs**

Change all functions to accept `client: &impl HttpClient`. Example:

```rust
// FROM:
pub async fn create_list(req: &CreateListRequest) -> Result<List, String> {
    super::post_json(&format!("{}/lists", super::API_BASE), req).await
}

// TO:
pub async fn create_list(client: &impl super::client::HttpClient, req: &CreateListRequest) -> Result<List, super::ApiError> {
    super::api_post(client, &format!("{}/lists", super::API_BASE), req).await
}
```

Do this for every function in lists.rs, items.rs, containers.rs, tags.rs, settings.rs, preferences.rs.

- [ ] **Step 2: Migrate each module one at a time**

For each module, update all functions, verify compilation:
Run: `cargo check -p kartoteka-frontend --target wasm32-unknown-unknown 2>&1 | head -30`

This will show errors in components that call these functions without passing `client`. Don't fix those yet — we'll update call sites next.

- [ ] **Step 3: Commit each module separately**

```bash
git add crates/frontend/src/api/lists.rs && git commit -m "refactor: migrate lists API to HttpClient"
git add crates/frontend/src/api/items.rs && git commit -m "refactor: migrate items API to HttpClient"
# ... etc
```

### Task 3.4: Provide GlooClient via context and update call sites

**Files:**
- Modify: `crates/frontend/src/app.rs`
- Modify: all pages and components that call API functions

- [ ] **Step 1: Provide GlooClient in App**

In `crates/frontend/src/app.rs`, add:

```rust
use crate::api::client::GlooClient;

// Inside App component, before the view:
provide_context(GlooClient);
```

- [ ] **Step 2: Update components to get client from context**

In each component/page that calls API functions, add:

```rust
let client = use_context::<GlooClient>().expect("GlooClient not provided");
```

Then pass `&client` to all API calls:

```rust
// FROM:
let lists = api::fetch_lists().await;
// TO:
let lists = api::fetch_lists(&client).await;
```

This is a mechanical change across many files. Work through them systematically:
1. Pages: home.rs, list/mod.rs, container.rs, today.rs, item_detail.rs, settings.rs, tags/mod.rs, tags/detail.rs, calendar/day.rs
2. Components: item_actions.rs, sublist_section.rs, tag_tree.rs, sync_locale.rs, editable_title.rs, editable_description.rs, confirm_delete_modal.rs

- [ ] **Step 3: Update error handling at call sites**

Functions now return `ApiError` instead of `String`. Update `.map_err(...)` and error display patterns. In most places, use `ApiError::to_i18n_key()` for toast messages or `.to_string()` for debug display.

- [ ] **Step 4: Remove old HTTP helpers**

Once all call sites are migrated, remove the old `get()`, `del()`, `post_json()`, `put_json()`, `patch_json()` functions from `api/mod.rs`. Also remove the `ErrorBody` struct and direct `gloo-net` imports that are no longer needed at module level.

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p kartoteka-frontend --target wasm32-unknown-unknown`

Expected: Clean compilation.

- [ ] **Step 6: Run all tests**

Run: `cargo test --workspace`

- [ ] **Step 7: Commit**

```bash
git add crates/frontend/src/
git commit -m "refactor: wire GlooClient through all components, remove old HTTP helpers"
```

### Task 3.5: Write mock-based API tests

**Files:**
- Create: `crates/frontend/tests/api_tests.rs` (or inline in each api module with `#[cfg(test)]`)

- [ ] **Step 1: Create mock HttpClient**

```rust
// In test module
struct MockClient {
    response: HttpResponse,
}

impl MockClient {
    fn ok(body: &str) -> Self {
        MockClient {
            response: HttpResponse { status: 200, body: body.to_string() },
        }
    }
    fn error(status: u16, code: &str) -> Self {
        MockClient {
            response: HttpResponse {
                status,
                body: serde_json::json!({"code": code, "status": status}).to_string(),
            },
        }
    }
}

impl HttpClient for MockClient {
    async fn request(&self, _method: Method, _url: &str, _body: Option<&str>) -> Result<HttpResponse, String> {
        Ok(self.response.clone())
    }
}
```

Note: This is a simple mock. For tests that need to verify the URL or method, add fields to capture those:

```rust
struct MockClient {
    response: HttpResponse,
    captured_method: std::cell::RefCell<Option<Method>>,
    captured_url: std::cell::RefCell<Option<String>>,
}
```

- [ ] **Step 2: Write tests for key API functions**

```rust
#[tokio::test]
async fn test_fetch_lists_success() {
    let lists = vec![/* test List data */];
    let client = MockClient::ok(&serde_json::to_string(&lists).unwrap());
    let result = api::fetch_lists(&client).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), lists.len());
}

#[tokio::test]
async fn test_fetch_lists_http_error() {
    let client = MockClient::error(404, "list_not_found");
    let result = api::fetch_lists(&client).await;
    assert!(matches!(result, Err(ApiError::Http { status: 404, .. })));
}

#[tokio::test]
async fn test_create_item_sends_correct_body() {
    let client = MockClient::ok_capturing(/* ... */);
    let req = CreateItemRequest { title: "Test".into(), ..Default::default() };
    let _ = api::create_item(&client, "list-1", &req).await;
    let captured = client.captured_body();
    assert!(captured.contains("Test"));
}
```

Note: These tests run as regular `cargo test` — no WASM target needed, because `MockClient` doesn't use `gloo-net`. The `#[cfg(test)]` modules should use `#[tokio::test]` for async test support. Add `tokio = { version = "1", features = ["macros", "rt"] }` as a dev-dependency.

- [ ] **Step 3: Run tests**

Run: `cargo test -p kartoteka-frontend -- api 2>&1`

Expected: All API tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/frontend/
git commit -m "test: add mock-based API tests for all endpoint functions"
```

---

## PR 4: State Transforms

### Task 4.1: Create transforms module with tests

**Files:**
- Create: `crates/frontend/src/state/mod.rs`
- Create: `crates/frontend/src/state/transforms.rs`

- [ ] **Step 1: Write failing tests**

```rust
// crates/frontend/src/state/transforms.rs

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_shared::Item;

    fn make_item(id: &str, completed: bool) -> Item {
        Item {
            id: id.to_string(),
            list_id: "list-1".to_string(),
            title: format!("Item {id}"),
            description: None,
            completed,
            position: 0,
            quantity: None,
            actual_quantity: None,
            unit: None,
            start_date: None,
            start_time: None,
            deadline: None,
            deadline_time: None,
            hard_deadline: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn test_toggle_item() {
        let items = vec![make_item("1", false), make_item("2", true)];
        let (result, new_val) = with_item_toggled(&items, "1");
        assert!(new_val); // was false, now true
        assert!(result[0].completed);
        assert!(result[1].completed); // unchanged
    }

    #[test]
    fn test_toggle_item_idempotent_double() {
        let items = vec![make_item("1", false)];
        let (toggled, _) = with_item_toggled(&items, "1");
        let (back, _) = with_item_toggled(&toggled, "1");
        assert_eq!(back[0].completed, items[0].completed);
    }

    #[test]
    fn test_toggle_missing_id() {
        let items = vec![make_item("1", false)];
        let (result, new_val) = with_item_toggled(&items, "nonexistent");
        assert!(!new_val); // default false
        assert_eq!(result[0].completed, false); // unchanged
    }

    #[test]
    fn test_without_item() {
        let items = vec![make_item("1", false), make_item("2", false)];
        let result = without_item(&items, "1");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "2");
    }

    #[test]
    fn test_without_item_missing_id() {
        let items = vec![make_item("1", false)];
        let result = without_item(&items, "nonexistent");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_without_item_empty() {
        let result = without_item(&[], "1");
        assert!(result.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p kartoteka-frontend -- transforms 2>&1 | head -10`

Expected: Compilation error (functions not implemented yet).

- [ ] **Step 3: Implement transform functions**

```rust
// crates/frontend/src/state/transforms.rs
use kartoteka_shared::Item;

/// Toggle an item's completed status. Returns (new list, new completed value).
/// If item_id not found, returns unchanged list and false.
pub fn with_item_toggled(items: &[Item], item_id: &str) -> (Vec<Item>, bool) {
    let mut new_completed = false;
    let result = items
        .iter()
        .map(|item| {
            if item.id == item_id {
                let toggled = !item.completed;
                new_completed = toggled;
                Item {
                    completed: toggled,
                    ..item.clone()
                }
            } else {
                item.clone()
            }
        })
        .collect();
    (result, new_completed)
}

/// Remove an item by ID. Returns new list without the item.
pub fn without_item(items: &[Item], item_id: &str) -> Vec<Item> {
    items.iter().filter(|i| i.id != item_id).cloned().collect()
}
```

- [ ] **Step 4: Create state/mod.rs**

```rust
// crates/frontend/src/state/mod.rs
pub mod transforms;
```

Add `pub mod state;` to `crates/frontend/src/main.rs` or wherever the module tree is rooted.

- [ ] **Step 5: Run tests**

Run: `cargo test -p kartoteka-frontend -- transforms -v`

Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/frontend/src/state/
git commit -m "feat: add state transform functions with tests (toggle, delete)"
```

### Task 4.2: Apply transforms to item_actions.rs

**Files:**
- Modify: `crates/frontend/src/components/items/item_actions.rs`

- [ ] **Step 1: Refactor on_toggle to use with_item_toggled**

In item_actions.rs, find the `on_toggle` callback and replace the inline mutation with:

```rust
let on_toggle = Callback::new(move |item_id: String| {
    let previous = items_signal.get_untracked();
    let (new_items, new_completed) = crate::state::transforms::with_item_toggled(&previous, &item_id);
    items_signal.set(new_items);
    let client = client.clone();
    leptos::task::spawn_local(async move {
        let body = UpdateItemRequest {
            completed: Some(new_completed),
            ..Default::default()
        };
        if api::update_item(&client, &list_id, &item_id, &body).await.is_err() {
            items_signal.set(previous); // rollback
        }
    });
});
```

- [ ] **Step 2: Refactor on_delete to use without_item**

```rust
let on_delete = Callback::new(move |item_id: String| {
    let previous = items_signal.get_untracked();
    items_signal.set(crate::state::transforms::without_item(&previous, &item_id));
    let client = client.clone();
    leptos::task::spawn_local(async move {
        if api::delete_item(&client, &list_id, &item_id).await.is_err() {
            items_signal.set(previous); // rollback
        }
    });
});
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p kartoteka-frontend --target wasm32-unknown-unknown 2>&1 | tail -10`

- [ ] **Step 4: Commit**

```bash
git add crates/frontend/src/components/items/item_actions.rs
git commit -m "refactor: use transform functions in item_actions with rollback"
```

### Task 4.3: Apply snapshot+rollback to remaining pages

**Files:**
- Modify: `crates/frontend/src/pages/home.rs`
- Modify: `crates/frontend/src/pages/today.rs`
- Modify: `crates/frontend/src/pages/tags/detail.rs`

- [ ] **Step 1: Update home.rs**

Find all inline optimistic mutations (toggle, delete) and apply the same pattern:
```rust
let previous = signal.get_untracked();
signal.set(transform(&previous, &id));
spawn_local(async move {
    if api_call.await.is_err() { signal.set(previous); }
});
```

- [ ] **Step 2: Update today.rs**

Same pattern for toggle/delete operations.

- [ ] **Step 3: Update tags/detail.rs**

Same pattern for tag-related mutations.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p kartoteka-frontend --target wasm32-unknown-unknown`

- [ ] **Step 5: Run all tests**

Run: `cargo test --workspace`

- [ ] **Step 6: Commit**

```bash
git add crates/frontend/src/pages/
git commit -m "refactor: apply snapshot+rollback pattern across all pages"
```

### Task 4.4: Add more transform functions as needed

**Files:**
- Modify: `crates/frontend/src/state/transforms.rs`

- [ ] **Step 1: Identify additional transforms needed**

Review all pages for inline data transformations (filtering, sorting, grouping). Common patterns to extract:

- `filter_items_by_tag` — if used in tag detail pages
- `group_items_by_date` — if used in today/calendar pages
- `sorted_by_position` — if used in list views

Only extract transforms that are actually used in multiple places or that have non-trivial logic worth testing.

- [ ] **Step 2: Write tests first, then implement**

Follow TDD: write test → verify fail → implement → verify pass.

- [ ] **Step 3: Commit**

```bash
git add crates/frontend/src/state/transforms.rs
git commit -m "feat: add additional transform functions (filter, sort, group)"
```

---

## Final Tasks

### Task 5.1: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update relevant sections**

Changes:
- Remove `SendWrapper` / `&*result` notes under Frontend section
- Update `LocalResource` notes: "Leptos 0.8 — `LocalResource::get()` returns `Option<T>` directly"
- Add note about `HttpClient` trait pattern
- Add note about `state/transforms.rs` for pure transform functions
- Update `gloo-net` version reference if changed

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for Leptos 0.8 and new architecture patterns"
```

### Task 5.2: Comment on issue #24

- [ ] **Step 1: Add comment documenting steps taken**

```bash
gh issue comment 24 --repo jpalczewski/kartoteka --body "Steps taken toward target architecture (from Leptos 0.8 migration):

1. **Shared crate expanded** — split into models/, dto/, deserializers, constants, date_utils modules. Re-exports maintain backward compat.
2. **Type deduplication** — PreferencesResponse, ErrorResponse, ContainerChildrenResponse, HomeData now in shared.
3. **HttpClient trait** — frontend HTTP layer abstracted behind trait. GlooClient for WASM, MockClient for tests.
4. **date_utils migrated** — pure date math in shared (testable without WASM), js_sys wrappers in frontend.
5. **State transforms** — pure functions for data mutations, snapshot+rollback pattern for optimistic updates.

Remaining for #24: rename shared→models, replace gloo-net with reqwest (behind HttpClient trait), add Axum server crate."
```

- [ ] **Step 2: Commit any remaining changes**

Run: `cargo test --workspace` one final time to ensure everything is clean.
