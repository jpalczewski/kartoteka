# Cloudflare Exit v2: Leptos SSR + Axum on VPS

Supersedes: `2026-03-28-cloudflare-exit-rewrite-design.md`
GitHub issue: #24

## Motivation

Cloudflare Workers CPU limit (10ms free tier) makes OAuth flow impossible without paid plan ($5/month). Two-language stack (Rust API + TypeScript Gateway) adds complexity. Exit to single Rust binary on own VPS.

Key change vs v1 spec: **Leptos SSR instead of CSR** — server functions replace REST API as primary frontend data layer. REST API kept as placeholders for external integrations.

## Target Architecture

```
Internet → Custom domain (DNS)
         → Caddy (reverse proxy, auto SSL)
         → localhost:3000 → kartoteka binary
         → SQLite file: data.db

Single binary serves:
  /           → Leptos SSR (HTML + hydration)
  /api/*      → REST API (thin wrappers on domain::)
  /auth/*     → Login, register, 2FA
  /mcp        → rmcp StreamableHTTP
  /oauth/*    → oxide-auth (MCP OAuth 2.1 + PKCE)
  /.well-known/* → OAuth metadata
```

## Workspace Structure

```
crates/
  shared/       — models, DTOs, enums, date_utils, constants
                  deps: serde, chrono, schemars
  db/           — sqlx queries + migrations, SqlitePool (internal to domain::)
                  deps: shared, sqlx[sqlite,chrono]
  domain/       — business logic, validation, orchestration — ONLY entry point for data access
                  deps: shared, db, chrono-tz, tokio
  i18n/         — FTL files, leptos-fluent (unchanged)
  mcp/          — rmcp tool_router, 5 tools + OAuth provider
                  deps: shared, domain, rmcp, schemars, oxide-auth, jsonwebtoken
  frontend/     — Leptos 0.8 SSR (cdylib + rlib)
                  features: ssr [leptos/ssr, leptos_axum, dep:domain]
                            hydrate [leptos/hydrate, wasm-bindgen]
                  deps: shared, i18n, leptos, leptos_router, leptos_meta, leptos-fluent
                  deps(ssr): domain, axum, axum-login, sqlx
  server/       — Axum binary, glue
                  deps: shared, domain, mcp, frontend(ssr)
                        axum, axum-login, tower-sessions, argon2, totp-rs
                        tower-http
```

### What disappears

- `crates/api/` (2164 LOC) — worker crate, replaced by server functions + REST placeholders
- `gateway/` (797 LOC TypeScript) — auth, MCP, proxy — all in Rust now

## Database (crates/db)

One consolidated migration (not 10 incremental D1 migrations). Clean cut — no data migration from D1.

### Schema

