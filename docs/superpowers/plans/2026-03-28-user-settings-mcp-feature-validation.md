# User Settings + MCP Feature Validation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a generic `user_settings` key-value store, use it to drive a `mcp_auto_enable_features` preference, and make the API reject feature-gated item fields when the list doesn't have the required feature enabled.

**Architecture:** DB migration adds `user_settings(user_id, key, value TEXT)`. API gets new `/api/settings` CRUD endpoints and feature validation in `create_item`/`update_item` (422 when feature-gated fields used without the feature). MCP removes silent `ensureFeatures`, adds explicit `enable_list_feature`/`disable_list_feature` tools, and handles 422s with on-demand auto-enable when `mcp_auto_enable_features = true`.

**Tech Stack:** Rust (worker 0.7, sqlx-d1, serde_json), TypeScript (Zod, MCP SDK), Leptos 0.7 (WASM frontend), Cloudflare D1 (SQLite)

**Spec:** `docs/superpowers/specs/2026-03-28-user-settings-mcp-feature-validation-design.md`

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `crates/api/migrations/0011_user_settings.sql` | DB schema for user_settings |
| Modify | `crates/shared/src/lib.rs` | `UserSetting`, `UpsertSettingRequest`, `SETTING_MCP_AUTO_ENABLE_FEATURES` const |
| Create | `crates/api/src/handlers/settings.rs` | `list_all`, `upsert`, `delete` handlers |
| Modify | `crates/api/src/handlers/mod.rs` | Expose `pub mod settings` |
| Modify | `crates/api/src/router.rs` | Register `/api/settings` routes |
| Modify | `crates/api/src/handlers/items.rs` | `check_item_features` helper + validation in `create`/`update` |
| Create | `crates/frontend/src/api/settings.rs` | `fetch_settings`, `upsert_setting` |
| Modify | `crates/frontend/src/api/mod.rs` | Expose `mod settings` |
| Modify | `crates/frontend/src/pages/settings.rs` | AI preferences toggle section |
| Modify | `gateway/src/mcp/api.ts` | Remove `ensureFeatures` |
| Modify | `gateway/src/mcp/tools/items.ts` | Remove `ensureFeatures` calls, add on-demand auto-enable logic, update descriptions |
| Modify | `gateway/src/mcp/tools/lists.ts` | Add `enable_list_feature`, `disable_list_feature` |

---

## Task 1: DB Migration

**Files:**
- Create: `crates/api/migrations/0011_user_settings.sql`

- [ ] **Step 1: Verify migration number**

  ```bash
  ls crates/api/migrations/ | sort | tail -3
  ```
  Expected: highest existing is `0010_containers.sql`. Confirm `0011` is free.

- [ ] **Step 2: Create migration file**

  ```sql
  -- crates/api/migrations/0011_user_settings.sql
  CREATE TABLE user_settings (
      user_id TEXT NOT NULL,
      key TEXT NOT NULL,
      value TEXT NOT NULL,
      updated_at TEXT NOT NULL DEFAULT (datetime('now')),
      PRIMARY KEY (user_id, key)
  );
  ```

- [ ] **Step 3: Apply to local dev DB**

  ```bash
  just migrate-dev
  ```
  Expected: migration applied without errors.

- [ ] **Step 4: Commit**

  ```bash
  git add crates/api/migrations/0011_user_settings.sql
  git commit -m "feat: add user_settings migration"
  ```

---

## Task 2: Shared Types

**Files:**
- Modify: `crates/shared/src/lib.rs` — add after existing `FEATURE_QUANTITY`/`FEATURE_DEADLINES` consts

- [ ] **Step 1: Add types and constant**

  In `crates/shared/src/lib.rs`, after the `parse_deadlines_config` function, add:

  ```rust
  pub const SETTING_MCP_AUTO_ENABLE_FEATURES: &str = "mcp_auto_enable_features";

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct UserSetting {
      pub key: String,
      pub value: serde_json::Value,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct UpsertSettingRequest {
      pub value: serde_json::Value,
  }
  ```

