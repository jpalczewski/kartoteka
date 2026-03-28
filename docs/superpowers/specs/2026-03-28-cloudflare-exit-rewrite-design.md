# Kartoteka Rewrite: Cloudflare Exit

## Motivation

`@cloudflare/workers-oauth-provider` exceeds Cloudflare Workers free tier 10ms CPU limit (AES-KW key wrapping, 6x crypto.subtle per OAuth flow). Better Auth's scrypt adds ~2000ms CPU for password hashing. No configuration can fix this — the crypto is mandatory in the library.

We exit Cloudflare entirely: one Rust binary, one language, zero vendor lock-in. Hosted on Mikrus Frog (free LXC VPS).

## Target Architecture

Single Cargo workspace producing **one binary** with both API and MCP on the same port. Runs on Mikrus Frog (Alpine LXC, 256MB RAM, 3GB disk, shared IPv4 with 3 ports).

```
                    ┌──────────────────┐
  Browser ─────────►│  Mikrus reverse  │◄── auto HTTPS via wykr.es
  Claude Code ─────►│  proxy (port N)  │
                    └────────┬─────────┘
                             │
                      :PORT (HTTP)
                             ▼
                    ┌──────────────────┐
                    │   kartoteka      │
                    │   (one binary)   │
                    │                  │
                    │  /api/*   → REST │
                    │  /mcp    → MCP   │
                    │  /auth/* → OAuth │
                    │  /*      → SPA   │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │  SQLite data.db  │
                    └──────────────────┘
```

Mikrus provides auto HTTPS via `frogXX-PORT.wykr.es` subdomain — no Caddy, no nginx, no TLS config.
Custom domain possible via DNS CNAME if user has one.

### Why One Binary (Not Two)

- Mikrus has only 3 TCP ports on shared IPv4 — two binaries wastes a port
- 256MB RAM — two processes is wasteful overhead
- Single process = one SQLitePool, no cross-process locking issues, no WAL concerns
- Crate structure still supports splitting later (shared `db/`, `models/`, etc.)

When outgrowing Mikrus → paid VPS → split into two binaries without code changes.

## Cargo Workspace Structure

```
crates/
  models/        — pure types, DTOs, enums. Zero framework deps.
  db/            — sqlx queries + migrations. Shared by all.
  auth/          — GitHub OAuth client, session middleware, user management
  oauth-server/  — MCP OAuth provider (oxide-auth + PKCE + DCR)
  server/        — single Axum binary: API + MCP + auth + static frontend
  frontend/      — Leptos CSR (unchanged, trunk build → static files)
```

### Dependency Graph

```
models              (serde, schemars)
   │
   ▼
db                  (models, sqlx + SQLite)
   │
   ├────────┬───────────┐
   ▼        ▼           ▼
auth     oauth-server  server
(db,     (db,          (db, auth, oauth-server,
 oauth2,  oxide-auth,   rmcp, axum, tower-sessions,
 axum)    oxide-auth-   tower-http)
          axum)
```

No circular dependencies. Each crate has one responsibility.

### Crate Details

#### `kartoteka-models`
Pure data types. No I/O, no framework deps.
- `Container`, `List`, `Item`, `Tag` structs
- `CreateListRequest`, `UpdateItemRequest` DTOs
- `ContainerStatus`, `ListType` enums
- `ListFeature`, `DateItem`, `DaySummary`, `DayItems`
- Serde derives + `schemars::JsonSchema` (for rmcp tool params)

Note: D1-specific hacks (`bool_from_number`, `u32_from_number`) are dropped. sqlx maps types correctly.

#### `kartoteka-db`
Database access layer. All queries live here — no duplication between API handlers and MCP tools.
```rust
// db/src/lists.rs
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<List>> { ... }
pub async fn create(pool: &SqlitePool, user_id: &str, req: &CreateListRequest) -> Result<List> { ... }
pub async fn get_one(pool: &SqlitePool, user_id: &str, id: &str) -> Result<Option<List>> { ... }
// ... same pattern for containers, items, tags
```
- sqlx compile-time checked queries
- Migrations in `db/migrations/`
- One consolidated migration (fresh schema, not 10 incremental D1 migrations)

#### `kartoteka-auth`
GitHub OAuth login + session management.
- `oauth2` crate for GitHub OAuth flow (redirect → callback → create user)
- `tower-sessions` with `SqliteStore` for session cookies
- Middleware: extracts `UserId` from session, rejects unauthenticated
- User CRUD in db crate (`db::users::find_or_create_by_github`)

Data model (future-proof for multiple auth methods):
```sql
users (id, email, name, avatar_url, created_at, updated_at)
auth_methods (id, user_id, provider, provider_id, credential, created_at)
```