```sql
-- Auth
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    name TEXT,
    avatar_url TEXT,
    role TEXT NOT NULL DEFAULT 'user',  -- 'admin' for first registered user
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE auth_methods (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    provider TEXT NOT NULL,        -- 'password', future: 'github', 'passkey'
    provider_id TEXT NOT NULL,     -- email for password provider
    credential TEXT,               -- argon2 hash
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(provider, provider_id)
);

CREATE TABLE totp_secrets (
    user_id TEXT PRIMARY KEY REFERENCES users(id),
    secret TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Server config (global, not per-user)
CREATE TABLE server_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- Default: INSERT INTO server_config VALUES ('registration_enabled', 'true');

-- MCP OAuth (DCR clients, tokens are stateless via oxide-auth HMAC)
CREATE TABLE oauth_clients (
    client_id TEXT PRIMARY KEY,
    client_name TEXT,
    redirect_uris TEXT NOT NULL,   -- JSON array
    grant_types TEXT NOT NULL,     -- JSON array
    token_endpoint_auth_method TEXT DEFAULT 'none',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- App data (same as current, minus D1 quirks)
CREATE TABLE containers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT,
    status TEXT,  -- NULL=folder, 'active'/'done'/'paused'=project
    parent_container_id TEXT REFERENCES containers(id),
    position INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    last_opened_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE lists (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT,
    list_type TEXT NOT NULL DEFAULT 'checklist',
    parent_list_id TEXT REFERENCES lists(id),
    position INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    container_id TEXT REFERENCES containers(id),
    pinned INTEGER NOT NULL DEFAULT 0,
    last_opened_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE items (
    id TEXT PRIMARY KEY,
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    completed INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0,
    quantity INTEGER,
    actual_quantity INTEGER,
    unit TEXT,
    start_date TEXT,
    start_time TEXT,
    deadline TEXT,
    deadline_time TEXT,
    hard_deadline TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    color TEXT,
    parent_tag_id TEXT REFERENCES tags(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE item_tags (
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_id)
);

CREATE TABLE list_tags (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (list_id, tag_id)
);

CREATE TABLE list_features (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    feature_name TEXT NOT NULL,
    config TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (list_id, feature_name)
);

CREATE TABLE user_settings (
    user_id TEXT NOT NULL REFERENCES users(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, key)
);

-- No separate preferences table — locale, timezone etc. stored in user_settings

-- Indexes for hot query paths
CREATE INDEX idx_items_list_id ON items(list_id);
CREATE INDEX idx_items_deadline ON items(deadline) WHERE deadline IS NOT NULL;
CREATE INDEX idx_items_start_date ON items(start_date) WHERE start_date IS NOT NULL;
CREATE INDEX idx_lists_user_id ON lists(user_id);
CREATE INDEX idx_lists_pinned ON lists(user_id, pinned) WHERE pinned = 1;
CREATE INDEX idx_lists_container ON lists(container_id) WHERE container_id IS NOT NULL;
CREATE INDEX idx_containers_user_id ON containers(user_id);
CREATE INDEX idx_containers_pinned ON containers(user_id, pinned) WHERE pinned = 1;
CREATE INDEX idx_auth_methods_user_provider ON auth_methods(user_id, provider);
CREATE INDEX idx_tags_user_id ON tags(user_id);

-- OAuth state tables (for MCP OAuth provider)
CREATE TABLE IF NOT EXISTS oauth_authorization_codes (
    code TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
    user_id TEXT NOT NULL REFERENCES users(id),
    redirect_uri TEXT NOT NULL,
    code_challenge TEXT NOT NULL,
    code_challenge_method TEXT NOT NULL DEFAULT 'S256',
    scopes TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE IF NOT EXISTS oauth_refresh_tokens (
    token TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
    user_id TEXT NOT NULL REFERENCES users(id),
    scopes TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Sessions: auto-created by tower-sessions SqliteStore::migrate()
```

### Query pattern

```rust
// db/src/lists.rs
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<List>> {
    sqlx::query_as!(List, "SELECT ... FROM lists WHERE user_id = ?", user_id)
        .fetch_all(pool).await
}
```

Queries extracted 1:1 from current `crates/api/src/handlers/`. Same logic, different access layer (sqlx instead of D1 bindings). `bool_from_number` deserializer gone — sqlx maps INTEGER to bool natively.

Date fields (`start_date`, `deadline`, `hard_deadline`, etc.) use `chrono::NaiveDate` / `chrono::NaiveTime` instead of `Option<String>`. sqlx with `features = ["chrono"]` maps SQLite TEXT ↔ chrono types natively. Serde serializes `NaiveDate` as `"2026-04-12"` — identical JSON format, but type-safe.

## SQLite Optimization

### Connection setup (SqliteConnectOptions + after_connect)

- `journal_mode = WAL` — concurrent reads + single writer
- `foreign_keys = ON` — FK enforcement (off by default in SQLite!)
- `synchronous = NORMAL` — safe with WAL, faster than FULL
- `busy_timeout = 5000` — 5s retry on SQLITE_BUSY (via after_connect)
- `mmap_size = 268435456` — 256MB memory-mapped I/O (via after_connect)
- `optimize = 0x10002` — query planner stats on connect (via after_connect)

### STRICT tables

All tables use `STRICT` keyword — enforces column types, prevents silent type coercion.

### RETURNING clause

INSERT/UPDATE queries use `RETURNING *` instead of INSERT + separate SELECT. One round-trip instead of two.

### Partial indexes

```sql
CREATE INDEX idx_items_deadline ON items(deadline) WHERE deadline IS NOT NULL;
CREATE INDEX idx_items_start_date ON items(start_date) WHERE start_date IS NOT NULL;
CREATE INDEX idx_lists_pinned ON lists(user_id, pinned) WHERE pinned = 1;
CREATE INDEX idx_containers_pinned ON containers(user_id, pinned) WHERE pinned = 1;
CREATE INDEX idx_lists_container ON lists(container_id) WHERE container_id IS NOT NULL;
```

### SQLite-specific features retained

- `json_group_array` / `json_object` — feature aggregation in single query
- `WITH RECURSIVE` — tag tree traversal, cycle detection
- `datetime('now')` — server-side timestamps
- `ON CONFLICT ... DO UPDATE` — upserts

