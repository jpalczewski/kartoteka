# Rewrite Implementation Plan Structure

## Context

The 6 design specs (`docs/superpowers/specs/rewrite/00-05`) define a rewrite from Cloudflare Workers to a single Rust binary. This document defines **22 mini-plans** — each a vertical feature slice (~400-800 LOC), independently testable, one PR per plan.

## Crate Architecture (10 crates)

```
shared/    — types, FlexDate, constants (WASM + native, leaf)
i18n/      — FTL files (leaf)
db/        — pure queries, migration, pool, FlexDate Encode/Decode (depends: shared)
domain/    — rules + orchestration (depends: shared, db)
auth/      — AuthBackend, JWT, TOTP, sessions, personal tokens (depends: shared, domain)
mcp/       — rmcp tools + resources (depends: shared, domain, i18n)
oauth/     — oxide-auth provider, DCR (depends: shared, domain, auth)
jobs/      — apalis workers, maintenance, notifications (depends: shared, domain)
frontend/  — Leptos SSR (depends: shared, domain, i18n, auth[ssr])
server/    — thin glue: main.rs mounts everything (depends: all)
```

Compilation isolation: change in auth/ → only oauth/, frontend/, server/ recompile. mcp/, jobs/ untouched.

domain:: is the ONLY entry point for data access. db:: is internal — consumers never import it. domain:: does NOT re-export db:: types.

## Mini-Plan Structure

### Phase A: Foundation

**A1: Scaffold + Migration + Shared Types** (~600 LOC)
- Scaffold ALL 10 crate Cargo.tomls with empty stubs (shared, db, domain, auth, mcp, oauth, jobs, frontend, server, i18n)
- Root workspace Cargo.toml with `[[workspace.metadata.leptos]]`
- Consolidated SQLite migration (~20 tables + indexes + FTS5 + triggers)
- `db::create_pool` with WAL, pragmas, after_connect
- Shared types: FlexDate enum (without sqlx Encode/Decode — that lives in db/)
- db/: FlexDate sqlx Encode/Decode impl, sqlx row types, From conversions
- Test helpers: `test_pool()`, `create_test_user()`
- **Deliverable:** `cargo check --workspace` compiles, migration runs, pool connects
- **Creates:** all 10 crate scaffolds

### Phase B: Core CRUD (vertical slices — db + domain + REST per domain)

**B1: Containers + Home** (~500 LOC)
- `db::containers` — all queries (CRUD, children, progress, pin, move)
- `db::home` — composite home query with `tokio::join!`
- `domain::containers` — orchestration (hierarchy validation, move, pin)
- `domain::rules::containers` — validate_hierarchy, validate_move
- `domain::home` — pass-through
- REST: `/api/containers/*`, `/api/home` (in server/, no auth yet)
- Tests: db queries + domain rules + integration
- **Depends on:** A1
- **Deliverable:** curl containers CRUD works (no auth yet)

**B2: Lists** (~600 LOC)
- `db::lists` — CRUD, sublists, features, archive, reset, pin, move, get_create_item_context
- `domain::lists` — create with features (transaction), reset, toggle archive/pin
- `domain::rules::lists` — validate_list_type_features
- REST: `/api/lists/*`
- Tests: db + domain + feature slice validation
- **Depends on:** A1
- **Deliverable:** curl lists CRUD with features

**B3: Items** (~700-800 LOC, largest core module)
- `db::items` — CRUD, by-date, calendar, move
- `domain::items` — create (feature validation + position), update (dynamic single query + auto-complete), move, toggle_complete (blocker check placeholder — full enforcement in E2)
- `domain::rules::items` — validate_features, should_auto_complete
- REST: `/api/lists/:id/items/*`, `/api/items/by-date`, `/api/items/calendar`
- Includes `estimated_duration` field exposure in CRUD
- Tests: feature validation, auto-complete, by-date/calendar queries
- **Depends on:** A1, **B2** (needs list feature context for item creation validation)
- **Deliverable:** curl items CRUD with date queries

