# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Projekt

Kartoteka — aplikacja todo/listy. Leptos 0.8 SSR (Axum) + SQLite + `axum-login`/`tower-sessions`. MCP server via `rmcp`.

## Architektura

- **Monorepo**: Cargo workspace — `api`, `auth`, `db`, `domain`, `frontend`, `frontend-v2`, `i18n`, `jobs`, `logging`, `mcp`, `oauth`, `server`, `shared`
- **Server**: Axum (`crates/server`) — SSR, `/api/*`, auth middleware
- **Frontend**: Leptos 0.8 SSR w `crates/frontend-v2` (główny). `crates/frontend` trzyma Tailwind input + `node_modules`
- **MCP**: `crates/mcp` używa `rmcp` (Rust MCP SDK), wystawiany przez `crates/server`
- **Gateway** (`gateway/`): osobny worker Cloudflare — edge proxy dla MCP, deploy via wrangler. Migracje schema przez endpoint `/migrate`
- **Auth**: `crates/auth` — `axum-login` + `tower-sessions` (SQLite store), hasła `argon2`, 2FA `totp-rs`. OAuth providers w `crates/oauth`. Sesja via HttpOnly cookie
- **DB**: SQLite przez `sqlx`, UUID v4 (TEXT) IDs, timestampy `datetime('now')`. Migracje w `crates/db/migrations/` wykonywane przy starcie serwera

## Kluczowe konwencje

### Env vars
- `OAUTH_SIGNING_SECRET` — wymagany do dev (min 32 znaków), zarządzane przez `.env` + `set dotenv-load` w justfile

### Frontend (Leptos 0.8 SSR)
- Używaj `Resource::new`, nie `LocalResource` — SSR futures muszą być `Send`
- `Resource::get()` zwraca `Option<T>` — pattern: `if let Some(Ok(data)) = resource.get()`
- Server functions przez `#[server]` makro w `crates/frontend-v2/src/server_fns/`
- `use_context` tylko w ciele komponentu — nie w closures
- Non-Copy typy w `Fn` closure → `StoredValue::new()` lub `.clone()` przed wejściem

### Shared crate (`crates/shared/`)
- `models/`, `dto/` (requests/responses), `deserializers.rs` (pub(crate)), `constants.rs`, `date_utils.rs`
- `lib.rs` reeksportuje flat (`pub use models::*; pub use dto::*; pub use date_utils::*`)
- `chrono = { version = "0.4.44", default-features = false, features = ["alloc"] }` — WASM-safe
- Daty: zobacz `date_utils.rs` (parse_date, week_range, month_grid_range, is_overdue, sort_by_deadline, …)

### Tracing / Logging
Każdy handler w `crates/server/src/` **musi** mieć `#[instrument(fields(action = "verb_noun", <entity_id> = %id))]`. Bez `&` przed `tracing::field::display` (clippy `needless_borrows_for_generic_args` blokuje CI).

## Komendy

```bash
just setup        # cargo-leptos + wasm target + npm install (crates/frontend)
just dev          # dev-leptos + dev-tailwind w parallel (trap/wait)
just check        # cargo check --workspace
just test         # cargo test --workspace
just test-e2e     # Playwright (wymaga running just dev)
just lint         # Clippy + fmt check
just fmt          # cargo fmt
just ci           # fmt + lint + audit + machete + test
just deploy       # gateway (Cloudflare) + auth migrations
just deploy-preview  # build AMD64 obraz lokalnie + deploy preview
```

### Build frontend+server (Leptos SSR)
- **ZAWSZE** `cargo leptos build` — buduje WASM (hydrate) i serwer w spójnych feature'ach
- **NIGDY** `cargo build -p kartoteka-server` — default features rozjeżdżają się z WASM, hydration pęka
- Produkcja: `cargo leptos build --release`

## Pre-commit (lefthook)

`lefthook.yml` uruchamia `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` w parallel przed każdym commitem.

## Dokumentacja bibliotek (context7)

Projekt używa szybko ewoluujących bibliotek (Leptos 0.8, DaisyUI 5, rmcp). Przed pisaniem kodu sprawdzaj aktualne API przez context7 MCP (`mcp__context7__resolve-library-id` + `mcp__context7__query-docs`). Proaktywnie, nie czekaj na błędy.

## Testy

- Unit (`crates/shared/`): `cargo test -p kartoteka-shared`
- i18n (`crates/i18n/tests/`): kompletność PL/EN, parsowanie FTL, pokrycie MCP
- E2E (`tests/e2e/`): Playwright, 9+ spec files (auth, items, lists, tags, settings, sublists) — `just test-e2e`

## Deployment

- **Główna aplikacja**: Docker (`Dockerfile`: node css-builder → rust nightly builder → debian-slim runtime). Deploy na **Coolify** przez `docker-build-deploy` workflow (push na main)
- **Gateway** (MCP edge): Cloudflare Workers via `wrangler deploy` (z `gateway/`)
- Preview: `just deploy-preview` (AMD64 build via Colima)

## CI/CD

- `.github/workflows/`: `ci.yml` (fmt→check→clippy→deny→machete→test), `docker-build-deploy.yml`, `docker-preview.yml`, `security-audit.yml` (weekly), `codeql.yml`, `release-please.yml`, `claude-code-review.yml`, `deps-outdated.yml`
- Workspace lints w `Cargo.toml` — clippy correctness=deny, suspicious/complexity/perf/style=warn
- `deny.toml` — licencje, advisories, supply chain
- Lokalne sprawdzenie: `just ci`

## Pliki do NIE commitowania

- `.env` — sekrety
- `target/`, `build/`, `.wrangler/` — build/cache