## Domain Layer (crates/domain)

Business logic separated from data access. All mutations go through domain::, reads can go directly to db::.

### Boundary

- **db::** — pure queries. INSERT/UPDATE/DELETE with user_id enforcement (defense in depth). Zero business logic. Internal to domain:: — consumers never call db:: directly.
- **domain::** — the ONLY entry point for all data access. Reads are thin pass-throughs (hook for future access control). Writes include validation, orchestration, transactions.

### Functions in domain::

- `domain::items::create` — feature validation + insert + auto-position
- `domain::items::update` — feature validation + partial update + auto-complete on quantity
- `domain::items::move_item` — ownership validation + target list validation + position
- `domain::containers::create` — hierarchy validation (parent != project) + insert
- `domain::containers::move_container` — self-reference check + hierarchy validation
- `domain::tags::update` — cycle detection + update
- `domain::tags::merge` — reassign links + reparent + delete (transaction)
- `domain::lists::create` — insert + default features from ListType (transaction)
- `domain::lists::reset` — reset items + sublist items (transaction)
- `domain::lists::toggle_pin` — toggle + future feature hook
- `domain::lists::toggle_archive` — toggle + future feature hook
- `domain::auth::register` — check registration enabled + first user = admin + hash password + create user + auth_method

### Consumers

Server functions, REST handlers, and MCP tools always call domain:: — never db:: directly:

```
GET  /api/lists      → domain::lists::list_all (pass-through, hook for future access control)
POST /api/lists      → domain::lists::create   (validation + transaction)
PUT  /api/lists/:id  → domain::lists::update   (validation)
```

### Workspace structure (updated)

```
crates/
  shared/       — models, DTOs, enums, date_utils, constants
  db/           — pure queries, SQLite optimized (RETURNING, partial indexes, STRICT)
  domain/       — business logic, validation, orchestration
  i18n/         — FTL files
  mcp/          — rmcp tools (calls domain:: for writes, db:: for reads)
  frontend/     — Leptos 0.8 SSR (server functions call domain::/db::)
  server/       — Axum binary (REST handlers call domain::/db::)
```

## Auth (axum-login + argon2 + totp-rs)

### Stack

- `axum-login` 0.18 — `AuthSession` extractor, `login_required!` middleware
- `tower-sessions` 0.15 + `tower-sessions-sqlx-store` — sessions in SQLite
- `argon2` — password hashing
- `totp-rs` 5.7 — TOTP 2FA

### Flows

**Register:**
```
POST /auth/register { email, password }
  → check server_config.registration_enabled
  → argon2 hash → insert users + auth_methods(provider='password')
  → first user gets role='admin'
  → auto-login → session cookie → redirect /
```

**Login:**
```
POST /auth/login { email, password }
  → verify argon2 → session cookie
  → if totp_secrets.verified=true → redirect /auth/2fa
  → else → redirect /
```

**2FA setup:**
```
POST /auth/totp/setup (authenticated)
  → totp-rs generate secret → insert totp_secrets(verified=false)
  → return QR URI
POST /auth/totp/verify { code }
  → verify → set verified=true
```

**2FA login:**
```
POST /auth/2fa { code }
  → verify → upgrade session → redirect /
```

### Server config

`server_config` table for global settings. First registered user = admin. Admin sees "Server" section in `/settings` with registration toggle. No restart required.

### AuthnBackend

```rust
struct AuthBackend { pool: SqlitePool }

impl AuthnBackend for AuthBackend {
    type User = User;
    type Credentials = LoginCredentials;
    async fn authenticate(&self, creds: Self::Credentials) -> Result<Option<Self::User>> {
        let user = db::users::find_by_email(&self.pool, &creds.email).await?;
        let method = db::auth_methods::find(&self.pool, user.id, "password").await?;
        verify_argon2(&method.credential, &creds.password)?;
        Ok(Some(user))
    }
}
```

## Frontend (Leptos 0.8 SSR)

### Key changes from current CSR

| Current (CSR) | New (SSR) |
|---|---|
| `gloo-net` HTTP calls | `#[server]` functions |
| `LocalResource` + `SendWrapper` | `Resource` (Send, serialized server→client) |
| `mount_to_body(App)` | `hydrate_body(App)` + server shell |
| Trunk build | cargo-leptos build |
| `API_BASE_URL` compile-time env | not needed, server functions = same process |
| `credentials: "include"` on every request | session available server-side via `extract()` |