**B4: Tags** (~500 LOC)
- `db::tags` — CRUD, recursive CTE, merge, tag links, typed taxonomy (tag_type, metadata)
- `domain::tags` — merge (transaction), cycle detection, exclusive type enforcement (priority)
- `domain::rules::tags` — validate_merge, validate_parent, validate_exclusive_type, validate_location_hierarchy
- REST: `/api/tags/*`, `/api/tag-links/*`, assign/remove
- Tests: hierarchy, cycle detection, merge, exclusive priority
- **Depends on:** A1 (soft dependency on B2/B3 for tag link testing — test helpers create items/lists)
- **Deliverable:** curl tags CRUD with typed taxonomy

**B5: Settings + Preferences** (~200 LOC)
- `db::settings`, `db::preferences` (locale, timezone — both read from user_settings table)
- `domain::settings`, `domain::preferences`
- REST: `/api/settings/*`, `/api/preferences`
- **Depends on:** A1
- **Deliverable:** curl settings CRUD

### Phase C: Auth (vertical — auth/ crate + server wiring)

**C1: Basic Auth** (~600 LOC)
- **Creates `crates/auth/`** — AuthnBackend impl, AuthSession, session layer (tower-sessions + SqliteStore)
- `db::users`, `db::auth_methods`, `db::server_config`
- `domain::auth` — register (first user = admin, check registration enabled, argon2 in spawn_blocking)
- Endpoints in server/: POST /auth/register, /auth/login, /auth/logout
- Require middleware on API routes
- Admin middleware + `/api/server-config`
- Tests: register, login, wrong password 401, admin check
- **Depends on:** A1, B1-B5 (REST endpoints need auth protection)
- **Deliverable:** register + login + protected API works via curl

**C2: TOTP 2FA** (~300 LOC)
- `db::totp`
- auth/ crate: 2FA flow (pending_user_id in session, verify completes login)
- Endpoints: POST /auth/totp/setup, /auth/totp/verify, DELETE /auth/totp, POST /auth/2fa
- Tests: setup + verify flow, login with 2FA enabled
- **Depends on:** C1
- **Deliverable:** 2FA works end-to-end

**C3: Bearer JWT Tokens** (~400 LOC)
- `db::personal_tokens`
- auth/ crate: create_token (JWT with scope + jti), validate_jwt
- Bearer middleware (JWT verify → jti revocation check for long-lived → AuthContext)
- Unified middleware: bearer first, session fallback
- Endpoints: POST/GET/DELETE /auth/tokens
- Tests: create token, use bearer, revoke, scope enforcement
- **Depends on:** C1
- **Deliverable:** curl with Bearer token works on all API endpoints

### Phase D: Frontend SSR

**D1: Shell + Routing** (~400 LOC)
- cargo-leptos workspace metadata config
- `frontend/lib.rs` — shell(), hydrate(), App component
- Router with all routes (empty placeholder components)
- i18n SSR (leptos-fluent with cookie, Accept-Language)
- Tailwind 4 build setup (separate process)
- `server/main.rs` — leptos_routes_with_context, AppState with pool
- **frontend/ SSR depends on auth/ crate** for AuthSession extractor in server functions
- **Depends on:** C1-C3 (auth/ crate must exist)
- **Deliverable:** `cargo leptos watch` serves SSR shell with routing, i18n works

**D2: Home page** (~400 LOC)
- Server functions for home data (call domain::home)
- Home page component
- Refactor: split into `components/home/{pinned,recent,root}_section.rs`
- **Depends on:** D1
- **Deliverable:** Home page renders SSR with real data

**D3: Lists + Items** (~600 LOC)
- Server functions for lists, items, sublists, features
- List page migration (normal view + date view)
- Item detail page migration
- **Depends on:** D1
- **Deliverable:** List browsing + item detail works

**D4: Calendar + Today** (~400 LOC)
- Server functions for by-date, calendar
- Today page, calendar month/week views, calendar day page
- **Depends on:** D1
- **Deliverable:** Today + calendar views work

