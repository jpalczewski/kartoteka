# Structured Logging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add structured JSON logging across Rust API and TypeScript Gateway, using `tracing` facade with CF Workers-native output, designed for seamless migration to Axum.

**Architecture:** New `kartoteka-logging` crate provides `init_cf()` (tracing-web + MakeConsoleWriter) and `ApiError`/`ApiResult` types. API worker gets a request span in `lib.rs`, handlers use `#[instrument]` and `ApiResult<T>`. Gateway gets a request logging middleware with `X-Request-Id` propagation. Both workers enable CF Workers Logs via `[observability]` in wrangler.toml.

**Tech Stack:** tracing 0.1, tracing-subscriber 0.3 (json, env-filter, time), tracing-web 0.1, time 0.3 (wasm-bindgen), nanoid, Hono middleware

---

## File Map

### New files

| File | Responsibility |
|------|---------------|
| `crates/logging/Cargo.toml` | Crate manifest with `cf`/`axum` feature flags |
| `crates/logging/src/lib.rs` | `init_cf()`, `init_axum()`, re-exports |
| `crates/logging/src/error.rs` | `ApiError`, `ApiResult<T>`, `log_level()`, `Display` |
| `gateway/src/logger.ts` | Structured JSON logger + request logging middleware |

### Modified files

| File | Change |
|------|--------|
| `Cargo.toml` (workspace root) | Add `crates/logging` to workspace members |
| `crates/api/Cargo.toml` | Add `kartoteka-logging`, `tracing`, `nanoid` deps |
| `crates/api/src/lib.rs` | Add `#[event(start)]` for tracing init, request span in `main()` |
| `crates/api/src/error.rs` | Replace `json_error` with `into_response` wrapper using `ApiError` |
| `crates/api/src/handlers/containers.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/src/handlers/lists.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/src/handlers/items.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/src/handlers/tags.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/src/handlers/preferences.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/src/handlers/settings.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/src/handlers/admin.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/src/handlers/me.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/src/handlers/public.rs` | Add `#[instrument]` + `ApiResult` to all handlers |
| `crates/api/wrangler.toml` | Add `[observability]` section to enable Workers Logs |
| `gateway/wrangler.toml` | Add `[observability]` section to enable Workers Logs |
| `gateway/src/index.ts` | Add request logging middleware, generate/propagate `X-Request-Id` |
| `gateway/src/proxy.ts` | Propagate `X-Request-Id` header to API Worker |
| `gateway/src/middleware.ts` | Log auth events |

---

## Task 1: Create `kartoteka-logging` crate

**Files:**
- Create: `crates/logging/Cargo.toml`
- Create: `crates/logging/src/lib.rs`
- Create: `crates/logging/src/error.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Add `crates/logging` to workspace members**

In `Cargo.toml` (workspace root), add `"crates/logging"` to the members list:

```toml
[workspace]
resolver = "2"
members = [
    "crates/shared",
    "crates/api",
    "crates/frontend",
    "crates/i18n",
    "crates/logging",
]
```

- [ ] **Step 2: Create `crates/logging/Cargo.toml`**

```toml
[package]
name = "kartoteka-logging"
version.workspace = true
edition.workspace = true
publish = false

[lints]
workspace = true

[features]
default = ["cf"]
cf = ["tracing-web", "dep:time"]
axum = ["tracing-subscriber/ansi"]

[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter", "time"] }
worker = { version = "0.7", default-features = false }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# CF-only
tracing-web = { version = "0.1", optional = true }
time = { version = "0.3", features = ["wasm-bindgen"], optional = true }
```

Note: `worker` dependency is needed because `ApiError` needs `worker::Error` for `From` impl. Uses `default-features = false` to keep it light.

- [ ] **Step 3: Create `crates/logging/src/error.rs`**

```rust
use serde::Serialize;
use std::fmt;
use worker::Response;

/// Typed API error with automatic log-level mapping.
pub enum ApiError {
    /// 404 — resource not found. Log at DEBUG (client mistake).
    NotFound(String),
    /// 403 — forbidden. Log at WARN (suspicious).
    Forbidden,
    /// 400 — bad request. Log at DEBUG (client mistake).
    BadRequest(String),
    /// 500 — internal error. Log at ERROR (our problem).
    Internal(String),
}

/// Convenience alias for handler return types.
pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::Forbidden => 403,
            Self::BadRequest(_) => 400,
            Self::Internal(_) => 500,
        }
    }

    pub fn log_level(&self) -> tracing::Level {
        match self {
            Self::NotFound(_) | Self::BadRequest(_) => tracing::Level::DEBUG,
            Self::Forbidden => tracing::Level::WARN,
            Self::Internal(_) => tracing::Level::ERROR,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::NotFound(c) | Self::BadRequest(c) | Self::Internal(c) => c.as_str(),
            Self::Forbidden => "forbidden",
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(c) => write!(f, "not found: {c}"),
            Self::Forbidden => write!(f, "forbidden"),
            Self::BadRequest(c) => write!(f, "bad request: {c}"),
            Self::Internal(c) => write!(f, "internal error: {c}"),
        }
    }
}

