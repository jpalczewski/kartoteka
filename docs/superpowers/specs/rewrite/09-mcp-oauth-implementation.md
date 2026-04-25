# MCP + OAuth Implementation — Design Spec

Parent: `00-overview.md`
Supersedes (implementation detail): `05-mcp.md`, `06-oauth.md`
Scope: combines F1 (MCP tools/resources) and F2 (OAuth 2.1 provider) into a single PR.

## Goal

Ship a production-grade MCP server with full OAuth 2.1 authorization (DCR, PKCE S256, refresh token rotation). Claude Code talks to `/mcp` with a bearer JWT obtained via the standard OAuth flow — no dev-mode shortcut that would later need to be replaced.

Single PR because (a) OAuth without MCP is unused, (b) MCP without OAuth requires a throwaway bearer middleware, and (c) the user explicitly preferred one large, well-thought PR over two smaller ones.

## Architecture

### Request flow (end-to-end)

```
Claude Code → POST /mcp → 401 + WWW-Authenticate
  → GET /.well-known/oauth-authorization-server        (no auth, metadata)
  → POST /oauth/register                                (DCR → client_id)
  → GET /oauth/authorize                                (validates params, redirects)
      → (no session) 302 /auth/login?return_to=...
      → (session) 302 /oauth/consent                   (Leptos SSR page)
          → form POST /oauth/authorize                 (CSRF + decision)
          → 302 <redirect_uri>?code=...&state=...
  → POST /oauth/token  (code + PKCE verifier)          → JWT access + refresh
  → POST /mcp (Bearer JWT)                              → bearer middleware → tools
```

### Crate layout

```
crates/oauth/                 NEW
  src/
    lib.rs          OAuthState, routes(), well_known_routes()
    handlers.rs     authorize_get/post, token, register, metadata_as, metadata_pr
    bearer.rs       bearer_auth_middleware (JWT → UserId + UserLocale in extensions)
    storage.rs      db-backed helpers (client lookup, code consume, refresh rotate)
    types.rs        DCR req/res, metadata structs, PendingOAuthRequest, ConsentData
    errors.rs       OAuthError → axum IntoResponse
    pkce.rs         verify_s256 helper + unit tests

crates/mcp/                   FILLED IN (currently stub)
  src/
    lib.rs          McpError, extract_user_id, extract_locale, re-exports
    server.rs       KartotekaServer struct, #[tool_router], ServerHandler impl
    tools/
      mod.rs
      items.rs      CreateItemParams, UpdateItemParams
      search.rs     SearchItemsParams
      comments.rs   AddCommentParams
      relations.rs  AddRelationParams, RemoveRelationParams
      time.rs       StartTimerParams, StopTimerParams, LogTimeParams
      templates.rs  CreateListFromTemplateParams, SaveAsTemplateParams
    resources.rs    ResourceUri enum + parser, list_resources, read_resource
    i18n_errors.rs  DomainError → localized ErrorData

crates/db/src/oauth.rs        NEW MODULE
  clients::{create, find}
  codes::{insert, consume, cleanup_expired}
  refresh::{insert, find_and_delete, cleanup_expired}

crates/shared/src/auth_ctx.rs NEW
  UserId(String), UserLocale(String)   — newtypes for request extensions

crates/frontend-v2/src/pages/oauth_consent.rs   NEW
  OAuthConsentPage component + load_consent_data server fn

crates/i18n/locales/{en,pl}.ftl              EXTENDED
  mcp-tool-*-desc, mcp-err-*, oauth-consent-*, mcp-res-*-desc
```

### Why custom OAuth (not oxide-auth)

We evaluated oxide-auth. It does not cover DCR (RFC 7591) or MCP `.well-known` metadata (we write those regardless), and its `Registrar`/`Authorizer`/`Issuer` traits are sync — we'd adapt to async SQLx via blocking workarounds. The security-critical surface is ~80 LOC (PKCE verify, JWT sign/verify, single-use code consume in a transaction, refresh rotation in a transaction). Each is small, testable, and reviewable. A custom implementation has lower total complexity than an adapter layer over oxide-auth.

### Why avoid cyclic crate deps

`oauth/` injects `UserId` + `UserLocale` into request extensions; `mcp/` reads them. Placing the newtypes in `shared/` prevents `oauth ↔ mcp` coupling. `mcp/` does not depend on `oauth/`.

## OAuth provider

### Endpoints