- [ ] **Step 2: Verify compilation**

  ```bash
  cargo check -p kartoteka-shared
  ```
  Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

  ```bash
  git add crates/shared/src/lib.rs
  git commit -m "feat: add UserSetting types and SETTING_MCP_AUTO_ENABLE_FEATURES const"
  ```

---

## Task 3: API Settings Handler

**Files:**
- Create: `crates/api/src/handlers/settings.rs`
- Modify: `crates/api/src/handlers/mod.rs`
- Modify: `crates/api/src/router.rs`

- [ ] **Step 1: Create `settings.rs` handler**

  ```rust
  // crates/api/src/handlers/settings.rs
  use kartoteka_shared::UpsertSettingRequest;
  use std::collections::HashMap;
  use worker::*;

  pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
      let user_id = ctx.data.clone();
      let d1 = ctx.env.d1("DB")?;

      let rows = d1
          .prepare("SELECT key, value FROM user_settings WHERE user_id = ?1")
          .bind(&[user_id.into()])?
          .all()
          .await?
          .results::<serde_json::Value>()?;

      // Build flat map {key: parsed_value, ...}
      let mut map: HashMap<String, serde_json::Value> = HashMap::new();
      for row in rows {
          if let (Some(key), Some(raw_value)) = (
              row.get("key").and_then(|v| v.as_str()).map(String::from),
              row.get("value").and_then(|v| v.as_str()),
          ) {
              let parsed = serde_json::from_str(raw_value).unwrap_or(serde_json::Value::Null);
              map.insert(key, parsed);
          }
      }
      Response::from_json(&map)
  }

  pub async fn upsert(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
      let user_id = ctx.data.clone();
      let key = ctx
          .param("key")
          .ok_or_else(|| Error::from("Missing key"))?
          .to_string();
      let body: UpsertSettingRequest = req.json().await?;
      let d1 = ctx.env.d1("DB")?;

      let value_str = body.value.to_string();
      d1.prepare(
          "INSERT OR REPLACE INTO user_settings (user_id, key, value, updated_at) \
           VALUES (?1, ?2, ?3, datetime('now'))",
      )
      .bind(&[user_id.into(), key.into(), value_str.into()])?
      .run()
      .await?;

      Ok(Response::empty()?.with_status(204))
  }

  pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
      let user_id = ctx.data.clone();
      let key = ctx
          .param("key")
          .ok_or_else(|| Error::from("Missing key"))?
          .to_string();
      let d1 = ctx.env.d1("DB")?;

      d1.prepare("DELETE FROM user_settings WHERE user_id = ?1 AND key = ?2")
          .bind(&[user_id.into(), key.into()])?
          .run()
          .await?;

      Ok(Response::empty()?.with_status(204))
  }
  ```

  **Note on 204 pattern:** Look at an existing handler that returns 204 (e.g., `items::delete`) for the exact pattern used in this codebase. The `Response::empty()?.with_status(204)` may need `Ok(...)` wrapping — follow existing pattern.

- [ ] **Step 2: Register module in `handlers/mod.rs`**

  Add `pub mod settings;` to `crates/api/src/handlers/mod.rs`:

  ```rust
  pub mod containers;
  pub mod items;
  pub mod lists;
  pub mod settings;
  pub mod tags;
  ```

- [ ] **Step 3: Add routes to `router.rs`**

  In `crates/api/src/router.rs`, add import and routes:

  ```rust
  use crate::handlers::{containers, items, lists, settings, tags};
  ```

  Add before `.run(req, env)`:
  ```rust
  // User settings
  .get_async("/api/settings", settings::list_all)
  .put_async("/api/settings/:key", settings::upsert)
  .delete_async("/api/settings/:key", settings::delete)
  ```

- [ ] **Step 4: Compile check**

  ```bash
  cargo check -p kartoteka-api
  ```
  Expected: `Finished` with no errors.