impl From<worker::Error> for ApiError {
    fn from(e: worker::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    status: u16,
}

/// Convert `ApiResult<T>` to `worker::Result<Response>`, logging at the appropriate level.
pub fn into_response<T: Serialize>(result: ApiResult<T>, status: u16) -> worker::Result<Response> {
    match result {
        Ok(data) => {
            tracing::info!("success");
            Response::from_json(&data).map(|r| r.with_status(status))
        }
        Err(ref e) => {
            match e.log_level() {
                tracing::Level::ERROR => tracing::error!(error = %e, "failed"),
                tracing::Level::WARN => tracing::warn!(error = %e, "failed"),
                _ => tracing::debug!(error = %e, "failed"),
            }
            let body = ErrorBody {
                code: e.code().to_string(),
                status: e.status_code(),
            };
            Response::from_json(&body).map(|r| r.with_status(e.status_code()))
        }
    }
}

/// Shortcut for 200 OK response with logging.
pub fn ok_response<T: Serialize>(result: ApiResult<T>) -> worker::Result<Response> {
    into_response(result, 200)
}

/// Shortcut for 201 Created response with logging.
pub fn created_response<T: Serialize>(result: ApiResult<T>) -> worker::Result<Response> {
    into_response(result, 201)
}

/// Shortcut for 204 No Content response with logging.
pub fn no_content_response(result: ApiResult<()>) -> worker::Result<Response> {
    match result {
        Ok(()) => {
            tracing::info!("success");
            Ok(Response::empty()?.with_status(204))
        }
        Err(ref e) => {
            match e.log_level() {
                tracing::Level::ERROR => tracing::error!(error = %e, "failed"),
                tracing::Level::WARN => tracing::warn!(error = %e, "failed"),
                _ => tracing::debug!(error = %e, "failed"),
            }
            let body = ErrorBody {
                code: e.code().to_string(),
                status: e.status_code(),
            };
            Response::from_json(&body).map(|r| r.with_status(e.status_code()))
        }
    }
}
```

- [ ] **Step 4: Create `crates/logging/src/lib.rs`**

```rust
pub mod error;

pub use error::{ApiError, ApiResult, created_response, into_response, no_content_response, ok_response};

#[cfg(feature = "cf")]
pub fn init_cf() {
    use tracing_subscriber::fmt::format::Pretty;
    use tracing_subscriber::fmt::time::UtcTime;
    use tracing_subscriber::prelude::*;
    use tracing_web::{MakeConsoleWriter, performance_layer};

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_timer(UtcTime::rfc_3339())
        .with_writer(MakeConsoleWriter);
    let perf_layer = performance_layer().with_details_from_fields(Pretty::default());
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perf_layer)
        .init();
}