| Method | Path | Auth | Purpose |
|---|---|---|---|
| GET | `/.well-known/oauth-authorization-server` | none | RFC 8414 |
| GET | `/.well-known/oauth-protected-resource` | none | RFC 9728 |
| POST | `/oauth/register` | none | DCR (RFC 7591) |
| GET | `/oauth/authorize` | session | Validate + redirect to consent |
| POST | `/oauth/authorize` | session | Consent decision → code |
| POST | `/oauth/token` | none | Exchange code/refresh → JWT |
| GET | `/oauth/consent` | session | Leptos SSR consent page |

### Metadata

```json
GET /.well-known/oauth-authorization-server
{
  "issuer": "https://<PUBLIC_BASE_URL>",
  "authorization_endpoint": "https://<host>/oauth/authorize",
  "token_endpoint": "https://<host>/oauth/token",
  "registration_endpoint": "https://<host>/oauth/register",
  "response_types_supported": ["code"],
  "grant_types_supported": ["authorization_code", "refresh_token"],
  "code_challenge_methods_supported": ["S256"],
  "token_endpoint_auth_methods_supported": ["none"],
  "scopes_supported": ["mcp"]
}

GET /.well-known/oauth-protected-resource
{
  "resource": "https://<host>/mcp",
  "authorization_servers": ["https://<host>"],
  "bearer_methods_supported": ["header"],
  "scopes_supported": ["mcp"]
}
```

Absolute URLs derived from env var `PUBLIC_BASE_URL` (fallback `http://localhost:3000` in dev).

### DCR (`POST /oauth/register`)

Request:
```json
{
  "client_name": "Claude Code",
  "redirect_uris": ["http://localhost:33418/oauth/callback"],
  "token_endpoint_auth_method": "none",
  "grant_types": ["authorization_code", "refresh_token"],
  "response_types": ["code"]
}
```

Validation:
- `redirect_uris` non-empty, each must parse as absolute URL
- `token_endpoint_auth_method` must be `"none"` (public clients only, PKCE required)
- `grant_types` subset of `["authorization_code", "refresh_token"]`

Insert into `oauth_clients` with generated UUID `client_id`, `client_secret = NULL`. Echo back submitted metadata plus `client_id`. Rate limiting deferred to F4.

### `GET /oauth/authorize`

Query params: `client_id, redirect_uri, response_type=code, code_challenge, code_challenge_method=S256, scope, state`.

1. **Strict validation** (reject with 400 `invalid_request`):
   - `response_type == "code"`
   - `code_challenge_method == "S256"` — no plain (OAuth 2.1, decision 7)
   - `code_challenge` non-empty, 43–128 chars, base64url charset
   - `scope == "mcp"` (only supported scope in F1)
   - `state` present (CSRF on client side)
2. `client_id` must exist in `oauth_clients`; `redirect_uri` must match one of registered `redirect_uris` exactly.
3. If no session (axum-login `AuthSession::user` is `None`) → 302 to `/auth/login?return_to=<escaped-current-url>`.
4. Generate `csrf_token = base64url(32 random bytes)`.
5. Save to session key `pending_oauth_request`:
   ```rust
   struct PendingOAuthRequest {
       client_id: String,
       redirect_uri: String,
       scope: String,
       state: String,
       code_challenge: String,
       csrf_token: String,
       created_at: DateTime<Utc>,   // TTL 10 min enforced in authorize_post
   }
   ```
6. 302 to `/oauth/consent` (no query string).

### `GET /oauth/consent` (Leptos SSR)

Server fn `load_consent_data()` reads session `pending_oauth_request`, looks up client name from `db::oauth::clients::find`, returns `{ client_name, scope, csrf_token }`. Page renders via DaisyUI card. Form is a **native `<form method="post" action="/oauth/authorize">`** — no JS required, no server fn for submit. Two submit buttons with `name="decision"` values `approve`/`deny`. Hidden input `csrf_token`.

FTL keys: `oauth-consent-title`, `oauth-consent-client-requests`, `oauth-consent-scope-label`, `oauth-consent-warning`, `oauth-consent-approve`, `oauth-consent-deny`, `oauth-consent-scope-mcp`.

### `POST /oauth/authorize`

Form body: `decision`, `csrf_token`.

