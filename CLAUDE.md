# CLAUDE.md

## Projekt

Kartoteka — aplikacja todo/listy na Cloudflare Workers (Rust API + TypeScript Gateway) + Leptos CSR frontend + Better Auth.

## Architektura

- **Monorepo**: Cargo workspace (`crates/shared`, `crates/api`, `crates/frontend`) + `gateway/` (TypeScript)
- **API**: CF Worker z `worker` crate (0.7+), D1 database, `sqlx-d1` (0.3+)
- **Frontend**: Leptos 0.7 CSR, kompilowany do WASM przez Trunk, serwowany z CF Pages
- **Auth**: Better Auth — cookie-based sessions, email+password, GitHub OAuth optional
- **Gateway**: TypeScript Worker w `gateway/` — Hono + Better Auth + MCP server (5 tools), serwuje `/auth/*`, `/mcp/*`, proxy do API Worker via CF service binding

## Kluczowe konwencje

### Env vars (compile-time / .env)
- `GATEWAY_URL` — prod gateway URL (https://kartoteka-gateway.jpalczewski.workers.dev)
- `GATEWAY_DEV_URL` — dev gateway URL (https://kartoteka-gateway-dev.jpalczewski.workers.dev)
- `CLOUDFLARE_ACCOUNT_ID` — CF account ID
- Gateway sekrety (wrangler secrets, nie w .env): `BETTER_AUTH_SECRET`, `BETTER_AUTH_URL`, `MIGRATE_SECRET`
- Zarządzane przez `.env` + `set dotenv-load` w justfile

### Better Auth / Cookie auth
- Frontend wysyła żądania z `credentials: "include"` — sesja via cookie (HttpOnly, set przez Gateway)
- `API_BASE_URL` — opcjonalny compile-time env var, domyślnie `/api` (wystarczy do dev + testów)
- Lokalnie: default `/api` — Trunk proxy → Gateway (8788) → API Worker (8787)
- Prod/dev deploy: `API_BASE_URL="${GATEWAY_URL}/api"` — frontend → Gateway → API Worker via service binding
- Gateway Worker waliduje sesję i dodaje `X-User-Id` header do żądań do API Worker
- `DEV_AUTH_USER_ID` env var (w `[env.local]` wrangler.toml) — bypass auth w dev lokalnym
- `/auth/api/get-session` zwraca fake sesję gdy `DEV_AUTH_USER_ID` jest ustawiony

### D1 / SQLite
- D1 zwraca boolean jako float (`0.0`/`1.0`) — `Item.completed` ma custom deserializer `bool_from_number` w `shared/src/lib.rs`
- IDs jako UUID v4 (TEXT), timestampy jako TEXT (`datetime('now')`)
- Migracje w `crates/api/migrations/`, zarządzane przez `wrangler d1 migrations`

### Frontend
- `LocalResource` zamiast `Resource` — futures z `gloo-net` nie są `Send` (WASM)
- `LocalResource` wrappuje w `SendWrapper` — dereferencja przez `&*result` w match
- Optymistyczne update'y w list page (toggle/delete aktualizują lokalny `RwSignal`, API call w tle)
- `gloo-net` 0.6: `Request::get()` zwraca `RequestBuilder`, `.body()` zwraca `Result<Request>`

### Workers API
- `worker` crate 0.7+ wymagany przez `worker-build`
- `sqlx-d1` musi być w tej samej major wersji `worker` co API crate (inaczej dwa `worker` w drzewie → build error)
- D1 bind: `ctx.env.d1("DB")?`, parametry jako `JsValue` (nie custom `D1Type`)
- `Headers::new()` — nie wymaga `mut`
- `Response::empty()?.with_status(204)` — zwraca `Response`, nie `Result`, trzeba `Ok()`

### Deploy
- CF Pages wymaga `--branch=main` (production branch)
- `CLOUDFLARE_ACCOUNT_ID` wymagany (dwa konta w systemie)
- Account ID w `wrangler.toml` i jako env var w justfile

## Komendy

```bash
just dev          # API + frontend lokalnie
just dev-gateway  # Gateway lokalnie
just check        # Kompilacja workspace
just deploy       # Deploy wszystkiego
just lint         # Clippy + fmt check
```

## Dokumentacja i aktualne wersje bibliotek

Projekt używa szybko ewoluujących bibliotek (Leptos 0.7, gloo-net 0.6, worker 0.7+,
sqlx-d1 0.3+, DaisyUI 5). Przed pisaniem kodu sprawdzaj aktualne API przez context7 MCP:

- `mcp__context7__resolve-library-id` — znajdź ID biblioteki (np. "leptos", "gloo-net")
- `mcp__context7__query-docs` — pobierz aktualną dokumentację

Używaj tego proaktywnie, nie czekaj na błędy kompilacji.

## CI/CD

- PR workflow (`.github/workflows/ci.yml`): fmt → check → clippy → deny → machete → test
- Security audit (`.github/workflows/security-audit.yml`): weekly cron + on Cargo.lock changes
- Workspace lints w `Cargo.toml` — clippy correctness=deny, suspicious/complexity/perf/style=warn
- `deny.toml` — licencje, advisories, supply chain security
- Lokalne sprawdzenie: `just ci` (uruchamia wszystko naraz)

## Pliki do NIE commitowania

- `.env` — sekrety i konfiguracja
- `crates/frontend/dist/` — build output
- `.wrangler/`, `build/` — wrangler cache
