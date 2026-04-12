# Plan 4: MCP + OAuth — Design Spec

Parent: `docs/superpowers/specs/2026-04-12-cloudflare-exit-v2-design.md`
Depends on: Plan 1+1a (db + domain), Plan 2 (auth), Plan 3 (frontend SSR)

## Goal

MCP server with 5 tools + OAuth 2.1 provider for MCP client authentication. Replaces the TypeScript gateway (797 LOC). Uses rmcp for MCP transport, oxide-auth core for OAuth security primitives, custom Axum handlers for MCP-specific endpoints (DCR, metadata).

## Architecture

```
Claude Code → POST /mcp (Bearer token)
                ↓
        bearer_auth middleware
        (oxide-auth ResourceFlow, validates token, injects UserId to extensions)
                ↓
        StreamableHttpService (rmcp)
                ↓
        KartotekaTools (5 tools)
        (reads UserId from request Parts extensions, calls domain::)
```

OAuth flow (one-time setup per MCP client):
```
Claude Code → POST /mcp → 401
  → GET /.well-known/oauth-authorization-server → metadata
  → POST /oauth/register → client_id (DCR)
  → GET /oauth/authorize?code_challenge=...&redirect_uri=...
    → session check → no session? redirect /auth/login
    → login (email+password+2FA) → back to /oauth/authorize
    → Leptos SSR consent page → user approves
  → POST /oauth/authorize → generate auth code → redirect to client
  → POST /oauth/token → verify PKCE S256 → issue JWT (access + refresh)
  → POST /mcp (Bearer: access_token) → tools work
```

## MCP Tools (5, unchanged functionality)

Same 5 tools as current TypeScript gateway. All call `domain::` (never db:: directly).

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

    #[tool(name = "get_list_items", description = "Get items for a list")]
    async fn get_list_items(
        &self,
        Extension(parts): Extension<http::request::Parts>,
        Parameters(p): Parameters<ListIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let user_id = extract_user_id(&parts)?;
        let items = domain::items::list_all(&self.pool, &user_id, &p.list_id).await?;
        Ok(CallToolResult::success(serde_json::to_value(items)?))
    }

    // create_item, update_item, search_items — same pattern
}

fn extract_user_id(parts: &http::request::Parts) -> Result<String, McpError> {
    parts.extensions.get::<UserId>()
        .map(|u| u.0.clone())
        .ok_or_else(|| McpError::new("unauthorized"))
}
```

### User ID flow

rmcp `StreamableHttpService` injects `http::request::Parts` into tool handler extensions per-request. Axum bearer middleware validates OAuth token and inserts `UserId` into request extensions before rmcp sees it. Tool handlers extract `UserId` from `Parts.extensions`.

### i18n in tools

Tool descriptions and error messages use user's locale. Resolution priority:
1. User preferences from db (via `domain::preferences::get_locale`)
2. `Accept-Language` header from MCP request
3. Default: `"en"`

### Timezone

Date-related tool responses use user's timezone from settings (via `domain::` which resolves with chrono-tz).

## OAuth 2.1 Provider

### Stack

- **oxide-auth** (core) — authorization code grant engine, PKCE S256 verification, token signing (HMAC-SHA256)
- **Custom Axum handlers** — DCR, well-known metadata, authorize/token endpoints wrapping oxide-auth
- **Bearer middleware** — Axum `from_fn`, validates token via oxide-auth `ResourceFlow`, injects `UserId`

### Why oxide-auth core (not oxide-auth-axum)

oxide-auth-axum is opinionated and doesn't cover DCR or MCP-specific metadata. We use oxide-auth core directly (`Registrar`, `Authorizer`, `Issuer` traits) and write thin Axum handlers.

### Endpoints

| Endpoint | Method | Auth | Purpose |
|----------|--------|------|---------|
| `/.well-known/oauth-authorization-server` | GET | none | RFC 8414 metadata |
| `/.well-known/oauth-protected-resource` | GET | none | RFC 9728 protected resource metadata |
| `/oauth/register` | POST | none | Dynamic Client Registration (RFC 7591) |
| `/oauth/authorize` | GET | session | Show consent page (Leptos SSR route) |
| `/oauth/authorize` | POST | session | User approves → generate auth code → redirect |
| `/oauth/token` | POST | none | Exchange auth code + PKCE verifier → JWT tokens |
| `/mcp` | POST/GET/DELETE | Bearer | rmcp StreamableHttpService |

### Dynamic Client Registration (DCR)

~60 LOC Axum handler. Stores client in `oauth_clients` table:

```rust
async fn register_client(
    State(state): State<OAuthState>,
    Json(req): Json<ClientRegistrationRequest>,
) -> Result<Json<ClientRegistrationResponse>, AppError> {
    let client_id = uuid::Uuid::new_v4().to_string();
    db::oauth_clients::create(&state.pool, &client_id, &req).await?;
    Ok(Json(ClientRegistrationResponse { client_id, ... }))
}
```

### Tokens

- **Access token:** JWT signed with HMAC-SHA256 (`OAUTH_SIGNING_SECRET` env var). Contains `sub` (user_id), `client_id`, `exp`, `iat`. Stateless — no token table needed.
- **Refresh token:** Opaque random string stored in db, linked to user_id + client_id.
- **Authorization code:** Short-lived (5 min), stored in db with PKCE code_challenge, consumed on exchange.

### PKCE

oxide-auth handles PKCE S256 verification natively. Required on all authorization requests (MCP spec mandates PKCE).

### Consent page

Leptos SSR route rendered at `GET /oauth/authorize` (from Plan 3). Shows:
- Client name (from DCR registration)
- Requested scopes
- Approve / Deny buttons

POST approve → oxide-auth generates authorization code → redirect to client's `redirect_uri`.

Session check: if user not logged in, redirect to `/auth/login` with `return_to` in session pointing back to consent page URL.

## Crate structure

```
crates/mcp/
  Cargo.toml
  src/
    lib.rs              — re-exports
    tools.rs            — KartotekaTools, 5 tools, tool_router
    tools/              — per-tool params (schemars derives)
      lists.rs
      items.rs
      containers.rs
      tags.rs
      calendar.rs
    oauth/
      mod.rs            — OAuthState, routes()
      handlers.rs       — authorize, token, register, metadata endpoints
      bearer.rs         — bearer_auth_middleware (validates token, injects UserId)
      storage.rs        — oxide-auth Registrar/Authorizer/Issuer backed by db::
      types.rs          — request/response types for OAuth endpoints