- [ ] **Step 5: Manual test via curl (local dev)**

  Start local dev (`just dev-api`) then:
  ```bash
  # PUT a setting
  curl -X PUT http://localhost:8787/api/settings/mcp_auto_enable_features \
    -H "Content-Type: application/json" \
    -H "X-User-Id: test-user" \
    -d '{"value": true}'
  # Expected: 204

  # GET all settings
  curl http://localhost:8787/api/settings \
    -H "X-User-Id: test-user"
  # Expected: {"mcp_auto_enable_features": true}

  # DELETE
  curl -X DELETE http://localhost:8787/api/settings/mcp_auto_enable_features \
    -H "X-User-Id: test-user"
  # Expected: 204

  # GET after delete — should return empty map
  curl http://localhost:8787/api/settings \
    -H "X-User-Id: test-user"
  # Expected: {}
  ```

- [ ] **Step 6: Commit**

  ```bash
  git add crates/api/src/handlers/settings.rs \
          crates/api/src/handlers/mod.rs \
          crates/api/src/router.rs
  git commit -m "feat: add user settings CRUD API"
  ```

---

## Task 4: API Item Feature Validation

**Files:**
- Modify: `crates/api/src/handlers/items.rs`

This task adds a helper `check_item_features` and calls it in `create` and `update`.

- [ ] **Step 1: Add `check_item_features` helper**

  Add after the `update_nullable_str` helper (around line 137):

  ```rust
  /// Returns a 422 Response if the request uses feature-gated fields without
  /// the corresponding feature being enabled on the list.
  /// Returns None if validation passes.
  fn check_item_features(
      feature_names: &[String],
      has_date_field: bool,
      has_quantity_field: bool,
  ) -> Option<Response> {
      if has_date_field && !feature_names.iter().any(|f| f == FEATURE_DEADLINES) {
          return Response::error(
              r#"{"error":"feature_required","feature":"deadlines","message":"This list does not have the 'deadlines' feature enabled. Enable it in list settings or retry without date fields."}"#,
              422,
          ).ok();
      }
      if has_quantity_field && !feature_names.iter().any(|f| f == FEATURE_QUANTITY) {
          return Response::error(
              r#"{"error":"feature_required","feature":"quantity","message":"This list does not have the 'quantity' feature enabled. Enable it in list settings or retry without quantity fields."}"#,
              422,
          ).ok();
      }
      None
  }
  ```

- [ ] **Step 2: Add feature validation to `create` handler**

  In the `create` function, after the ownership check (after `if list_check.is_none() { return ... }`), before the `max_pos` query, add:

  ```rust
  // Fetch list features for validation
  let feature_rows = d1
      .prepare("SELECT feature_name FROM list_features WHERE list_id = ?1")
      .bind(&[list_id.clone().into()])?
      .all()
      .await?
      .results::<serde_json::Value>()?;
  let feature_names: Vec<String> = feature_rows
      .iter()
      .filter_map(|r| r.get("feature_name")?.as_str().map(String::from))
      .collect();

  let has_date_field = body.start_date.is_some()
      || body.deadline.is_some()
      || body.hard_deadline.is_some()
      || body.start_time.is_some()
      || body.deadline_time.is_some();
  let has_quantity_field = body.quantity.is_some() || body.unit.is_some();

  if let Some(err_resp) = check_item_features(&feature_names, has_date_field, has_quantity_field) {
      return Ok(err_resp);
  }
  ```

- [ ] **Step 3: Update ownership query in `update` to also return `list_id`**

  In the `update` function, change the ownership query from:
  ```rust
  "SELECT items.id FROM items \
   JOIN lists ON lists.id = items.list_id \
   WHERE items.id = ?1 AND lists.user_id = ?2"
  ```
  to:
  ```rust
  "SELECT items.id, items.list_id FROM items \
   JOIN lists ON lists.id = items.list_id \
   WHERE items.id = ?1 AND lists.user_id = ?2"
  ```

  Then extract `list_id` from the result — change the check to:
  ```rust
  let item_check = d1
      .prepare(
          "SELECT items.id, items.list_id FROM items \
           JOIN lists ON lists.id = items.list_id \
           WHERE items.id = ?1 AND lists.user_id = ?2",
      )
      .bind(&[id.clone().into(), user_id.into()])?
      .first::<serde_json::Value>(None)
      .await?;
  if item_check.is_none() {
      return Response::error("Not found", 404);
  }
  let list_id_for_features = item_check
      .as_ref()
      .and_then(|v| v.get("list_id")?.as_str().map(String::from))
      .ok_or_else(|| Error::from("Missing list_id on item"))?;
  ```