#### `kartoteka-oauth-server`
MCP OAuth 2.1 provider using oxide-auth + oxide-auth-axum.
- Authorization code grant with PKCE required (`Pkce::required()`, S256 only)
- oxide-auth handles: authorization, token exchange, PKCE verification, refresh tokens
- oxide-auth `TokenSigner` with HMAC-SHA256 for stateless tokens (no token table needed)
- Dynamic Client Registration (~60 LOC manual endpoint, stores in `oauth_clients` table)
- Well-known endpoints: `/.well-known/oauth-authorization-server` (RFC 8414)
- Links MCP token to `users.id` via authorize flow (checks session from `kartoteka-auth`)

#### `kartoteka-server` (single binary)
One Axum process, all routes:
```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::init();

    let pool = SqlitePool::connect(&db_url).await?;
    sqlx::migrate!().run(&pool).await?;

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await?;

    // MCP server (rmcp)
    let mcp_service = StreamableHttpService::new(
        move || Ok(KartotekaTools::new(pool.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let app = Router::new()
        // REST API
        .nest("/api", api_routes(pool.clone()))
        // GitHub OAuth login
        .nest("/auth", auth::routes(pool.clone()))
        // MCP server
        .nest_service("/mcp", mcp_service)
        // MCP OAuth endpoints
        .nest("/oauth", oauth_server::routes(pool.clone()))
        // Well-known
        .route("/.well-known/oauth-authorization-server", get(oauth_metadata))
        .route("/.well-known/oauth-protected-resource", get(resource_metadata))
        // Frontend static files (Leptos CSR)
        .fallback_service(ServeDir::new("static"))
        // Middleware
        .layer(session_layer)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on {}", addr);
    axum::serve(listener, app).await?;
}
```

Serves on a single port (one of the 3 Mikrus ports). Mikrus reverse proxy handles HTTPS.

#### `kartoteka-frontend`
Unchanged. `trunk build --release` produces static files. Served by server binary via `ServeDir`.

Only change: `API_BASE_URL="/api"` (same origin, no CORS issues).

## MCP Tools (5, same as current)

Defined via rmcp proc macros in the server crate:
```rust
#[tool_router]
impl KartotekaTools {
    #[tool(description = "List all lists for the user")]
    async fn list_lists(&self, ...) -> Result<Json<Vec<List>>, McpError> {
        let lists = db::lists::list_all(&self.pool, &user_id).await?;
        Ok(Json(lists))
    }
    // get_list_items, create_item, update_item, search_items
}
```

Both MCP tools and API handlers call the same `db::` functions. Zero duplication.

## Auth Flows

### Web App (GitHub OAuth)
```
Browser → GET /auth/github → redirect to github.com/login/oauth/authorize
GitHub → GET /auth/github/callback?code=xxx → exchange code → create/find user → set session cookie → redirect /
```

### MCP (OAuth 2.1 + PKCE)
```
Claude Code → POST /mcp → 401 (WWW-Authenticate header)
Claude Code → GET /.well-known/oauth-authorization-server → metadata (endpoints, PKCE support)
Claude Code → POST /oauth/register → client_id (DCR)
Claude Code → GET /oauth/authorize?code_challenge=...&code_challenge_method=S256 → consent page
    (if no session → redirect to /auth/github first, then back to consent)
User → approve → POST /oauth/authorize → auth code → redirect to Claude callback
Claude Code → POST /oauth/token → exchange code + code_verifier (PKCE verify via oxide-auth) → access token + refresh token
Claude Code → POST /mcp (Bearer token) → MCP tools
```

The authorize endpoint checks the same session cookie as the web app. User logs in once (GitHub), authorizes MCP separately.

## Database Schema

One SQLite file (`data.db`), one consolidated migration:

```sql
-- Auth
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    name TEXT,
    avatar_url TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE auth_methods (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    provider TEXT NOT NULL,        -- 'github', future: 'password', 'passkey'
    provider_id TEXT NOT NULL,     -- GitHub user ID
    credential TEXT,               -- future: argon2 hash, passkey public key
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(provider, provider_id)
);

-- MCP OAuth server (DCR clients only — tokens are stateless via oxide-auth TokenSigner)
CREATE TABLE oauth_clients (
    client_id TEXT PRIMARY KEY,
    client_name TEXT,
    redirect_uris TEXT NOT NULL,   -- JSON array
    grant_types TEXT NOT NULL,     -- JSON array
    token_endpoint_auth_method TEXT DEFAULT 'none',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- App data (migrated from D1, same schema minus D1 quirks)
CREATE TABLE containers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    description TEXT,
    status TEXT,                   -- NULL=folder, 'active'/'done'/'paused'=project
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
    config TEXT NOT NULL DEFAULT '{}',  -- JSON
    PRIMARY KEY (list_id, feature_name)
);

-- sessions table auto-created by tower-sessions SqliteStore::migrate()
```

## Deployment

### Mikrus Frog Setup (one-time)
```bash
# On VPS (Alpine LXC)
apk add sqlite
mkdir -p /opt/kartoteka

# Binary runs on one of the 3 allocated ports (e.g., 20XXX)
# Mikrus auto-proxies HTTPS: frogXX-20XXX.wykr.es → localhost:20XXX

# OpenRC service (/etc/init.d/kartoteka)
#!/sbin/openrc-run
command="/opt/kartoteka/kartoteka-server"
command_args=""
command_background=true
pidfile="/run/kartoteka.pid"
directory="/opt/kartoteka"
output_log="/var/log/kartoteka.log"
error_log="/var/log/kartoteka.err"
```