```

### Dependencies

```toml
[dependencies]
kartoteka-shared = { path = "../shared" }
kartoteka-db = { path = "../db" }
kartoteka-domain = { path = "../domain" }
kartoteka-i18n = { path = "../i18n" }
rmcp = { version = "1", features = ["server", "transport-streamable-http-server", "macros"] }
oxide-auth = "0.6"
axum = "0.8"
sqlx = { version = "0.8", features = ["sqlite"] }
schemars = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
jsonwebtoken = "10"
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
```

## Mounting in server/main.rs

```rust
// crates/server/src/main.rs
let oauth_state = mcp::oauth::OAuthState::new(pool.clone(), signing_secret);

let mcp_config = StreamableHttpServerConfig::default()
    .with_allowed_hosts(vec![base_url_host])
    .with_cancellation_token(ct.child_token());

let mcp_service = StreamableHttpService::new(
    {
        let pool = pool.clone();
        move || Ok(KartotekaTools::new(pool.clone()))
    },
    Arc::new(LocalSessionManager::default()),
    mcp_config,
);

let mcp_routes = Router::new()
    .nest_service("/mcp", mcp_service)
    .layer(axum::middleware::from_fn_with_state(
        oauth_state.clone(),
        mcp::oauth::bearer::bearer_auth_middleware,
    ));

let app = Router::new()
    .leptos_routes_with_context(...)
    .nest("/api", api::routes(pool.clone()))
    .nest("/auth", auth::routes(pool.clone()))
    .merge(mcp_routes)
    .nest("/oauth", mcp::oauth::handlers::routes(oauth_state))
    .route("/.well-known/oauth-authorization-server", get(mcp::oauth::handlers::metadata))
    .route("/.well-known/oauth-protected-resource", get(mcp::oauth::handlers::resource_metadata))
    .layer(auth_layer)
    .layer(TraceLayer::new_for_http());
```

## DB tables (from Plan 1 migration)

Already included in consolidated migration:

```sql
CREATE TABLE oauth_clients (
    client_id TEXT PRIMARY KEY,
    client_name TEXT,
    redirect_uris TEXT NOT NULL,   -- JSON array
    grant_types TEXT NOT NULL,     -- JSON array
    token_endpoint_auth_method TEXT DEFAULT 'none',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;
```

Additional tables for OAuth state (added in Plan 4 migration):

```sql
CREATE TABLE oauth_authorization_codes (
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

CREATE TABLE oauth_refresh_tokens (
    token TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
    user_id TEXT NOT NULL REFERENCES users(id),
    scopes TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;
```

Access tokens are JWTs — no table needed.

## Testing

- **Tools:** integration tests with in-memory SQLite. Create user, create data, call tool, verify response.
- **OAuth flow:** integration test with Axum test app. Register client → authorize → exchange code → verify JWT → call /mcp with bearer.
- **Bearer middleware:** unit test — valid token → UserId extracted, invalid → 401.
- **PKCE:** unit test — S256 challenge/verifier pair, verification through oxide-auth.

## Migration from current gateway

1. Current gateway TS tools → rmcp Rust tools (same 5 tools, same functionality)
2. `@cloudflare/workers-oauth-provider` → oxide-auth core + custom handlers
3. Better Auth session check on consent → axum-login session check
4. Gateway proxy to API Worker → direct `domain::` calls (zero HTTP overhead)
5. MCP Inspector test to verify flow works end-to-end

## What this plan does NOT include

- Deploy (Plan 5)
- Additional MCP tools (future)
- MCP resources or prompts (future MCP spec features)