- [ ] **Step 4: Add feature validation to `update` handler**

  After extracting `list_id_for_features`, before the first field-update block, add:

  ```rust
  // Fetch list features for validation
  let feature_rows = d1
      .prepare("SELECT feature_name FROM list_features WHERE list_id = ?1")
      .bind(&[list_id_for_features.into()])?
      .all()
      .await?
      .results::<serde_json::Value>()?;
  let feature_names: Vec<String> = feature_rows
      .iter()
      .filter_map(|r| r.get("feature_name")?.as_str().map(String::from))
      .collect();

  // For update: date field = Some(Some(_)) only (not clear/no-change)
  let has_date_field = matches!(&body.start_date, Some(Some(_)))
      || matches!(&body.deadline, Some(Some(_)))
      || matches!(&body.hard_deadline, Some(Some(_)))
      || matches!(&body.start_time, Some(Some(_)))
      || matches!(&body.deadline_time, Some(Some(_)));
  let has_quantity_field = body.quantity.is_some()
      || body.actual_quantity.is_some()
      || body.unit.is_some();

  if let Some(err_resp) = check_item_features(&feature_names, has_date_field, has_quantity_field) {
      return Ok(err_resp);
  }
  ```

- [ ] **Step 5: Compile check**

  ```bash
  cargo check -p kartoteka-api
  ```
  Expected: `Finished` with no errors.

- [ ] **Step 6: Manual tests (local dev)**

  ```bash
  # Create a plain checklist (no features)
  LIST_ID=$(curl -s -X POST http://localhost:8787/api/lists \
    -H "Content-Type: application/json" -H "X-User-Id: test-user" \
    -d '{"name":"Test","list_type":"checklist"}' | jq -r .id)

  # Try to add item with deadline → expect 422
  curl -s -X POST http://localhost:8787/api/lists/$LIST_ID/items \
    -H "Content-Type: application/json" -H "X-User-Id: test-user" \
    -d '{"title":"Task","deadline":"2026-04-01"}'
  # Expected: 422 with {"error":"feature_required","feature":"deadlines",...}

  # Add item without deadline → expect 201
  curl -s -X POST http://localhost:8787/api/lists/$LIST_ID/items \
    -H "Content-Type: application/json" -H "X-User-Id: test-user" \
    -d '{"title":"Simple task"}'
  # Expected: 201 with item JSON

  # Enable deadlines feature then add with deadline → expect 201
  curl -s -X POST http://localhost:8787/api/lists/$LIST_ID/features/deadlines \
    -H "Content-Type: application/json" -H "X-User-Id: test-user" \
    -d '{"config":{"has_start_date":false,"has_deadline":true,"has_hard_deadline":false}}'
  curl -s -X POST http://localhost:8787/api/lists/$LIST_ID/items \
    -H "Content-Type: application/json" -H "X-User-Id: test-user" \
    -d '{"title":"Task with deadline","deadline":"2026-04-01"}'
  # Expected: 201
  ```

- [ ] **Step 7: Lint**

  ```bash
  just lint
  ```
  Expected: no errors.

- [ ] **Step 8: Commit**

  ```bash
  git add crates/api/src/handlers/items.rs
  git commit -m "feat: validate feature-gated item fields, return 422 if feature not enabled"
  ```

---

## Task 5: Frontend Settings API

**Files:**
- Create: `crates/frontend/src/api/settings.rs`
- Modify: `crates/frontend/src/api/mod.rs`