1. Load `pending_oauth_request` from session. Missing → 400.
2. Age check: `now - created_at > 10 min` → 400 `request_expired`.
3. CSRF: `subtle::ConstantTimeEq(form_csrf, session_csrf)`. Mismatch → 400.
4. `decision == "deny"` → 302 to `redirect_uri?error=access_denied&state=<state>`, clear session key.
5. `decision == "approve"`:
   - Generate auth code `base64url(32 random bytes)`.
   - Insert `oauth_authorization_codes` row: `code, client_id, user_id (from session), code_challenge, scope, redirect_uri, expires_at = now + 5 min, used = 0`.
   - Clear session `pending_oauth_request` + `csrf_token`.
   - 302 to `redirect_uri?code=<code>&state=<state>`.

No persisted consent (decision 6 — ask every time).

### `POST /oauth/token`

Content-type: `application/x-www-form-urlencoded`.

**`grant_type=authorization_code`** body: `grant_type, code, redirect_uri, client_id, code_verifier`.

1. Lookup `oauth_authorization_codes` by `code` AND `client_id` in a transaction with `UPDATE ... SET used = 1 WHERE used = 0 RETURNING *`. No row returned → 400 `invalid_grant` (covers unknown, replayed, or wrong-client codes).
2. `expires_at > now` else 400 `invalid_grant`.
3. `redirect_uri` matches row else 400 `invalid_grant`.
4. **PKCE S256**: `base64url_nopad(sha256(code_verifier)) == code_challenge`. Constant-time compare. Mismatch → 400 `invalid_grant`.
5. Build access token JWT (HS256, secret from env `OAUTH_SIGNING_SECRET`, min 32 chars):
   ```json
   { "sub": user_id, "scope": "mcp", "jti": uuid4(),
     "iat": now, "exp": now + 3600 }
   ```
6. Build refresh token: `base64url(32 random bytes)`. Store `sha256(refresh)` in `oauth_refresh_tokens` with `client_id, user_id, scope, expires_at = now + 30 days`.
7. Respond 200:
   ```json
   { "access_token": "<jwt>", "token_type": "Bearer",
     "expires_in": 3600, "refresh_token": "<opaque>", "scope": "mcp" }
   ```

**`grant_type=refresh_token`** body: `grant_type, refresh_token, client_id`.

1. `sha256(refresh_token)` → `SELECT + DELETE` the row atomically (`DELETE ... RETURNING *`). Missing → 400 `invalid_grant` (covers unknown, reused, expired).
2. Row `client_id` matches body `client_id` else 400 `invalid_grant`.
3. `expires_at > now` else 400.
4. Generate **new** access token (fresh `jti`) and **new** refresh token. The new refresh keeps the **original absolute `expires_at`** — rotation does not extend lifetime (decision 9 — max 30 days from initial grant).
5. Insert new refresh into `oauth_refresh_tokens`. Respond same shape as auth-code grant.

Error response shape (all grants): `{ "error": "invalid_grant", "error_description": "..." }` per RFC 6749 §5.2. Always `Content-Type: application/json`.

### Bearer middleware (`crates/oauth/src/bearer.rs`)

```rust
pub async fn bearer_auth_middleware<B>(
    State(state): State<OAuthState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode>
```

