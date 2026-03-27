# Gateway Worker — Unified Auth + MCP Server

**Date:** 2026-03-27
**Status:** Approved
**Scope:** Replace Hanko auth with Better Auth, add MCP server, unify behind a single Gateway Worker

## Problem

Kartoteka has fragmented auth:
- Hanko Cloud with ugly, hard-to-style widgets in the frontend
- Manual JWT validation in the Rust API Worker
- No auth story for MCP or future CLI

Need a unified auth layer that supports web frontend (custom UI), MCP (OAuth 2.1), and future CLI (device flow).

## Decision

Single TypeScript Gateway Worker (Hono + Better Auth + MCP SDK) as the public entry point. Rust API Worker becomes internal, accessed via CF service binding.

## Architecture

```
CF Pages (Leptos frontend)
    │
    ▼
┌──────────────────────────────────────┐
│  Gateway Worker (TypeScript + Hono)  │
│                                      │
│  /auth/*    → Better Auth            │
│  /mcp/*     → MCP Server             │
│  /api/*     → proxy → API Worker     │
│                                      │
│  Bindings:                           │
│  - D1: auth DB (users, sessions)     │
│  - KV: OAuth tokens, MCP state       │
│  - Service: API_WORKER               │
└──────────────┬───────────────────────┘
               │ service binding
               ▼
┌──────────────────────────────────────┐
│  API Worker (Rust)                   │
│                                      │
│  /api/*  → business logic            │
│                                      │
│  Bindings:                           │
│  - D1: app DB (lists, items, etc.)   │
│  - Trusts Gateway (no own auth)      │
└──────────────────────────────────────┘
```

### Key decisions

- Gateway is the only public entry point for API calls — frontend never calls Rust Worker directly
- Gateway validates session, injects `X-User-Id` header, proxies to API Worker via service binding
- Rust API Worker drops its own auth middleware — trusts `X-User-Id` from Gateway
- Two separate D1 databases: auth DB (users, sessions, accounts) and app DB (lists, items)
- Dev mode: `DEV_AUTH_USER_ID` env var bypasses auth in Gateway, consistent with existing API Worker convention
- D1 holds Better Auth tables (users, sessions, accounts); KV holds OAuth 2.1 authorization codes and access tokens managed by the MCP OAuth provider

## Auth — Better Auth

### Why Better Auth

- Native Cloudflare D1 adapter (pass binding directly, no custom setup)
- Runs on CF Workers with Hono
- Plugin-based: enable auth methods as needed
- Built-in rate limiting (3 req/10s sign-in/sign-up)
- OAuth provider capability for MCP and CLI

### Auth methods (start)

- **Email + password** — baseline, always available
- **GitHub social login** — optional, one button

Future (no architecture changes needed): magic link, passkeys, 2FA — all Better Auth plugins.

### Auth flows by client

**Web frontend:**
1. Custom login/signup UI in Leptos (no external widgets)
2. Frontend POSTs credentials to Gateway `/auth/api/sign-in/email`
3. Better Auth returns session cookie
4. Subsequent requests carry cookie (`credentials: "include"` on all `gloo-net` requests) → Gateway validates → proxies to API with `X-User-Id`

**MCP (Claude Desktop/Code):**
1. Claude initiates OAuth 2.1 PKCE flow
2. Gateway shows consent screen (simple HTML: "Grant Kartoteka access to Claude")
3. User authenticates (email+password or GitHub) in consent screen
4. Gateway returns OAuth token → Claude uses it for subsequent MCP calls

**CLI (future):**
1. CLI opens browser → Gateway `/auth/device`
2. User authenticates in browser, approves
3. CLI receives token via polling/callback

**Dev mode:**
- Env var `DEV_AUTH_USER_ID` — Gateway passes through without auth, sets `X-User-Id` to hardcoded value

## MCP Server — Scope

Minimum viable, 5 tools:

| Tool | Description | API endpoint |
|------|-------------|-------------|
| `list_lists` | Show all user's lists | `GET /api/lists` |
| `get_list_items` | Show items in a list | `GET /api/lists/:list_id/items` |
| `create_list` | Create a new list | `POST /api/lists` |
| `add_item` | Add item to a list | `POST /api/lists/:list_id/items` |
| `toggle_item` | Mark item done/undone | `PUT /api/lists/:list_id/items/:id` (sends `{ completed: true/false }` to general update endpoint) |

Each tool is a thin wrapper: Zod input validation → service binding call to Rust API → format response for LLM.

Note: The Gateway proxy passes through ALL `/api/*` routes to the API Worker, not just those listed above. The MCP tools are a curated subset of the full API surface. Expansion (containers, tags, sublists) in future iterations without architecture changes.

## Hanko Migration

### What changes

- **Frontend:** Remove `hanko-init.js`, `hanko-init.js.template`, Hanko SDK. Replace with custom login/signup forms in Leptos calling Better Auth API.
- **API Worker:**
  - Remove `auth.rs` (Hanko JWT validation). Replace with simple `X-User-Id` header presence check — return 401 if absent.
  - Set `workers_dev = false` in `wrangler.toml` to prevent direct public access (only reachable via Gateway service binding).
  - `/api/health` remains unauthenticated (no `X-User-Id` required).
  - Remove CORS handling from `router.rs` — Gateway owns CORS now.
- **Config:** Drop `HANKO_API_URL` env var. Add `BETTER_AUTH_SECRET`, `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`.
- **User migration:** Not needed — no production users on Hanko.

## Code Structure

```
gateway/                        ← new TS project in repo root (replaces mcp/)
├── src/
│   ├── index.ts                ← Hono app, routing /auth/* /mcp/* /api/*
│   ├── auth.ts                 ← Better Auth config + plugins
│   ├── mcp/
│   │   ├── server.ts           ← McpServer setup + OAuthProvider
│   │   └── tools.ts            ← 5 tool definitions
│   ├── proxy.ts                ← service binding proxy to API Worker
│   └── middleware.ts           ← session → X-User-Id injection
├── package.json
├── tsconfig.json
├── wrangler.toml               ← D1 auth, KV, service binding config
└── drizzle/
    └── schema.ts               ← Better Auth tables (auto-generated)
```

## Error Handling & Security

- **Service binding** = internal communication, no public internet hop
- Gateway sets `X-User-Id` — API Worker trusts unconditionally
- API Worker rejects requests without `X-User-Id` (defense in depth)
- Better Auth built-in rate limiting for auth endpoints
- Cloudflare default rate limiting for MCP/API on Workers level
- Gateway owns CORS via Hono `cors()` middleware on `/api/*` and `/auth/*` routes — replaces CORS handling removed from Rust API Worker

## Environment Variables

### Gateway Worker
- `BETTER_AUTH_SECRET` — session encryption
- `BETTER_AUTH_URL` — public URL of the Gateway Worker (cookie domain, OAuth redirect URLs)
- `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET` — GitHub OAuth app
- `DEV_AUTH_USER_ID` — dev mode bypass (optional)

### API Worker
- `DEV_AUTH_USER_ID` — kept for direct dev access (optional)
- D1 binding to app database

## Out of Scope

- CLI implementation (future, uses device flow against Gateway)
- Additional auth methods beyond email+password and GitHub
- MCP tools beyond the initial 5
- Frontend redesign of login UI (separate task)