- [ ] **Step 1: Create `api/settings.rs`**

  ```rust
  // crates/frontend/src/api/settings.rs
  use gloo_net::http::Request;

  pub async fn fetch_settings() -> Result<serde_json::Value, String> {
      super::get(&format!("{}/settings", super::API_BASE))
          .send()
          .await
          .map_err(|e| e.to_string())?
          .json()
          .await
          .map_err(|e| e.to_string())
  }

  pub async fn upsert_setting(key: &str, value: serde_json::Value) -> Result<(), String> {
      let json = serde_json::to_string(&serde_json::json!({ "value": value }))
          .map_err(|e| e.to_string())?;
      let resp = Request::put(&format!("{}/settings/{key}", super::API_BASE))
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

- [ ] **Step 2: Register module in `api/mod.rs`**

  Add `mod settings; pub use settings::*;` to `crates/frontend/src/api/mod.rs`:

  ```rust
  mod containers;
  mod items;
  mod lists;
  mod settings;
  mod tags;

  pub use containers::*;
  pub use items::*;
  pub use lists::*;
  pub use settings::*;
  pub use tags::*;
  // ... rest unchanged
  ```

- [ ] **Step 3: Compile check (note: needs API_BASE_URL env var)**

  ```bash
  cd crates/frontend && API_BASE_URL="/api" cargo check 2>&1 | grep -v "API_BASE_URL"
  ```
  Or just proceed — frontend compile errors surface during `just dev` or `just check`.

- [ ] **Step 4: Commit**

  ```bash
  git add crates/frontend/src/api/settings.rs crates/frontend/src/api/mod.rs
  git commit -m "feat: add settings API functions to frontend"
  ```

---

## Task 6: Frontend Settings Page UI

**Files:**
- Modify: `crates/frontend/src/pages/settings.rs`

- [ ] **Step 1: Update `SettingsPage` component**

  Replace the entire `SettingsPage` component body with a version that:
  1. Loads settings on mount via `fetch_settings()`
  2. Derives `mcp_auto_enable` bool from the map
  3. Adds a toggle that calls `upsert_setting`

  ```rust
  // crates/frontend/src/pages/settings.rs
  use leptos::prelude::*;
  use kartoteka_shared::SETTING_MCP_AUTO_ENABLE_FEATURES;

  use crate::api;

  #[component]
  pub fn McpRedirect() -> impl IntoView {
      let url = format!("{}/mcp", api::auth_base());
      if let Some(window) = web_sys::window() {
          let _ = window.location().set_href(&url);
      }
      view! { <></> }
  }

  #[component]
  pub fn SettingsPage() -> impl IntoView {
      let mcp_url = format!("{}/mcp", api::auth_base());
      let copied = RwSignal::new(false);
      let auto_enable = RwSignal::new(false);

      // Load settings on mount
      leptos::task::spawn_local(async move {
          if let Ok(settings) = api::fetch_settings().await {
              let val = settings
                  .get(SETTING_MCP_AUTO_ENABLE_FEATURES)
                  .and_then(|v| v.as_bool())
                  .unwrap_or(false);
              auto_enable.set(val);
          }
      });

      let mcp_url_copy = mcp_url.clone();
      let on_copy = move |_| {
          let url = mcp_url_copy.clone();
          if let Some(window) = web_sys::window() {
              let _ = window.navigator().clipboard().write_text(&url);
              copied.set(true);
              leptos::task::spawn_local(async move {
                  gloo_timers::future::TimeoutFuture::new(2000).await;
                  copied.set(false);
              });
          }
      };

      view! {
          <div class="container mx-auto max-w-2xl p-4">
              <h2 class="text-2xl font-bold mb-4">"Ustawienia"</h2>

              <div class="card bg-base-200 border border-base-300 mb-4">
                  <div class="card-body">
                      <h3 class="card-title text-lg">"Claude / MCP"</h3>
                      <p class="text-sm text-base-content/70 mb-2">
                          "Wklej ten URL w konfiguracji Claude Code jako MCP server:"
                      </p>
                      <div class="flex gap-2 items-center">
                          <code class="bg-base-300 rounded px-3 py-2 text-sm flex-1 break-all">
                              {mcp_url.clone()}
                          </code>
                          <button
                              class="btn btn-sm btn-outline"
                              on:click=on_copy
                          >
                              {move || if copied.get() { "Skopiowano!" } else { "Kopiuj" }}
                          </button>
                      </div>
                  </div>
              </div>

              <div class="card bg-base-200 border border-base-300 mb-4">
                  <div class="card-body">
                      <h3 class="card-title text-lg">"Zachowanie AI"</h3>
                      <label class="label cursor-pointer justify-start gap-4">
                          <input
                              type="checkbox"
                              class="toggle toggle-sm"
                              prop:checked=auto_enable
                              on:change=move |ev| {
                                  let checked = event_target_checked(&ev);
                                  auto_enable.set(checked);
                                  leptos::task::spawn_local(async move {
                                      let _ = api::upsert_setting(
                                          SETTING_MCP_AUTO_ENABLE_FEATURES,
                                          serde_json::Value::Bool(checked),
                                      ).await;
                                  });
                              }
                          />
                          <div>
                              <div class="label-text font-medium">
                                  "Automatycznie włączaj funkcje list"
                              </div>
                              <div class="label-text text-xs text-base-content/60">
                                  "Gdy AI potrzebuje terminu lub ilości na liście bez tych funkcji, włączy je bez pytania."
                              </div>
                          </div>
                      </label>
                  </div>
              </div>

              <div class="card bg-base-200 border border-base-300">
                  <div class="card-body">
                      <p class="text-base-content/60">"Ustawienia konta"</p>
                  </div>
              </div>
          </div>
      }
  }
  ```

- [ ] **Step 2: Run `just dev` and verify settings page**

  Navigate to `/settings`, confirm the toggle renders, toggle it, refresh — confirm it persists (loads saved value on mount).

- [ ] **Step 3: Commit**

  ```bash
  git add crates/frontend/src/pages/settings.rs
  git commit -m "feat: add AI auto-enable preferences toggle to settings page"
  ```

---

## Task 7: MCP — Remove `ensureFeatures`, Add Feature Tools

**Files:**
- Modify: `gateway/src/mcp/api.ts`
- Modify: `gateway/src/mcp/tools/items.ts`
- Modify: `gateway/src/mcp/tools/lists.ts`

- [ ] **Step 1: Remove `ensureFeatures` from `api.ts`**

  Delete the entire `ensureFeatures` function (lines 56-78) from `gateway/src/mcp/api.ts`. Nothing else in this file changes.

- [ ] **Step 2: Update `items.ts` — remove imports and calls, update descriptions, add auto-enable logic**

  Replace the entire content of `gateway/src/mcp/tools/items.ts`:

  ```typescript
  import { z } from "zod";
  import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
  import type { ApiContext } from "../api";
  import { callTool, apiCall, errorResult, jsonResult } from "../api";

  export function registerItemTools(server: McpServer, api: ApiContext): void {
    server.registerTool("get_list_items", {
      description: "Get all items in a specific list",
      inputSchema: {
        list_id: z.string().describe("The list ID"),
      },
    }, ({ list_id }) => callTool(api, "GET", `/api/lists/${list_id}/items`));

    server.registerTool("add_item", {
      description: "Add a new item to a list. Returns an error if the list does not have the required feature enabled (use enable_list_feature to enable it first, or retry without the field).",
      inputSchema: {
        list_id: z.string().describe("The list ID"),
        title: z.string().describe("Item title"),
        description: z.string().optional().describe("Item description"),
        quantity: z.number().optional().describe("Target quantity"),
        unit: z.string().optional().describe("Unit of measurement"),
        start_date: z.string().optional().describe("Start date YYYY-MM-DD"),
        start_time: z.string().optional().describe("Start time HH:MM"),
        deadline: z.string().optional().describe("Deadline YYYY-MM-DD"),
        deadline_time: z.string().optional().describe("Deadline time HH:MM"),
        hard_deadline: z.string().optional().describe("Hard deadline YYYY-MM-DD"),
      },
    }, async ({ list_id, ...fields }) => {
      return withAutoEnable(api, list_id, fields, (f) =>
        apiCall(api, "POST", `/api/lists/${list_id}/items`, f)
      );
    });

    server.registerTool("update_item", {
      description: "Update an item. Returns an error if updating feature-gated fields on a list without the required feature enabled.",
      inputSchema: {
        list_id: z.string().describe("The list ID"),
        item_id: z.string().describe("The item ID"),
        title: z.string().optional().describe("New title"),
        description: z.string().nullable().optional().describe("New description (null to clear)"),
        completed: z.boolean().optional().describe("Completion state"),
        quantity: z.number().optional().describe("Target quantity"),
        actual_quantity: z.number().optional().describe("Actual quantity (auto-completes when >= quantity)"),
        unit: z.string().nullable().optional().describe("Unit (null to clear)"),
        start_date: z.string().nullable().optional().describe("Start date YYYY-MM-DD (null to clear)"),
        start_time: z.string().nullable().optional().describe("Start time HH:MM (null to clear)"),
        deadline: z.string().nullable().optional().describe("Deadline YYYY-MM-DD (null to clear)"),
        deadline_time: z.string().nullable().optional().describe("Deadline time HH:MM (null to clear)"),
        hard_deadline: z.string().nullable().optional().describe("Hard deadline YYYY-MM-DD (null to clear)"),
      },
    }, async ({ list_id, item_id, ...fields }) => {
      return withAutoEnable(api, list_id, fields, (f) =>
        apiCall(api, "PUT", `/api/lists/${list_id}/items/${item_id}`, f)
      );
    });

    server.registerTool("toggle_item", {
      description: "Toggle the completed state of an item",
      inputSchema: {
        list_id: z.string().describe("The list ID"),
        item_id: z.string().describe("The item ID"),
        completed: z.boolean().describe("New completed state"),
      },
    }, ({ list_id, item_id, completed }) =>
      callTool(api, "PUT", `/api/lists/${list_id}/items/${item_id}`, { completed }));

    server.registerTool("move_item", {
      description: "Move an item to a different list",
      inputSchema: {
        item_id: z.string().describe("The item ID"),
        target_list_id: z.string().describe("Target list ID"),
      },
    }, ({ item_id, target_list_id }) =>
      callTool(api, "PATCH", `/api/items/${item_id}/move`, { list_id: target_list_id }));
  }

  /**
   * Execute an API call. If the API returns 422 feature_required, check the
   * user's mcp_auto_enable_features setting. If true, auto-enable the feature
   * and retry once. Otherwise, return an actionable error for Claude to surface.
   */
  async function withAutoEnable(
    api: ApiContext,
    listId: string,
    fields: Record<string, unknown>,
    apiFn: (f: Record<string, unknown>) => Promise<Response>
  ): Promise<{ content: { type: "text"; text: string }[]; isError?: boolean }> {
    const res = await apiFn(fields);
    if (!res.ok) {
      if (res.status === 422) {
        let body: { error?: string; feature?: string; message?: string } = {};
        try { body = await res.json(); } catch { /* ignore */ }

        if (body.error === "feature_required" && body.feature) {
          // Check user preference (on-demand — server is stateless)
          let autoEnable = false;
          try {
            const settings = await apiCall(api, "GET", "/api/settings").then(r => r.json());
            autoEnable = settings["mcp_auto_enable_features"] === true;
          } catch { /* default false */ }

          if (autoEnable) {
            const config = body.feature === "deadlines"
              ? { has_start_date: false, has_deadline: true, has_hard_deadline: false }
              : {};
            const enableRes = await apiCall(api, "POST", `/api/lists/${listId}/features/${body.feature}`, { config });
            if (!enableRes.ok) {
              return errorResult(`Failed to auto-enable feature '${body.feature}': ${await enableRes.text()}`);
            }
            const retry = await apiFn(fields);
            if (!retry.ok) {
              return errorResult(`API error ${retry.status}: ${await retry.text()}`);
            }
            return jsonResult(await retry.json());
          }

          return errorResult(
            `${body.message ?? "Feature not enabled."} Options: (1) use enable_list_feature tool to enable it, (2) retry without the field.`
          );
        }
      }
      return errorResult(`API error ${res.status}: ${await res.text()}`);
    }
    return jsonResult(await res.json());
  }
  ```

- [ ] **Step 3: Add `enable_list_feature` and `disable_list_feature` to `lists.ts`**

  In `gateway/src/mcp/tools/lists.ts`, add these two tools inside `registerListTools` (before the closing `}`):

  ```typescript
  server.registerTool("enable_list_feature", {
    description: "Enable a feature on a list. For 'deadlines', optionally configure which date fields are available. For 'quantity', optionally set a default unit. Call only after confirming with the user (unless mcp_auto_enable_features is set).",
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      feature: z.enum(["quantity", "deadlines"]).describe("Feature to enable"),
      // deadlines sub-config (defaults: start=false, deadline=true, hard=false)
      has_start_date: z.boolean().optional().describe("Show start date field (default false)"),
      has_deadline: z.boolean().optional().describe("Show deadline field (default true)"),
      has_hard_deadline: z.boolean().optional().describe("Show hard deadline field (default false)"),
      // quantity sub-config
      unit_default: z.string().optional().describe("Default unit label, e.g. 'szt', 'kg'"),
    },
  }, async ({ list_id, feature, has_start_date, has_deadline, has_hard_deadline, unit_default }) => {
    const config = feature === "deadlines"
      ? {
          has_start_date: has_start_date ?? false,
          has_deadline: has_deadline ?? true,
          has_hard_deadline: has_hard_deadline ?? false,
        }
      : unit_default
        ? { unit_default }
        : {};
    return callTool(api, "POST", `/api/lists/${list_id}/features/${feature}`, { config });
  });

  server.registerTool("disable_list_feature", {
    description: "Disable a feature on a list. Item data (quantities, dates) is preserved — data is hidden in UI but not deleted.",
    inputSchema: {
      list_id: z.string().describe("The list ID"),
      feature: z.enum(["quantity", "deadlines"]).describe("Feature to disable"),
    },
  }, ({ list_id, feature }) =>
    callTool(api, "DELETE", `/api/lists/${list_id}/features/${feature}`));
  ```

- [ ] **Step 4: TypeScript type check**

  ```bash
  cd gateway && npx tsc --noEmit
  ```
  Expected: no errors. Fix any type issues (e.g., `Response` type from `apiCall` returning CF `Response` vs built-in).

- [ ] **Step 5: Manual test via local MCP**

  Start `just dev-gateway`. In Claude Code with MCP connected:
  - Ask Claude to add an item with a deadline to a list without deadlines
  - Expected: Claude surfaces "feature not enabled" error with options
  - Ask Claude to enable deadlines on the list
  - Expected: Claude calls `enable_list_feature`, succeeds, retries add_item
  - Set `mcp_auto_enable_features = true` in settings page, restart session
  - Ask Claude to add an item with a deadline to a list without deadlines
  - Expected: Claude auto-enables and adds without asking

- [ ] **Step 6: Commit**

  ```bash
  git add gateway/src/mcp/api.ts \
          gateway/src/mcp/tools/items.ts \
          gateway/src/mcp/tools/lists.ts
  git commit -m "feat: remove ensureFeatures, add enable/disable_list_feature tools, on-demand auto-enable"
  ```

---

## Task 8: Final Check & Deploy

- [ ] **Step 1: Full CI check**

  ```bash
  just ci
  ```
  Expected: all passes (fmt, lint, audit, machete, test).

- [ ] **Step 2: Apply migration to dev environment**

  ```bash
  just migrate-dev
  ```

- [ ] **Step 3: Deploy to dev**

  ```bash
  just deploy-dev
  ```

- [ ] **Step 4: Smoke test on dev**

  - Open `https://kartoteka-dev.pages.dev/settings` → toggle visible
  - Via MCP on dev: add item with deadline to list without feature → error surfaced
  - Enable feature via `enable_list_feature` → retry succeeds

- [ ] **Step 5: Apply migration to prod**

  ```bash
  just deploy-migrate
  ```

- [ ] **Step 6: Deploy to prod**

  ```bash
  just deploy
  ```

- [ ] **Step 7: Final commit if any fixes applied during deploy**

  ```bash
  git add -A && git commit -m "chore: post-deploy fixes" # only if needed
  ```