No Caddy, no nginx — Mikrus handles TLS automatically via `wykr.es` subdomain.

### CI/CD (GitHub Actions)
```yaml
name: Deploy
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/cache@v4
        with:
          path: target
          key: cargo-${{ hashFiles('Cargo.lock') }}

      - name: Build
        run: |
          rustup target add x86_64-unknown-linux-musl
          cargo build --release --target x86_64-unknown-linux-musl -p kartoteka-server

      - name: Build frontend
        run: |
          cd crates/frontend && trunk build --release

      - name: Deploy
        run: |
          scp target/x86_64-unknown-linux-musl/release/kartoteka-server user@$VPS:~/kartoteka-new
          scp -r crates/frontend/dist/* user@$VPS:/opt/kartoteka/static/
          ssh user@$VPS 'mv ~/kartoteka-new /opt/kartoteka/kartoteka-server && rc-service kartoteka restart'
        env:
          VPS: ${{ secrets.VPS_HOST }}
```

Build target: `x86_64-unknown-linux-musl` (static linking, no glibc needed on Alpine).

### Environment Variables (on VPS)
```
DATABASE_URL=sqlite:///opt/kartoteka/data.db
GITHUB_CLIENT_ID=xxx
GITHUB_CLIENT_SECRET=xxx
BASE_URL=https://frogXX-PORT.wykr.es
PORT=20XXX
SESSION_SECRET=xxx              # random 64 bytes
OAUTH_SIGNING_SECRET=xxx        # for oxide-auth TokenSigner HMAC
```

## Migration Path from Cloudflare

1. Export D1 data (`wrangler d1 export kartoteka-db --output dump.sql`)
2. Create new SQLite with consolidated schema
3. Import app data (containers, lists, items, tags, features, tag links)
4. Create initial user + auth_method from existing data
5. Deploy binary to Mikrus
6. Configure GitHub OAuth app with new callback URL (`https://frogXX-PORT.wykr.es/auth/github/callback`)
7. Verify MCP flow with MCP Inspector
8. Update Claude Code MCP config to new URL
9. Decommission Cloudflare Workers + D1 + Pages

## What's NOT Changing

- Frontend: Leptos CSR, same components, same UX
- API endpoints: same paths, same request/response shapes
- MCP tools: same 5 tools, same functionality
- User-facing behavior: identical

## What IS Changing

| Before (Cloudflare) | After (Mikrus) |
|---------------------|----------------|
| 2 Workers (Rust API + TS Gateway) | 1 Rust binary |
| D1 (SQLite-like) | SQLite file |
| Better Auth (TS, scrypt) | GitHub OAuth (`oauth2` crate) |
| `@cloudflare/workers-oauth-provider` (AES-KW) | `oxide-auth` (HMAC-SHA256, PKCE) |
| TypeScript Gateway | gone |
| Two languages (Rust + TS) | one (Rust) |
| `worker` crate | `axum` |
| Service bindings | shared SqlitePool |
| Wrangler deploy | GitHub Actions + scp |
| CF Pages (frontend hosting) | ServeDir from binary |
| 10ms CPU limit | no limit |
| $5/month (if paid) | free (5 PLN one-time) |

## Tech Stack Summary

| Component | Crate | Purpose |
|-----------|-------|---------|
| HTTP framework | `axum` 0.8 | Routing, extractors, middleware |
| MCP server | `rmcp` 1.3 | StreamableHTTP + tool macros |
| OAuth server (MCP) | `oxide-auth` 0.6 + `oxide-auth-axum` 0.6 | Auth code + PKCE + token signing |
| GitHub login | `oauth2` | OAuth client for user auth |
| Sessions | `tower-sessions` + `tower-sessions-sqlx-store` | Cookie sessions in SQLite |
| Database | `sqlx` + SQLite | Async, compile-time checked |
| Serialization | `serde` + `serde_json` | Models |
| Schema gen | `schemars` | rmcp tool parameters |
| Logging | `tracing` + `tracing-subscriber` | Structured logging |
| Static files | `tower-http` (`ServeDir`) | Serve frontend |
| Frontend | Leptos 0.7 CSR | WASM, built with trunk |
| TLS | Mikrus reverse proxy | Auto HTTPS via wykr.es |

## Scaling Path

| Stage | Setup | Effort |
|-------|-------|--------|
| Now | 1 binary, 1 SQLite, Mikrus Frog | this rewrite |
| Growing | same binary on paid Mikrus/VPS (more RAM/disk) | `scp` binary |
| Multi-user (C) | split into 2 binaries (api + mcp), add email+password (argon2) + passkeys | extract binary targets, same crates |
| Scale | multiple api instances behind nginx, MCP separate | add nginx, WAL mode on SQLite |
| Big scale | switch SQLite → Postgres in `db` crate | change sqlx feature flag + queries |
