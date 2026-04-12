# Kartoteka Rewrite: Overview

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
  /oauth/*    → OAuth 2.1 provider (oxide-auth core)
  /.well-known/* → OAuth metadata
```

## Crate Architecture (10 crates)

```
crates/
  shared/    — types, FlexDate, constants (WASM + native, leaf)
               deps: serde, chrono, schemars
  i18n/      — FTL files, leptos-fluent (leaf)
  db/        — pure queries, migration, pool, FlexDate Encode/Decode (internal to domain)
               deps: shared, sqlx[sqlite,chrono]
  domain/    — rules + orchestration — ONLY entry point for data access
               deps: shared, db, chrono-tz, tokio
  auth/      — AuthBackend, JWT, TOTP, sessions, personal tokens
               deps: shared, domain, axum-login, tower-sessions, argon2, totp-rs, jsonwebtoken
  mcp/       — rmcp tools (11) + resources (8)
               deps: shared, domain, i18n, rmcp, schemars
  oauth/     — oxide-auth provider, DCR, well-known metadata
               deps: shared, domain, auth, oxide-auth, jsonwebtoken
  jobs/      — apalis workers, maintenance, notifications
               deps: shared, domain, apalis, apalis-sqlite, apalis-cron, reqwest
  frontend/  — Leptos 0.8 SSR (cdylib + rlib)
               deps: shared, i18n, leptos, leptos_router, leptos_meta, leptos-fluent
               deps(ssr): domain, auth, axum, leptos_axum, sqlx
  server/    — thin Axum glue: main.rs mounts everything
               deps: all crates
```

Compilation isolation: change in auth/ → only oauth/, frontend/, server/ recompile. mcp/, jobs/ untouched.

**Key rule:** domain:: is the ONLY entry point for data access. db:: is internal — consumers never import it.

### What disappears

- `crates/api/` (2164 LOC) — worker crate, replaced by server functions + REST placeholders
- `gateway/` (797 LOC TypeScript) — auth, MCP, proxy — all in Rust now

## Tech Stack

| Component | Crate | Purpose |
|-----------|-------|---------|
| HTTP framework | axum 0.8 | Routing, extractors, middleware |
| Frontend | Leptos 0.8 SSR | Components, server functions, hydration |
| Build tool | cargo-leptos | Dual-target build (server + WASM) |
| Database | sqlx + SQLite | Async queries, WAL, FTS5 |
| Auth sessions | axum-login 0.18 + tower-sessions 0.15 | Session management, auth middleware |
| Password hashing | argon2 | Secure password storage |
| 2FA | totp-rs 5.7 | TOTP generation + verification |
| JWT tokens | jsonwebtoken 10 | Unified bearer tokens (personal, MCP, calendar) |
| MCP server | rmcp | StreamableHTTP + tool/resource macros |
| OAuth server | oxide-auth 0.6 (core) | Auth code + PKCE + token signing |
| i18n | leptos-fluent 0.2 | Fluent translations, SSR support |
| Logging | tracing + tracing-subscriber | Structured logging (pretty dev, JSON prod) |
| Background jobs | apalis + apalis-sqlite + apalis-cron | Persistent job queue, cron, retry |
| HTTP client | reqwest | External API calls (Telegram, web push) |
| Compression | tower-http | gzip/brotli on responses |
| TLS | Caddy | Auto Let's Encrypt |

## What's NOT Changing

- UI/UX: same pages, same components, same DaisyUI neon-night theme
- i18n: same FTL files, same PL/EN support
- Data model: same core tables, same relations

## Deferred (not in rewrite)

- iCal feed (#31) — architecture supports it, endpoint not implemented
- SSE real-time updates (#30) — architecture supports it
- Recurring items (#34) — future migration + domain logic
- Reminder overrides (#32) — feature slice ready
- GitHub OAuth / Passkeys / WebAuthn — auth_methods table ready
- Timer UI (frontend) — backend only in rewrite
- Template picker UI — backend only in rewrite

## Spec Files

| # | File | Covers |
|---|------|--------|
| 00 | `00-overview.md` | This file — architecture, tech stack |
| 01 | `01-schema.md` | Full SQL migration |
| 02 | `02-db-domain.md` | DB + Domain layer |
| 03 | `03-auth.md` | Auth crate |
| 04 | `04-frontend.md` | Leptos SSR |
| 05 | `05-mcp.md` | MCP tools + resources |
| 06 | `06-oauth.md` | OAuth provider |
| 07 | `07-jobs-deploy.md` | Background jobs, deploy, security |
| 08 | `08-plan-structure.md` | 22 mini-plans |
