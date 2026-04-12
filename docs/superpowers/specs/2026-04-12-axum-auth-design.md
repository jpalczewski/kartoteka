# Plan 2: Axum Skeleton + Auth — Design Spec

Parent: `docs/superpowers/specs/2026-04-12-cloudflare-exit-v2-design.md`
Depends on: Plan 1 (crates/db)

## Goal

Standalone Axum binary with auth system and REST API placeholders. No frontend — JSON-only endpoints, testable via curl. Deliverable: `crates/server` that can run on VPS.

## Auth Architecture

### Authentication methods (present)

- **Email + password** — argon2 hash, stored in `auth_methods(provider='password')`
- **TOTP 2FA** — optional per user, totp-rs, stored in `totp_secrets`

### Authentication methods (future, not implemented now but architecture must not block)

- **Social OAuth** (GitHub, Google) — `auth_methods(provider='github', provider_id=github_uid)`. `Credentials` enum with `Password` and `OAuth` variants.
- **Passkeys/WebAuthn** — `auth_methods(provider='webauthn', credential=JSON)`. Separate challenge-response flow, then `auth_session.login()`.
- **Personal access tokens / bearer tokens** — for curl, automations, simple integrations. Token table (`personal_tokens(id, user_id, token_hash, name, last_used_at, expires_at)`). Auth middleware checks `Authorization: Bearer <token>` header before falling through to session check.

### Session management

- `axum-login` 0.18 — `AuthnBackend` trait, `AuthSession` extractor, `Require` middleware
- `tower-sessions` 0.15 + `tower-sessions-sqlx-store` (SQLite)
- Cookie: HttpOnly, Secure, SameSite=Lax, 7 day expiry on inactivity
- Expired session cleanup: `continuously_delete_expired()` background task

### 2FA flow

Password verify does NOT call `auth_session.login()`. Instead:

1. `POST /auth/login` — verify argon2 → check if TOTP enabled
2. If TOTP enabled: store `pending_user_id` in raw session → return `{"status": "2fa_required"}`
3. `POST /auth/2fa` — verify TOTP code → `auth_session.login(&user)` → full session
4. If TOTP not enabled: `auth_session.login(&user)` immediately

This way `Require` middleware works unchanged — user is only "logged in" after full auth chain.

### Post-login redirect

`return_to` stored in session (not URL query param). Set before redirect to login, consumed after successful login. No open redirect risk, survives page refresh.

### Registration

- Gated by `server_config.registration_enabled` (dynamic, no restart)
- First registered user gets `role = 'admin'`
- `POST /auth/register` checks config, hashes password, creates user + auth_method

### Admin middleware

Separate `admin_required` middleware layer (not inline check). Checks `user.role == "admin"`. Applied to server config routes.

### Future: Bearer token auth

Auth middleware resolution order:
1. `Authorization: Bearer <token>` header → look up `personal_tokens`, verify hash, get user_id
2. Session cookie → axum-login `AuthSession`
3. Neither → 401

This is NOT implemented in Plan 2 but the middleware architecture (Axum layers) allows inserting a bearer token check layer later without changing handlers.

## Crate structure

```
crates/server/
  src/
    main.rs              — bootstrap: pool, sessions, auth layer, router
    lib.rs               — re-exports for integration tests
    error.rs             — AppError → JSON responses
    auth/
      mod.rs
      backend.rs         — AuthnBackend impl (argon2 verify, user lookup)
      handlers.rs        — /auth/register, /auth/login, /auth/logout, /auth/2fa
      totp.rs            — /auth/totp/setup, /auth/totp/verify, DELETE /auth/totp
      middleware.rs       — admin_required
    api/
      mod.rs             — router composition, Require middleware
      containers.rs      — thin wrappers on db::containers
      lists.rs           — thin wrappers on db::lists
      items.rs           — thin wrappers on db::items
      tags.rs            — thin wrappers on db::tags
      settings.rs        — thin wrappers on db::settings
      preferences.rs     — thin wrappers on db::preferences
      home.rs            — thin wrapper on db::home
      server_config.rs   — admin-only GET/PUT
```

## New db modules (added to crates/db)

- `db::users` — User struct (with role), create, find_by_email, find_by_id, count
- `db::auth_methods` — create, find_by_user_and_provider
- `db::totp` — upsert, find, mark_verified, delete
- `db::server_config` — get, set, is_registration_enabled

## REST API endpoints

All current endpoints preserved 1:1. Same paths, same request/response shapes. Auth via `AuthSession` extractor. Each handler is 3-5 lines: extract user → call `db::` → return JSON.

Additional endpoints:
- `POST /auth/register` — `{email, password, name?}`
- `POST /auth/login` — `{email, password}` → `{status, user?, return_to?}` or `{status: "2fa_required"}`
- `POST /auth/2fa` — `{code}` → `{status, user, return_to?}`
- `POST /auth/logout`
- `POST /auth/totp/setup` (authenticated) → `{secret, otpauth_url}`
- `POST /auth/totp/verify` (authenticated) — `{code}` → marks verified
- `DELETE /auth/totp` (authenticated) — disables 2FA
- `GET /api/server-config` (admin) → `{registration_enabled: bool, ...}`
- `PUT /api/server-config/{key}` (admin) — `{value}`
- `GET /api/health` (no auth)

## Tech stack

| Crate | Version | Purpose |
|-------|---------|---------|
| axum | 0.8 | HTTP framework |
| axum-login | 0.18 | AuthSession, Require, AuthnBackend |
| tower-sessions | 0.15 | Session management |
| tower-sessions-sqlx-store | 0.15 | SQLite session storage |
| argon2 | 0.6 | Password hashing |
| totp-rs | 5.7 | TOTP 2FA |
| tower-http | 0.6 | TraceLayer, CorsLayer |
| tracing + tracing-subscriber | 0.1/0.3 | Structured logging |
| time | 0.3 | Session expiry durations |

## Testing strategy

- Integration tests via `axum::body::Body` + tower `ServiceExt::oneshot()`
- In-memory SQLite for test isolation
- Test cases: register (first=admin, second=user), login, wrong password → 401, 2FA flow, admin middleware, registration disabled → 403

## What this plan does NOT include

- HTML pages (Plan 3 — Leptos SSR)
- MCP server (Plan 4)
- OAuth provider for MCP (Plan 4)
- Deploy (Plan 5)
- Social auth / passkeys / bearer tokens (future plans)