**D5: Tags + Containers + Secondary** (~500 LOC)
- Server functions for tags, containers, settings
- Tags page + tag detail, container page
- Settings page (language, timezone picker, MCP URL, admin section, token management)
- Login/signup pages (server functions instead of gloo-net)
- **Depends on:** D1
- **Deliverable:** All existing pages migrated to SSR

### Phase E: New Features (vertical — db + domain + frontend + MCP tool per feature)

**E1: Comments** (~400 LOC)
- `db::comments` — polymorphic CRUD
- `domain::comments` — create with author_type
- Frontend: `CommentList` + `AddComment` reusable components on item/list/container detail
- MCP tool prep: `add_comment` (wired in F1)
- FTS5 sync (comments_fts triggers already in migration)
- **Depends on:** D3 (item detail page), D5 (list/container pages)
- **Deliverable:** Comments on items/lists/containers work in UI

**E2: Relations** (~400 LOC)
- `db::relations` — CRUD, get_unresolved_blockers, bidirectional queries
- `domain::relations` — validate ownership
- `domain::items::toggle_complete` — blocker check integration (from B3 placeholder)
- `domain::rules::items::validate_can_complete`
- Frontend: `RelatedEntities` component on detail pages
- MCP tool prep: `add_relation`, `remove_relation` (wired in F1)
- **Depends on:** D3 (item detail page)
- **Deliverable:** Blocking relations prevent completion, visible on detail pages

**E3: Time Tracking** (~500 LOC)
- `db::time_entries` — CRUD, running timer, inbox, summary
- `domain::time_entries` — start (auto-stop previous), stop, log manual, assign, summary
- REST endpoints for time entries
- MCP tool prep: `start_timer`, `stop_timer`, `log_time` (wired in F1)
- Frontend: minimal placeholder (full timer UI as follow-up issue)
- **Depends on:** D3 (item detail for time display)
- **Deliverable:** Time logging works via API, inbox for unassigned entries

**E4: Templates + Search + All Items** (~500 LOC)
- `db::templates` — CRUD, template_items, template_tags
- `db::search` — FTS5 search across items + comments
- `domain::templates` — create_from_list, create_list_from_templates (merge)
- `domain::items::list_all_for_user` — new function for /all view
- REST endpoints for templates + search
- MCP tool prep: `save_as_template`, `create_list_from_template`, `search_items` (wired in F1)
- Frontend: `/all` page (all items view)
- **Depends on:** D3 (list/item pages)
- **Deliverable:** Templates, full-text search, all-items view

### Phase F: MCP + OAuth + Jobs + Deploy

**F1: MCP Tools + Resources** (~600 LOC)
- **Creates `crates/mcp/`** — KartotekaServer struct, tool_router
- 11 tools (create_item, update_item, search_items, add_comment, add_relation, remove_relation, start_timer, stop_timer, log_time, create_list_from_template, save_as_template)
- 8 resources (lists, list detail, list items, containers, container detail, tags, today, time summary)
- Resource templates for dynamic URIs
- Integration with domain:: via request Parts extensions (UserId)
- Mounting in server/ via nest_service (dev mode: direct Bearer, no OAuth)
- **Depends on:** E1-E4 (all domain functions used by tools exist)
- **Deliverable:** MCP tools work in dev mode with Bearer token

**F2: OAuth Provider** (~550 LOC)
- **Creates `crates/oauth/`** — oxide-auth core integration
- Axum handlers: /oauth/authorize, /oauth/token, /oauth/register (DCR)
- Well-known metadata endpoints
- oxide-auth storage backed by db:: (Registrar, Authorizer, Issuer)
- Consent page (Leptos SSR route in frontend/ — no circular dep, wired by server/)
- Bearer middleware integration with MCP endpoint
- Refresh token rotation
- **Depends on:** F1, C1-C3 (auth/ crate for JWT signing/validation)
- **Deliverable:** Full MCP OAuth flow works with Claude Code