#[cfg(feature = "axum")]
pub fn init_axum() {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .init();
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check -p kartoteka-logging`
Expected: compiles successfully

- [ ] **Step 6: Commit**

```bash
git add crates/logging/ Cargo.toml
git commit -m "feat: add kartoteka-logging crate with tracing init and ApiError"
```

---

## Task 2: Enable Workers Logs in wrangler.toml

**Files:**
- Modify: `crates/api/wrangler.toml`
- Modify: `gateway/wrangler.toml`

- [ ] **Step 1: Add observability to API wrangler.toml**

Add at the end of `crates/api/wrangler.toml` (top-level, applies to all envs):

```toml
[observability]
enabled = true
```

- [ ] **Step 2: Add observability to Gateway wrangler.toml**

Read `gateway/wrangler.toml` and add the same `[observability]` section.

```toml
[observability]
enabled = true
```

- [ ] **Step 3: Commit**

```bash
git add crates/api/wrangler.toml gateway/wrangler.toml
git commit -m "feat: enable CF Workers Logs observability"
```

---

## Task 3: Wire tracing init + request span in API worker

**Files:**
- Modify: `crates/api/Cargo.toml`
- Modify: `crates/api/src/lib.rs`

- [ ] **Step 1: Add dependencies to `crates/api/Cargo.toml`**

Add these to the `[dependencies]` section:

```toml
kartoteka-logging = { path = "../logging" }
tracing = "0.1"
nanoid = "0.4"
```

- [ ] **Step 2: Add `#[event(start)]` and request span to `crates/api/src/lib.rs`**

Replace the entire file:

```rust
use tracing::Instrument;
use worker::*;

mod auth;
pub mod error;
mod handlers;
pub(crate) mod helpers;
mod router;

#[event(start)]
fn start() {
    kartoteka_logging::init_cf();
}

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let request_id = req
        .headers()
        .get("X-Request-Id")
        .ok()
        .flatten()
        .unwrap_or_else(|| nanoid::nanoid!());

    let user_id = req
        .headers()
        .get("X-User-Id")
        .ok()
        .flatten()
        .unwrap_or_default();

    let span = tracing::info_span!("request",
        request_id = %request_id,
        method = %req.method(),
        path = %req.path(),
        user_id = %user_id,
    );

    let response = router::handle(req, env).instrument(span.clone()).await;

    match &response {
        Ok(resp) => {
            let _enter = span.enter();
            tracing::info!(status = resp.status_code(), "request completed");
        }
        Err(e) => {
            let _enter = span.enter();
            tracing::error!(error = %e, "request failed");
        }
    }

    response
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p kartoteka-api`
Expected: compiles successfully. Note: this is a WASM target check — if native check fails due to WASM-only deps, try `cargo check -p kartoteka-api --target wasm32-unknown-unknown`.

- [ ] **Step 4: Commit**

```bash
git add crates/api/Cargo.toml crates/api/src/lib.rs
git commit -m "feat: wire tracing init and request span in API worker"
```

---

## Task 4: Update `error.rs` and migrate one handler module (containers)

This task establishes the handler migration pattern. Subsequent tasks follow it.

**Files:**
- Modify: `crates/api/src/error.rs`
- Modify: `crates/api/src/handlers/containers.rs`

- [ ] **Step 1: Update `crates/api/src/error.rs`**

Keep the existing `json_error` function (other handlers still use it) and add the new imports:

```rust
use kartoteka_shared::ErrorResponse;
use worker::Response;

pub fn json_error(code: &str, status: u16) -> worker::Result<Response> {
    let body = ErrorResponse {
        code: Some(code.to_string()),
        status,
    };
    Response::from_json(&body).map(|r| r.with_status(status))
}
```

No changes needed — `json_error` stays for now. Handlers will use `ApiError` from `kartoteka_logging` directly, and the `into_response` / `ok_response` / etc. functions handle error → Response conversion. Once all handlers are migrated, `json_error` can be removed.

- [ ] **Step 2: Add `#[instrument]` to containers handlers**

Add `use tracing::instrument;` at the top of `crates/api/src/handlers/containers.rs`.

For each handler, add `#[instrument(skip_all, fields(action = "..."))]` above the function signature. The `skip_all` is critical — `Request`, `RouteContext`, and D1 types don't implement `Debug` and cannot be logged.

Example for `create`:

```rust
#[instrument(skip_all, fields(action = "create_container"))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    // ... existing body unchanged ...
}
```

Apply to all handlers in `containers.rs`:
- `list_all` — no action field needed (GET, read-only)
- `create` — `action = "create_container"`
- `get_one` — no action field
- `update` — `action = "update_container"`
- `delete` — `action = "delete_container"`
- `get_children` — no action field
- `move_container` — `action = "move_container"`
- `toggle_pin` — `action = "toggle_container_pin"`
- `home` — no action field

For mutating handlers, add `tracing::Span::current().record(...)` after key operations to capture entity IDs. Example in `create`, after the INSERT succeeds:

```rust
tracing::Span::current().record("container_id", &tracing::field::display(&id));
```

To use dynamic span fields, the field must be declared in the `#[instrument]` macro with `Empty`:

```rust
#[instrument(skip_all, fields(action = "create_container", container_id = tracing::field::Empty))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    // ...
    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("container_id", &tracing::field::display(&id));
    // ... rest of handler
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p kartoteka-api`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/error.rs crates/api/src/handlers/containers.rs
git commit -m "feat: add tracing instrumentation to container handlers"
```

---

## Task 5: Instrument remaining handler modules

**Files:**
- Modify: `crates/api/src/handlers/lists.rs`
- Modify: `crates/api/src/handlers/items.rs`
- Modify: `crates/api/src/handlers/tags.rs`
- Modify: `crates/api/src/handlers/preferences.rs`
- Modify: `crates/api/src/handlers/settings.rs`
- Modify: `crates/api/src/handlers/admin.rs`
- Modify: `crates/api/src/handlers/me.rs`
- Modify: `crates/api/src/handlers/public.rs`

Follow the exact same pattern from Task 4. For each handler file:

1. Add `use tracing::instrument;` at the top
2. Add `#[instrument(skip_all, fields(action = "..."))]` to mutating handlers
3. Add `#[instrument(skip_all)]` to read-only handlers (no action field)
4. Add `Span::current().record()` for key entity IDs in mutating handlers

- [ ] **Step 1: Instrument `lists.rs`**

Action names: `create_list`, `update_list`, `delete_list`, `create_sublist`, `toggle_archive`, `reset_list`, `move_list`, `toggle_list_pin`, `add_list_feature`, `remove_list_feature`.

Dynamic fields: `list_id = tracing::field::Empty` on create/update/delete handlers.

- [ ] **Step 2: Instrument `items.rs`**

Action names: `create_item`, `update_item`, `delete_item`, `move_item`.

Dynamic fields: `item_id = tracing::field::Empty` on create/update/delete.

- [ ] **Step 3: Instrument `tags.rs`**

Action names: `create_tag`, `update_tag`, `delete_tag`, `merge_tags`, `assign_tag_to_item`, `remove_tag_from_item`, `assign_tag_to_list`, `remove_tag_from_list`.

Dynamic fields: `tag_id = tracing::field::Empty` on create/update/delete/merge.

- [ ] **Step 4: Instrument `preferences.rs`**

Action names: `update_preferences` (for PUT). GET has no action.

- [ ] **Step 5: Instrument `settings.rs`**

Action names: `upsert_setting`, `delete_setting`. GET has no action.

Dynamic fields: `setting_key = tracing::field::Empty`.

- [ ] **Step 6: Instrument `admin.rs`**

Action names: `update_instance_setting`, `create_invitation_code`, `delete_invitation_code`. GET has no action.

- [ ] **Step 7: Instrument `me.rs`**

Read-only — `#[instrument(skip_all)]` with no action.

- [ ] **Step 8: Instrument `public.rs`**

Read-only — `#[instrument(skip_all)]` with no action. `validate_invite` gets `action = "validate_invite"`.

- [ ] **Step 9: Verify it compiles**

Run: `cargo check -p kartoteka-api`
Expected: compiles successfully

- [ ] **Step 10: Commit**

```bash
git add crates/api/src/handlers/
git commit -m "feat: add tracing instrumentation to all handler modules"
```

---

## Task 6: Gateway structured logging + request ID propagation

**Files:**
- Create: `gateway/src/logger.ts`
- Modify: `gateway/src/index.ts`
- Modify: `gateway/src/proxy.ts`
- Modify: `gateway/src/middleware.ts`
- Modify: `gateway/package.json`

- [ ] **Step 1: Add `nanoid` dependency**

Run: `cd gateway && npm install nanoid`

- [ ] **Step 2: Create `gateway/src/logger.ts`**

```typescript
type LogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";

export function log(level: LogLevel, message: string, fields: Record<string, unknown> = {}) {
  console.log(JSON.stringify({
    timestamp: new Date().toISOString(),
    level,
    message,
    ...fields,
  }));
}
```

- [ ] **Step 3: Add request logging middleware to `gateway/src/index.ts`**

Add this middleware right after the CORS middleware (before any routes):

```typescript
import { nanoid } from "nanoid";
import { log } from "./logger";

app.use("*", async (c, next) => {
  const requestId = c.req.header("x-request-id") ?? nanoid();
  c.set("requestId", requestId);
  c.header("X-Request-Id", requestId);

  const start = Date.now();
  await next();

  log("INFO", "request completed", {
    request_id: requestId,
    method: c.req.method,
    path: new URL(c.req.url).pathname,
    status: c.res.status,
    duration_ms: Date.now() - start,
  });
});
```

Update the `Variables` interface in `gateway/src/types.ts` to include `requestId`:

```typescript
export interface Variables {
  userId: string;
  userEmail: string;
  requestId: string;
}
```

- [ ] **Step 4: Propagate `X-Request-Id` in `gateway/src/proxy.ts`**

In the `proxy.all("/*", ...)` handler, add after the existing header setup:

```typescript
const requestId = c.get("requestId");
if (requestId) headers.set("X-Request-Id", requestId);
```

This goes right after `if (userEmail) headers.set("X-User-Email", userEmail);`.

- [ ] **Step 5: Add auth event logging to `gateway/src/middleware.ts`**

Add log calls for auth events:

```typescript
import { log } from "./logger";

// Inside authMiddleware:
// After successful auth:
log("INFO", "auth success", {
  request_id: c.get("requestId"),
  user_id: session.user.id,
});

// After auth failure (before returning 401):
log("WARN", "auth failed", {
  request_id: c.get("requestId"),
  path: new URL(c.req.url).pathname,
});
```

- [ ] **Step 6: Verify gateway builds**

Run: `cd gateway && npx tsc --noEmit`
Expected: no type errors

- [ ] **Step 7: Commit**

```bash
git add gateway/
git commit -m "feat: add structured logging and request ID propagation to gateway"
```

---

## Task 7: Smoke test locally

**Files:** none (testing only)

- [ ] **Step 1: Start dev environment**

Run: `just dev`

- [ ] **Step 2: Make API requests and check logs**

Make a few requests to the API:
```bash
curl -s http://localhost:8788/api/home | head -c 100
curl -s -X POST http://localhost:8788/api/lists -H 'Content-Type: application/json' -d '{"name":"test-log"}'
```

- [ ] **Step 3: Verify JSON log output**

Check the terminal output for both Gateway and API workers. You should see:
- Gateway: JSON logs with `request_id`, `method`, `path`, `status`, `duration_ms`
- API worker: JSON logs with `request_id`, `user_id`, `method`, `path`, tracing span fields
- The same `request_id` appearing in both Gateway and API logs for the same request

- [ ] **Step 4: Verify tracing fields**

Look for business event logs (e.g., from creating a list) containing:
- `action` field (e.g., `"create_list"`)
- `list_id` field (the UUID of the created list)
- `request_id` field matching the Gateway log

- [ ] **Step 5: Clean up test data**

Delete the test list created in step 2.

---

## Task 8: Migrate handlers to `ApiResult<T>` + remove `json_error` (future)

**Files:**
- Modify: `crates/api/src/error.rs`
- Modify: all handler files
- Modify: `crates/api/src/router.rs` (call sites)

This task is a **separate, larger refactor** — it changes every handler's return type from `Result<Response>` to `ApiResult<T>` and wraps call sites in `router.rs` with `ok_response()` / `created_response()` / `no_content_response()`. Tasks 1-7 deliver full structured logging without this refactor. Do this when ready to invest in the handler ergonomics improvement described in the design spec.

- [ ] **Step 1: Audit remaining `json_error` usage**

Run: `grep -rn "json_error" crates/api/src/`

If the list is small, replace each call:
- `json_error("container_not_found", 404)` → `return Err(ApiError::NotFound("container_not_found".into()))`
- `json_error("forbidden", 403)` → `return Err(ApiError::Forbidden)`
- `json_error("bad_request", 400)` → `return Err(ApiError::BadRequest("bad_request".into()))`

- [ ] **Step 2: Remove `json_error` from `error.rs`**

Once no callers remain, delete the function and the `kartoteka_shared::ErrorResponse` import.

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p kartoteka-api`

- [ ] **Step 4: Commit**

```bash
git add crates/api/
git commit -m "refactor: replace json_error with ApiError across all handlers"
```