### Server function pattern

```rust
#[server]
pub async fn fetch_lists() -> Result<Vec<List>, ServerFnError> {
    let state = expect_context::<AppState>();
    let auth: AuthSession<AuthBackend> = extract_with_state(&state).await?;
    let user = auth.user.ok_or(ServerFnError::new("unauthorized"))?;
    let pool = expect_context::<SqlitePool>();
    Ok(db::lists::list_all(&pool, &user.id).await?)
}
```

### Resource usage in components

```rust
// Before (CSR):
let lists = LocalResource::new(|| api::fetch_lists());

// After (SSR):
let lists = Resource::new(|| (), |_| fetch_lists());
```

### Browser-only code

`web_sys` direct access (clipboard, location, navigator) requires `#[cfg(feature = "hydrate")]` guards.

### i18n (leptos-fluent SSR)

Feature flags: `leptos-fluent/ssr` + `leptos-fluent/axum` for SSR, `leptos-fluent/hydrate` for client.

```rust
leptos_fluent! {
    locales: "../../locales",
    set_language_to_cookie: true,
    initial_language_from_cookie: true,
    initial_language_from_navigator: true,
    initial_language_from_accept_language_header: true,
    cookie_name: "lang",
}
```

Change from current: localStorage → cookie for language persistence (SSR can't access localStorage).

### Migration approach

Route-by-route. Server functions and old gloo-net calls can coexist during transition.

## MCP (rmcp + oxide-auth)

### 5 tools (unchanged functionality)

```rust
pub struct KartotekaTools {
    pool: SqlitePool,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl KartotekaTools {
    #[tool(name = "list_lists", description = "List all lists")]
    async fn list_lists(
        &self,
        Extension(parts): Extension<http::request::Parts>,
    ) -> Result<CallToolResult, McpError> {
        let user_id = extract_user_id(&parts)?;
        let lists = domain::lists::list_all(&self.pool, &user_id).await?;
        Ok(CallToolResult::success(serde_json::to_value(lists)?))
    }
    // get_list_items, create_item, update_item, search_items — same pattern
}
```

User ID extracted per-request from `http::request::Parts` extensions (injected by bearer auth middleware). Not stored in struct.

### Locale resolution

Priority: user preferences (db) → Accept-Language header → "en" default.

### OAuth 2.1 flow (for MCP clients)

oxide-auth + oxide-auth-axum. PKCE required (S256), DCR, stateless tokens via HMAC-SHA256.

```
Claude Code → POST /mcp → 401
  → GET /.well-known/oauth-authorization-server
  → POST /oauth/register → client_id (DCR)
  → GET /oauth/authorize?code_challenge=... → consent page (Leptos SSR route)
    (no session → redirect /auth/login → back to consent)
  → approve → auth code → redirect
  → POST /oauth/token → access + refresh tokens
  → POST /mcp (Bearer token) → tools
```

### AppState

```rust
#[derive(FromRef, Debug, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: SqlitePool,
}
```

Provided to Leptos via `leptos_routes_with_context` and to Axum via `.with_state()`. Server functions access it via `expect_context::<AppState>()`.

### Mounting in Axum

```rust
let mcp_service = StreamableHttpService::new(
    move || Ok(KartotekaTools::new(pool.clone())),
    LocalSessionManager::default().into(),
    Default::default(),
);

let app = Router::new()
    .leptos_routes_with_context(&state, routes,
        move || provide_context(pool.clone()), frontend::shell)
    .nest("/api", api::routes(pool))
    .nest("/auth", auth::routes(pool))
    .nest_service("/mcp", mcp_service)
    .nest("/oauth", oauth::routes(pool))
    .route("/.well-known/oauth-authorization-server", get(oauth_metadata))
    .route("/.well-known/oauth-protected-resource", get(resource_metadata))
    .with_state(app_state);
```

## User timezone

Users can set their timezone in settings (e.g. `"Europe/Warsaw"`). Stored in `user_settings` as `timezone` key. Default: `"UTC"`.

Impact:
- **by-date / calendar queries:** "today" resolved using user's timezone, not server UTC. Domain layer converts user's "today" to UTC date range before querying.
- **Frontend:** dates displayed in user's timezone (chrono-tz for conversion).
- **MCP:** tool responses include dates in user's timezone.
- **chrono-tz** crate needed in shared/domain for timezone-aware date operations.

## REST API

Thin Axum wrappers on `domain::` functions. Same endpoints as current (`/api/lists`, `/api/containers/:id`, etc.). Auth via `AuthSession` extractor. Not used by frontend (server functions), but available for external integrations, MCP Inspector, curl.

```rust
async fn list_all(
    State(pool): State<SqlitePool>,
    auth: AuthSession<AuthBackend>,
) -> Result<Json<Vec<List>>, AppError> {
    let user = auth.user.ok_or(AppError::Unauthorized)?;
    Ok(Json(domain::lists::list_all(&pool, &user.id).await?))
}
```

## Deploy

- **VPS:** Mikrus 4GB, existing Caddy reverse proxy
- **Binary:** `x86_64-unknown-linux-musl` (static linking)
- **Process:** systemd unit, `Restart=always`
- **SSL:** Caddy auto-SSL on custom domain
- **Backup:** cron `sqlite3 data.db '.backup ...'` daily
- **CI/CD:** GitHub Actions → cargo leptos build → scp + systemctl restart

### Env vars on VPS

```
DATABASE_URL=sqlite:///opt/kartoteka/data.db
PORT=3000
SESSION_SECRET=<random 64 bytes>
OAUTH_SIGNING_SECRET=<random 64 bytes>
BASE_URL=https://kartoteka.yourdomain.pl
```

## Dev Experience

```bash
cargo leptos watch   # one process, hot reload server + client
                     # replaces: just dev (3 processes)
```

SQLite file locally, zero external dependencies. Full auth flow works locally.

## Tech Stack Summary

| Component | Crate | Purpose |
|-----------|-------|---------|
| HTTP framework | axum 0.8 | Routing, extractors, middleware |
| Frontend | Leptos 0.8 SSR | Components, server functions, hydration |
| Build tool | cargo-leptos | Dual-target build (server + WASM) |
| Database | sqlx + SQLite | Async, compile-time checked queries |
| Auth sessions | axum-login 0.18 + tower-sessions 0.15 | Session management, auth middleware |
| Password hashing | argon2 | Secure password storage |
| 2FA | totp-rs 5.7 | TOTP generation + verification |
| MCP server | rmcp | StreamableHTTP + tool macros |
| OAuth server (MCP) | oxide-auth 0.6 + oxide-auth-axum | Auth code + PKCE + token signing |
| i18n | leptos-fluent 0.2 | Fluent translations, SSR support |
| Logging | tracing + tracing-subscriber | Structured logging |
| Static files | tower-http ServeDir | Serve WASM bundle |
| TLS | Caddy | Auto Let's Encrypt |

## Implementation Plans

5 independent plans, each gets its own spec + branch + PR:

| # | Plan | Depends on | Deliverable |
|---|------|-----------|-------------|
| 1 | DB layer | — | `crates/db` + `crates/shared` refactor + migration + tests |
| 2 | Axum skeleton + auth | 1 | `crates/server` — binary, login/register/2FA, sessions, REST placeholders |
| 3 | Frontend SSR | 1, 2 | `crates/frontend` — Leptos 0.8 SSR, server functions, route-by-route migration |
| 4 | MCP + OAuth | 1, 2 | `crates/mcp` — rmcp tools, oxide-auth, consent page, DCR |
| 5 | Deploy | 1-4 | Caddy config, systemd, CI/CD, backup |

Plans are sequential: 1+1a → 2 → 3 → 4 → 5. Plan 4 depends on Plan 3 (consent page is a Leptos SSR route).

## What's NOT Changing

- UI/UX: same pages, same components, same DaisyUI styling
- MCP tools: same 5 tools, same functionality
- i18n: same FTL files, same PL/EN support
- Data model: same tables, same relations

## Security hardening (implement during relevant plans)

- Rate limiting on `/auth/login`, `/auth/register`, `/oauth/register`, `/oauth/token` (tower-governor or custom middleware)
- CSRF token on OAuth consent form POST
- OAuth `state` parameter for authorization endpoint CSRF protection
- DCR abuse prevention (limit clients per IP or require auth)
- Refresh token rotation (new refresh token on each use, invalidate old)
- Token scope enforcement in MCP tools
- CORS policy configuration (allowed origins for REST API)
- Expired OAuth authorization codes cleanup (background task like session cleanup)

## Future (out of scope)

- GitHub OAuth as additional auth method
- Passkeys/WebAuthn
- Personal access tokens / bearer tokens for REST API
- Data migration from D1 (clean cut for now)