**F3: Background Jobs** (~300 LOC)
- **Creates `crates/jobs/`** — apalis + apalis-sqlite + apalis-cron
- Job types: CleanupSessionsJob, CleanupOAuthCodesJob, MaintenanceJob (VACUUM/ANALYZE), OptimizeJob
- SendNotificationJob (Telegram channel placeholder)
- Worker registration in server/main.rs
- **Depends on:** A1 (can run parallel with F1/F2)
- **Deliverable:** Background jobs running, maintenance automated

**F4: Security + Deploy** (~400 LOC)
- Rate limiting (tower-governor on /auth/*, /oauth/*)
- CSRF on OAuth consent form POST
- CORS policy (CorsLayer config)
- systemd unit, Caddy config (reverse proxy, static cache headers)
- GitHub Actions CI/CD (build + deploy)
- Backup cron
- Environment config
- **Depends on:** F1-F3
- **Deliverable:** App running on VPS, security hardened, CI/CD working

## Dependency Graph

```
A1 (Foundation — scaffolds all 10 crates)
 ├── B1 (Containers+Home)
 ├── B2 (Lists)
 │    └── B3 (Items — needs B2 for feature context)
 ├── B4 (Tags)
 └── B5 (Settings)
      └── C1 (Basic Auth — creates auth/ crate)
           ├── C2 (TOTP 2FA)
           └── C3 (Bearer JWT)
                └── D1 (SSR Shell — frontend/ depends on auth/)
                     ├── D2 (Home)
                     ├── D3 (Lists+Items)
                     ├── D4 (Calendar+Today)
                     └── D5 (Tags+Containers+Secondary)
                          ├── E1 (Comments)
                          ├── E2 (Relations)
                          ├── E3 (Time Tracking)
                          └── E4 (Templates+Search+AllItems)
                               └── F1 (MCP Tools+Resources — creates mcp/)
                                    └── F2 (OAuth — creates oauth/)
A1 ──────────────────────────────── F3 (Jobs — creates jobs/, parallel with everything)
F1 + F2 + F3 ───────────────────── F4 (Security + Deploy)
```

Parallel opportunities:
- B1, B2, B4, B5 in parallel after A1 (B3 waits for B2)
- C2 and C3 in parallel after C1
- D2-D5 in parallel after D1
- E1-E4 in parallel after D5
- F3 in parallel with everything after A1

## Summary

| Phase | Plans | Total LOC | What it delivers |
|-------|-------|-----------|-----------------|
| A: Foundation | 1 | ~600 | Workspace, migration, types, pool |
| B: Core CRUD | 5 | ~2500 | Full API (no auth) |
| C: Auth | 3 | ~1300 | Login, 2FA, bearer tokens |
| D: Frontend | 5 | ~2300 | Full SSR frontend |
| E: Features | 4 | ~1800 | Comments, relations, time, templates, search |
| F: MCP+OAuth+Deploy | 4 | ~1850 | MCP, OAuth, jobs, production |
| **Total** | **22** | **~10350** | **Complete rewrite** |

## Deferred (not in rewrite, future issues)

- iCal feed (#31) — architecture supports it, endpoint not implemented
- SSE real-time updates (#30) — architecture supports it
- Timer UI (frontend) — E3 does backend only
- Template picker UI — E4 does backend only
- Recurring items (#34) — future migration + domain logic
- Reminder overrides (#32) — feature slice ready, not implemented

## Spec files to update (after plan approval)

All specs (00-05) need updating for 10-crate structure:
- Spec 00: crate list, MCP tools count (5→11)
- Spec 01: FlexDate Encode/Decode in db/ not shared/
- Spec 02: auth code in auth/ crate not server/
- Spec 03: frontend/ SSR depends on auth/
- Spec 04: OAuth in oauth/ crate not mcp/
- Spec 05: jobs in jobs/ crate not server/

## Verification

Each mini-plan has its own deliverable. End-to-end after all phases:

1. `cargo test --workspace` — all tests pass
2. `cargo clippy --workspace -- -D warnings` — clean
3. `cargo leptos build --release --precompress` — builds
4. Smoke test: register → login → 2FA → create list → add items → Bearer token → MCP tool call → OAuth flow
5. Deploy to VPS, verify Caddy + systemd + backup