1. Extract `Authorization: Bearer <jwt>`; missing → 401 with `WWW-Authenticate: Bearer resource_metadata="<host>/.well-known/oauth-protected-resource"`.
2. `jsonwebtoken::decode::<Claims>` with HS256 and shared secret. Invalid signature / expired → 401.
3. `claims.scope == "mcp"` → otherwise 403. (Personal/calendar tokens have other scopes and are rejected here; they're handled by a separate middleware on `/api/*`.)
4. Short-lived (1 h) → no jti revocation check.
5. Insert `UserId(claims.sub)` into `req.extensions_mut()`.
6. Parse `Accept-Language`, take first component, lowercase language subtag only, whitelist `pl`/`en`, fallback `"en"`. Insert `UserLocale(lang)`.
7. Call `next.run(req).await`.

## MCP server

### `KartotekaServer`

```rust
pub struct KartotekaServer {
    pool: SqlitePool,
    i18n: kartoteka_i18n::I18nLoader,
    tool_router: ToolRouter<Self>,
}

impl KartotekaServer {
    pub fn new(pool: SqlitePool, i18n: I18nLoader) -> Self {
        Self { pool, i18n, tool_router: Self::tool_router() }
    }
}

#[rmcp::tool_router]   // no `server_handler` — we override ServerHandler manually
impl KartotekaServer { /* 11 tools */ }

impl ServerHandler for KartotekaServer {
    async fn list_tools(&self, _req, ctx) -> Result<ListToolsResult, ErrorData>;
    async fn call_tool(&self, req, ctx)   -> Result<CallToolResult, ErrorData>;
    async fn list_resources(&self, _req, ctx) -> Result<ListResourcesResult, ErrorData>;
    async fn list_resource_templates(&self, _req, ctx) -> Result<ListResourceTemplatesResult, ErrorData>;
    async fn read_resource(&self, req, ctx) -> Result<ReadResourceResult, ErrorData>;
}
```

### Extension access pattern

Per rmcp: tools use `Extension(parts): Extension<http::request::Parts>` to read `req.extensions` including what the bearer middleware injected. `ServerHandler` overrides receive `RequestContext` which also exposes `extensions` directly.

```rust
pub(crate) fn extract_user_id(parts: &http::request::Parts) -> Result<String, McpError> {
    parts.extensions.get::<UserId>().map(|u| u.0.clone()).ok_or(McpError::Unauthorized)
}

pub(crate) fn extract_locale(parts: &http::request::Parts) -> &str {
    parts.extensions.get::<UserLocale>().map(|l| l.0.as_str()).unwrap_or("en")
}
```

### `list_tools` — localized descriptions

Each `#[tool(description = "mcp-tool-<name>-desc")]` registers the **FTL key** as its description. `list_tools` override resolves keys per request:

```rust
async fn list_tools(&self, _req, ctx) -> Result<ListToolsResult, ErrorData> {
    let locale = ctx.extensions.get::<UserLocale>().map(|l| l.0.as_str()).unwrap_or("en");
    let mut tools = self.tool_router.list_all();
    for t in &mut tools {
        if let Some(key) = &t.description {
            t.description = Some(self.i18n.translate(locale, key).unwrap_or_else(|| key.clone()).into());
        }
    }
    Ok(ListToolsResult { tools, next_cursor: None })
}
```

### Tools — 11 total

Each tool: params struct in `tools/<module>.rs` (derives `Deserialize`, `schemars::JsonSchema`), handler in `server.rs` delegates to `domain::`. All handlers follow the same skeleton: extract user_id, extract locale, call domain, serialize response.

| # | Tool | Params | Domain call |
|---|---|---|---|
| 1 | `create_item` | `list_id, title, description?, start_date?, deadline?, hard_deadline?, start_time?, deadline_time?, quantity?, target_quantity?, unit?, estimated_duration?, tag_ids?` | `items::create` |
| 2 | `update_item` | `item_id` + all fields as `Option<Option<T>>` (outer Some = update, inner None = clear) | `items::update` |
| 3 | `search_items` | `query, limit? (default 50, max 200), cursor?` | `search::search_items_paginated` |
| 4 | `add_comment` | `entity_type (item/list/container), entity_id, content, persona?` — `author_type` hardcoded `"assistant"` | `comments::create` |
| 5 | `add_relation` | `from_item_id, to_item_id, kind (blocks/relates_to)` | `relations::create` |
| 6 | `remove_relation` | same as 5 | `relations::delete` |
| 7 | `start_timer` | `item_id` | `time_entries::start_timer` |
| 8 | `stop_timer` | *(none)* | `time_entries::stop_timer` |
| 9 | `log_time` | `item_id?, started_at, duration_minutes, note?` | `time_entries::log_manual` |
| 10 | `create_list_from_template` | `template_ids: Vec<String>, container_id?, name?` | `templates::create_list_from_templates` |
| 11 | `save_as_template` | `list_id, template_name` | `templates::create_from_list` |

### Resources — 8 total

#### Static

| URI | Returns |
|---|---|
| `kartoteka://lists` | Projection `[{id, name, container_id, pinned, archived, item_count}]` |
| `kartoteka://containers` | Projection `[{id, name, parent_id, status, pinned}]` |
| `kartoteka://today` | Full items due today (user timezone); cap 200 |
| `kartoteka://time/summary` | Aggregates: today total, week total, per-list top 10 |

#### Templates (`list_resource_templates`)

| URI template (RFC 6570 Level 3) | Returns |
|---|---|
| `kartoteka://lists/{list_id}` | Full list object + features + item_count |
| `kartoteka://lists/{list_id}/items{?cursor,limit}` | `{ data: [Item; ≤limit], next_cursor, limit }` |
| `kartoteka://containers/{container_id}` | Container + children (projections) |
| `kartoteka://tags{?cursor}` | `{ data: [TagProjection], next_cursor }` |

#### Rationale — projections + pagination

Full list objects (with timestamps, descriptions) inflate context 5× vs. projections. For large collections (items in a list, tags if >200), we paginate via opaque base64 JSON cursors:

```rust
struct ItemsCursor { after_position: i32, after_id: String }  // tie-breaker
struct TagsCursor  { after_name: String, after_id: String }
struct SearchCursor { after_rank: f64, after_rowid: i64 }
```

SQL: `WHERE (sort_col, id) > (?, ?) ORDER BY sort_col, id LIMIT ?`. Cursor produced from the last row of a page; `next_cursor = None` when fewer than `limit` rows returned.

Pagination lives in `domain::` with a central `clamp_limit(opt) -> u32` helper (default 100, max 500). Both MCP and REST consumers share the cap.

#### URI parser

```rust
enum ResourceUri {
    Lists,
    ListDetail(String),
    ListItems { list_id: String, cursor: Option<String>, limit: Option<u32> },
    Containers,
    ContainerDetail(String),
    Tags { cursor: Option<String> },
    Today,
    TimeSummary,
}
```

Parser splits on first `?`, then on `/` for path segments. Unknown scheme/path → `ParseError` → localized `ErrorData::invalid_params`.

#### `read_resource`

Single entry point. Matches on `ResourceUri`, calls the appropriate `domain::` function, serializes via `serde_json`, returns `ResourceContents::text(uri, json, "application/json")` (not `blob` — JSON stays human-readable in transport).

`kartoteka://today` resolves "today" in user timezone: reads `user_settings.timezone` via `db::preferences`, applies `chrono_tz` to `Utc::now()`, passes `NaiveDate` to `domain::items::by_date`.

### Domain layer additions

New pub functions (no db:: internals leak):

```rust
// domain::lists
pub async fn list_projections(pool, user_id) -> Result<Vec<ListProjection>>;

// domain::containers
pub async fn list_projections(pool, user_id) -> Result<Vec<ContainerProjection>>;

// domain::tags
pub async fn list_projections_paginated(pool, user_id, cursor, limit) -> Result<PagedTags>;

// domain::items
pub async fn list_for_list_paginated(pool, user_id, list_id, cursor, limit) -> Result<PagedItems>;

// domain::search
pub async fn search_items_paginated(pool, user_id, query, cursor, limit) -> Result<PagedSearch>;

// domain::time_entries
pub async fn summary(pool, user_id) -> Result<TimeSummary>;  // today/week/per-list
```

Paged wrappers:
```rust
#[derive(Serialize)]
pub struct Paged<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub limit: u32,
}
```

### Error localization

`DomainError` → localized MCP `ErrorData`:

```rust
impl KartotekaServer {
    fn domain_err(&self, e: DomainError, locale: &str) -> ErrorData {
        let (key, args): (&str, &[(&str, &str)]) = match &e {
            DomainError::NotFound(kind)     => ("mcp-err-not-found", &[("entity", kind)]),
            DomainError::Validation(reason) => ("mcp-err-validation", &[("reason", reason)]),
            DomainError::FeatureRequired(f) => ("mcp-err-feature-required", &[("feature", f)]),
            DomainError::Forbidden          => ("mcp-err-forbidden", &[]),
            _ => ("mcp-err-internal", &[]),
        };
        ErrorData::invalid_request(self.i18n.translate_args(locale, key, args), None)
    }
}
```

## Wiring in `crates/server`

```rust
// Additions to AppState
pub struct AppState {
    /* existing fields */
    pub oauth_state: OAuthState,
    pub i18n: I18nLoader,
}

// Router composition (additions)
Router::new()
    .nest("/.well-known", kartoteka_oauth::well_known_routes().with_state(oauth_state.clone()))
    .nest("/oauth", kartoteka_oauth::routes().with_state(oauth_state.clone()))
    .nest_service("/mcp",
        ServiceBuilder::new()
            .layer(from_fn_with_state(oauth_state.clone(), bearer_auth_middleware))
            .service(mcp_service),
    )
    // existing /auth, /api, /leptos, Leptos routes (incl. /oauth/consent), fallback
    .layer(auth_layer)
    .with_state(state)
```

`main.rs` gains `I18nLoader::load()` at startup and passes it to `router()`. `OAUTH_SIGNING_SECRET` and `PUBLIC_BASE_URL` are required env vars in prod.

## Database

All OAuth tables already exist in `crates/db/migrations/001_init.sql`:
- `oauth_clients(client_id PK, name, redirect_uris JSON, created_at, last_used_at)`
- `oauth_authorization_codes(code PK, client_id FK, user_id, code_challenge, scope, redirect_uri, expires_at, used)`
- `oauth_refresh_tokens(token_hash PK, client_id FK, user_id, scope, expires_at, created_at)`

New `db::oauth` module wraps CRUD. No migration changes.

## Testing

### Unit tests
- `oauth/src/pkce.rs` — S256 verify, golden pairs from RFC 7636 Appendix B.
- `oauth/src/handlers.rs` (metadata) — stable JSON output with fake `PUBLIC_BASE_URL`.
- `mcp/src/resources.rs` — URI parser covers all 8 shapes + query parsing + invalid inputs.
- `domain::<module>::clamp_limit` — clamping edges.

### Integration tests
- `oauth/tests/flow.rs`:
  - DCR → authorize (with simulated session via `AuthSession::login` + `PendingOAuthRequest` seeded in session) → token → JWT decoded, claims asserted.
  - Refresh flow: token → refresh → new tokens; old refresh rejected; rotated refresh carries original `expires_at`.
  - Replay: reused auth code → `invalid_grant`.
  - PKCE mismatch → `invalid_grant`.
  - CSRF mismatch on `authorize POST` → 400.
  - `deny` decision → redirect with `error=access_denied`.
- `mcp/tests/tools.rs`: create in-memory pool via `db::test_helpers::test_pool`, inject `UserId`/`UserLocale` into a `RequestContext` fake. Exercise each of 11 tools against a seeded user. Assert domain-side state mutations.
- `mcp/tests/resources.rs`: projections returned; pagination across pages covers all rows with no duplicates/gaps; invalid URI → localized error; missing `UserId` → unauthorized.
- `mcp/tests/i18n.rs`: `list_tools` with `locale=pl` returns Polish descriptions; `locale=en` returns English; unknown key falls back to the raw key.
- `server/tests/mcp_e2e.rs`: full OAuth → JWT → real rmcp client over HTTP → `call_tool("create_item", ...)` end-to-end.

### Manual smoke (verification checklist)

- `curl POST /oauth/register` returns `client_id`.
- Browser: log in → open authorize URL → consent page renders (DaisyUI themed, both languages) → Approve → redirect to `redirect_uri?code=…`.
- `curl POST /oauth/token` with code + verifier returns JWT; decode shows `scope=mcp`, `exp = iat + 3600`.
- Claude Code (real): add the MCP server, confirm DCR + consent + tool listing (Polish in `pl` locale) + `create_item` works end-to-end.
- Revoke: `DELETE FROM oauth_refresh_tokens WHERE user_id = ?` → next `refresh_token` grant returns `invalid_grant`.

## Deferred to later PRs

- Tool/OAuth rate limiting (planned F4 security).
- "Connected Apps" revocation UI in Settings.
- Consent memory per client (currently every request).
- Refresh token reuse detection invalidating token family.
- Additional scopes beyond `mcp` (`read-only`, custom).
- Parameterised resource URIs (`?container={id}`) — we expect Claude Code to filter client-side for now.
- Removal of legacy `gateway/` and `crates/api/` once frontend-v2 is the only frontend.

## LOC budget

Roughly 1700 LOC total: ~1100 implementation + ~400 tests + ~200 types/boilerplate. Exceeds spec's F1 (~600) alone but fits F1 + F2 (~1150 combined) once consent UI and tests are included.

## Decisions recorded

| # | Decision | Chosen |
|---|---|---|
| 1 | PR size | Single, combined F1 + F2 |
| 2 | rmcp API | Current per context7 — `Extension(parts)` pattern confirmed |
| 3 | i18n scope | Tool descriptions + error messages + resource descriptions |
| 4 | DB tables | Already present in 001_init.sql — no migration |
| 5 | Consent page | Leptos SSR route in frontend-v2/ (`/oauth/consent`) |
| 6 | Consent persistence | None — prompt every time |
| 7 | PKCE strictness | S256 only; reject plain/missing |
| 8 | i18n scope | Descriptions, errors, resource descriptions — all localized |
| 9 | Token lifetimes | Access 1 h, refresh 30 days absolute + rotation on use |
| 10 | DCR rate limiting | Deferred to F4 |
| 11 | OAuth library | Custom — oxide-auth adapter cost > savings for our surface |
