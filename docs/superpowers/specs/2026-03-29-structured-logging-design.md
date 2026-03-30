# Structured Logging Design

## Motivation

Kartoteka has zero logging in the Rust API and Gateway. No `console_log!`, no `tracing`, nothing. We need full observability — request tracing, business events, error tracking — with the ability to debug on production.

The system must be Cloudflare-native now (CF Workers Logs + `console.log` JSON indexing) but designed so that migration to Axum/VPS (issue #24) requires changing only the initialization line. Handler code stays untouched.

## Approach

**Unified `tracing` facade** with `tracing-web` + `tracing-subscriber` JSON on CF Workers, swappable to `tracing-subscriber::fmt().json()` on Axum. One new crate: `kartoteka-logging`.

Key insight: Cloudflare's official [workers-rs tracing example](https://github.com/cloudflare/workers-rs/tree/main/examples/tracing) already provides the exact stack we need — no custom Layer required.

## JSON Log Schema

Every log entry (regardless of backend) shares a common core with optional contextual fields:

```json
{
  "timestamp": "2026-03-29T14:32:01.123Z",
  "level": "INFO",
  "request_id": "a1b2c3d4",
  "user_id": "uuid-...",
  "target": "kartoteka_api::handlers::lists",
  "message": "list created"
}
```

### Core fields (always present)

| Field | Source |
|-------|--------|
| `timestamp` | `UtcTime::rfc_3339()` (CF) / `tracing-subscriber` (Axum) |
| `level` | tracing level |
| `target` | Rust module path (automatic from `tracing`) |
| `message` | Event message |

### Contextual fields (present when applicable)

| Field | When |
|-------|------|
| `request_id` | All request-scoped logs — from `X-Request-Id` header or generated |
| `user_id` | All authenticated request logs — from `X-User-Id` header |
| `method`, `path`, `status`, `duration_ms` | Request lifecycle logs (span close) |
| `action` | Business events (`create_list`, `delete_item`, etc.) |
| `error` | Error logs |
| `list_id`, `item_id`, `tag_id` | Entity-specific business events |
| `mcp_tool`, `mcp_session_id` | MCP tool calls |
| `mcp_client_id`, `grant_type` | MCP OAuth flow |

Fields not relevant to a given log are omitted (not null, just absent). CF Workers Logs automatically indexes all JSON keys for filtering.

### Log examples by type

**HTTP request completed:**
```json
{"timestamp":"...","level":"INFO","request_id":"a1b2","user_id":"uuid","target":"kartoteka_api","message":"request completed","method":"POST","path":"/api/lists","status":201,"duration_ms":12}
```

**Business event:**
```json
{"timestamp":"...","level":"INFO","request_id":"a1b2","user_id":"uuid","target":"kartoteka_api::handlers::lists","message":"list created","action":"create_list","list_id":"uuid","list_name":"Groceries"}
```

**MCP tool call:**
```json
{"timestamp":"...","level":"INFO","request_id":"c3d4","user_id":"uuid","target":"kartoteka_gateway::mcp","message":"mcp tool executed","mcp_tool":"list_items","mcp_session_id":"sess-xyz","duration_ms":45}
```

**Error:**
```json
{"timestamp":"...","level":"ERROR","request_id":"a1b2","user_id":"uuid","target":"kartoteka_api::db","message":"query failed","error":"SQLITE_BUSY: database is locked","action":"create_list"}
```

## Architecture

### New crate: `kartoteka-logging`

```
crates/logging/
  Cargo.toml
  src/
    lib.rs           — init_cf(), init_axum(), re-exports (20 LOC)
    error.rs         — ApiError enum + ApiResult + log_level() (50 LOC)
    request_id.rs    — extract/generate X-Request-Id, inject into span (30 LOC)
```

~100 LOC total. Note: `into_response` wrapper lives in `crates/api` (not in logging crate) because it depends on `worker::Response`. The logging crate has no dependency on `worker`.

### Cargo.toml

```toml
[package]
name = "kartoteka-logging"

[features]
default = ["cf"]
cf = ["tracing-web", "time"]
axum = []

[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"

# CF-only
tracing-web = { version = "0.1", optional = true }
time = { version = "0.3", features = ["wasm-bindgen"], optional = true }
```

### Dependencies in other crates

```toml
# crates/api/Cargo.toml
[dependencies]
kartoteka-logging = { path = "../logging" }
tracing = "0.1"  # for #[instrument] and tracing::info!() in handlers
```

`crates/shared/` — no changes, shared crate does not log.

## Request Flow

### Current (CF Workers)

```
Client -> Gateway (TS/Hono)
           | generates request_id (nanoid)
           | logs: request received (JSON console.log)
           | sets X-Request-Id header
           v
         API Worker (Rust)
           | extracts X-Request-Id (or generates if missing)
           | opens tracing span: request_id, user_id, method, path
           | handlers emit business events inside span
           | span close -> logs status, duration_ms
           | tracing-web MakeConsoleWriter -> console.log(JSON)
           v
         CF Workers Logs (dashboard, query builder, 7-day retention)
```

### After migration (Axum)

```
Client -> Axum binary
           | tower-http TraceLayer opens span (automatic)
           | middleware extracts/generates request_id
           | handlers emit business events (SAME CODE)
           | tracing-subscriber fmt::json() -> stdout
           v
         stdout -> (Loki/Datadog/whatever)
```

### What changes at migration

| Element | CF (now) | Axum (future) |
|---------|----------|---------------|
| Init | `init_cf()` | `init_axum()` |
| Request span | Custom in main handler | `TraceLayer` from tower-http |
| Request ID | Custom extraction | Same middleware (ports 1:1) |
| Business events | `tracing::info!()` | `tracing::info!()` — **no changes** |
| Output | `console.log` via `MakeConsoleWriter` | stdout |
| Gateway | Separate TS worker | Gone — MCP in same binary |

## Initialization

### CF Workers

```rust
// crates/logging/src/lib.rs
use tracing_subscriber::fmt::format::Pretty;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;
use tracing_web::{performance_layer, MakeConsoleWriter};

pub fn init_cf() {
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
```

In API worker:
```rust
#[event(start)]
fn start() {
    kartoteka_logging::init_cf();
}
```

### Axum (future)

```rust
pub fn init_axum() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .init();
}
```

### Log level configuration

- **CF Workers**: Compile-time default (`info`). `#[event(start)]` has no access to `Env`. Feature flag `--features debug-logging` for debug builds.
- **Axum**: Standard `RUST_LOG` env var (`RUST_LOG=info`, `RUST_LOG=kartoteka_api::handlers::lists=debug`).
- **Dev**: `debug` level by default.

## Typed Handler Pattern

Handlers return `ApiResult<T>` instead of manually building responses and logging. A wrapper handles response conversion and logging automatically.

### ApiError

```rust
type ApiResult<T> = Result<T, ApiError>;

enum ApiError {
    NotFound(String),        // -> 404
    Forbidden,               // -> 403
    BadRequest(String),      // -> 400
    Internal(anyhow::Error), // -> 500
}

impl ApiError {
    fn status_code(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::Forbidden => 403,
            Self::BadRequest(_) => 400,
            Self::Internal(_) => 500,
        }
    }

    fn log_level(&self) -> tracing::Level {
        match self {
            Self::NotFound(_) | Self::BadRequest(_) => Level::DEBUG,
            Self::Forbidden => Level::WARN,
            Self::Internal(_) => Level::ERROR,
        }
    }
}
```

### Handler pattern

```rust
#[tracing::instrument(skip(d1), fields(action = "create_list"))]
async fn create_list(d1: &D1, user_id: &str, body: CreateListRequest) -> ApiResult<List> {
    let id = Uuid::new_v4();
    let pos = next_position(d1, "lists", "user_id = ?", &[user_id]).await?;
    d1.exec("INSERT INTO lists ...", &[&id, &body.name, &user_id]).await?;

    let list = List { id, name: body.name, position: pos, .. };
    Span::current().record("list_id", &tracing::field::display(&id));
    Ok(list)
}
```

### Response wrapper (lives in `crates/api`, not in logging crate)

```rust
// crates/api/src/response.rs
async fn into_response<T: Serialize>(result: ApiResult<T>, status: u16) -> Result<Response> {
    match result {
        Ok(data) => {
            tracing::info!("success");
            Response::from_json(&data).map(|r| r.with_status(status))
        }
        Err(e) => {
            match e.log_level() {
                Level::ERROR => tracing::error!(error = %e, "failed"),
                Level::WARN => tracing::warn!(error = %e, "failed"),
                _ => tracing::debug!(error = %e, "failed"),
            }
            Ok(Response::from_json(&ErrorResponse { error: e.to_string() })?
                .with_status(e.status_code()))
        }
    }
}
```

Benefits:
- Handler LOC drops from ~15-20 to ~5-8
- Success/error logging is automatic and consistent
- Error severity mapping is centralized
- Handlers are testable as pure functions returning `ApiResult<T>`
- Pattern ports 1:1 to Axum (`IntoResponse` trait impl)

## Span Structure

```
request{request_id="abc123" method="POST" path="/api/lists" user_id="uuid"}
  +-- INFO "list created" action="create_list" list_id="uuid" list_name="Groceries"
  +-- DEBUG "sql query" query="INSERT INTO lists..."
  +-- CLOSE -> INFO "request completed" status=201 duration_ms=12
```

One root span per request. `tracing-subscriber` JSON automatically attaches parent span fields to every event.

### Request span (CF Workers)

```rust
use tracing::Instrument;

pub async fn handle_request(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let request_id = req.headers().get("X-Request-Id")
        .unwrap_or_else(|| nanoid::nanoid!());
    let user_id = req.headers().get("X-User-Id").unwrap_or_default();

    let span = tracing::info_span!("request",
        request_id = %request_id,
        method = %req.method(),
        path = %req.path(),
        user_id = %user_id,
    );

    let response = route(req, env, ctx).instrument(span.clone()).await;

    let _enter = span.enter();
    tracing::info!(status = response.status_code(), "request completed");

    response
}
```

## Log Levels

| Level | What we log | Examples |
|-------|------------|---------|
| **ERROR** | Something broke, needs attention | DB query failed, panic caught, external service down |
| **WARN** | Suspicious but handled | Auth failed, rate limit hit, deprecated endpoint called |
| **INFO** | Normal operations, business events | Request completed, list created, item toggled, MCP tool executed |
| **DEBUG** | Troubleshooting details | SQL query text, request/response body, session lookup |
| **TRACE** | Ultra-verbose | Serialization steps, field parsing |

### What we log per layer

**Request lifecycle (automatic from middleware/span):**
- INFO span close: method, path, status, duration_ms, user_id, request_id
- WARN: 4xx responses (except 404)
- ERROR: 5xx responses

**Business events (manually in handlers):**
- INFO: CRUD operations with context (action, entity id, name)
- DEBUG: operation details (items moved, old vs new position)

**Auth/session:**
- INFO: login success, logout
- WARN: login failed, invalid session, unauthorized access attempt

**MCP (in Gateway):**
- INFO: tool call executed (tool name, duration, session_id)
- WARN: tool call failed (validation error)
- ERROR: tool call crashed
- INFO: OAuth flow events (authorize, token issued)

**DB:**
- DEBUG: query text + params (debug level only!)
- ERROR: query failure with error message
- WARN: slow query (> 100ms threshold)

### What we DON'T log

- GET requests don't get business events — request span (method, path, status, duration) is enough
- Request/response bodies at INFO level — DEBUG only
- PII (email, password, token) — `user_id` is sufficient

### Action naming convention

`verb_noun` consistently: `create_list`, `update_item`, `delete_tag`, `toggle_item`, `reorder_items`, `move_item`, `login_success`, `login_failed`, `mcp_tool_executed`, `mcp_tool_failed`.

## Gateway (TypeScript) Logger

Minimal structured logger with the same JSON schema. Generates and propagates `request_id`.

```typescript
// gateway/src/logger.ts
function log(level: string, message: string, fields: Record<string, unknown>) {
  console.log(JSON.stringify({
    timestamp: new Date().toISOString(),
    level,
    message,
    ...fields
  }));
}
```

### Gateway request middleware

```typescript
app.use('*', async (c, next) => {
  const requestId = c.req.header('x-request-id') ?? nanoid();
  c.set('requestId', requestId);
  c.header('X-Request-Id', requestId);

  const start = Date.now();
  await next();

  console.log(JSON.stringify({
    timestamp: new Date().toISOString(),
    level: "INFO",
    request_id: requestId,
    method: c.req.method,
    path: c.req.path,
    status: c.res.status,
    duration_ms: Date.now() - start,
    message: "request completed"
  }));
});
```

MCP tool calls logged here with the same `request_id`.

## Default log levels

- **Production:** `info`
- **Debug on prod (CF):** rebuild with debug feature flag
- **Debug on prod (Axum):** `RUST_LOG=kartoteka_api::handlers::lists=debug`
- **Dev:** `debug`
