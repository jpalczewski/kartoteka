# OAuth 2.1 Provider — Design Spec

Parent: `00-overview.md`
Crate: `crates/oauth/` (depends: shared, domain, auth, oxide-auth, jsonwebtoken)

## Architecture

oxide-auth core for authorization code grant + PKCE S256 verification + token signing. Custom Axum handlers for MCP-specific endpoints (DCR, well-known metadata). Bearer middleware validates JWT tokens and injects UserId.

```
Claude Code → POST /mcp → 401
  → GET /.well-known/oauth-authorization-server → metadata
  → POST /oauth/register → client_id (DCR)
  → GET /oauth/authorize → consent page (Leptos SSR route in frontend/)
  → POST /oauth/authorize → auth code → redirect
  → POST /oauth/token → verify PKCE → JWT (access + refresh)
  → POST /mcp (Bearer) → tools work
```

## Why oxide-auth core (not oxide-auth-axum)

oxide-auth-axum is opinionated and doesn't cover DCR or MCP-specific metadata. We use oxide-auth core (`Registrar`, `Authorizer`, `Issuer` traits) and write thin Axum handlers.

## Endpoints

| Endpoint | Method | Auth | Purpose |
|----------|--------|------|---------|
| `/.well-known/oauth-authorization-server` | GET | none | RFC 8414 metadata |
| `/.well-known/oauth-protected-resource` | GET | none | RFC 9728 resource metadata |
| `/oauth/register` | POST | none | Dynamic Client Registration (RFC 7591) |
| `/oauth/authorize` | GET | session | Consent page (Leptos SSR route) |
| `/oauth/authorize` | POST | session | Approve → auth code → redirect |
| `/oauth/token` | POST | none | Exchange code + PKCE verifier → JWT |

## Tokens (unified JWT system)

Same JWT format as personal tokens (see `03-auth.md`):
- **MCP access token:** JWT with `scope: "mcp"`, short-lived (1h). No revocation check.
- **Refresh token:** Opaque string in `oauth_refresh_tokens` table. Rotation on each use.
- **Authorization code:** Short-lived (5 min), in db with PKCE code_challenge, consumed on exchange.

Same HMAC-SHA256 signing secret. Same bearer middleware validates MCP and personal JWTs — differentiated by `scope` claim.

## Bearer middleware

```rust
// In oauth/ crate, applied to /mcp routes
async fn bearer_auth_middleware(
    State(state): State<OAuthState>,
    req: Request, next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer(&req)?;
    let claims = jsonwebtoken::decode::<Claims>(token, &state.key, &validation)?;
    // Short-lived: skip revocation. Long-lived: check jti.
    req.extensions_mut().insert(UserId(claims.sub));
    Ok(next.run(req).await)
}
```

## Consent page

Leptos SSR route in `frontend/` at `GET /oauth/authorize`. No circular dep — oauth/ redirects via URL, frontend/ renders the page, form POSTs to oauth/ endpoint. `server/main.rs` wires both.

Shows: client name, scopes, Approve/Deny. Session check: no login → redirect `/auth/login` with `return_to`.

## DCR

~60 LOC. Stores in `oauth_clients` table. Claude Code auto-registers.

## Crate structure

```
crates/oauth/src/
  lib.rs          — OAuthState, routes()
  handlers.rs     — authorize, token, register, metadata endpoints
  bearer.rs       — bearer_auth_middleware
  storage.rs      — oxide-auth Registrar/Authorizer/Issuer backed by domain::
  types.rs        — request/response types
```

## Testing

- OAuth flow: integration test — register client → authorize → exchange code → verify JWT → bearer call
- Bearer middleware: valid → UserId, invalid → 401
- PKCE: S256 challenge/verifier pair through oxide-auth
