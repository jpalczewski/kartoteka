# CLAUDE.md

## Projekt

Kartoteka — aplikacja todo/listy na Cloudflare Workers (Rust API + TypeScript Gateway) + Leptos CSR frontend + Better Auth.

## Architektura

- **Monorepo**: Cargo workspace (`crates/shared`, `crates/api`, `crates/frontend`) + `gateway/` (TypeScript)
- **API**: CF Worker z `worker` crate (0.7+), D1 database, `sqlx-d1` (0.3+)
- **Frontend**: Leptos 0.8 CSR, kompilowany do WASM przez Trunk, serwowany z CF Pages
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
- D1 zwraca boolean jako float (`0.0`/`1.0`) — `Item.completed` ma custom deserializer `bool_from_number` w `shared/src/deserializers.rs`
- IDs jako UUID v4 (TEXT), timestampy jako TEXT (`datetime('now')`)
- Migracje w `crates/api/migrations/`, zarządzane przez `wrangler d1 migrations`

### Frontend
- `LocalResource` zamiast `Resource` — futures z `gloo-net` nie są `Send` (WASM)
- Leptos 0.8: `LocalResource::get()` zwraca `Option<T>` bezpośrednio — pattern: `if let Some(Ok(data)) = resource.get()`
- Brak `SendWrapper` — usunięty w Leptos 0.8, `LocalResource` nie wymaga wrappowania
- HTTP: `HttpClient` trait w `crates/frontend/src/api/client.rs` — `GlooClient` (WASM) podawany przez `provide_context(GlooClient)` w `app.rs`, pobierany przez `use_context::<GlooClient>()`
- Optymistyczne update'y ze snapshot+rollback: `let previous = signal.get_untracked()` → optymistyczna zmiana → `signal.set(previous)` przy błędzie
- State transforms w `crates/frontend/src/state/transforms.rs` — `with_item_toggled`, `without_item` (pure functions, testowalne natywnie)

### Shared crate (`crates/shared/`)
- Struktura modułów: `models/` (Container, List, Item, Tag, Settings), `dto/` (requests.rs, responses.rs), `deserializers.rs` (pub(crate)), `constants.rs`, `date_utils.rs`
- `lib.rs` reeksportuje wszystko flat (`pub use models::*; pub use dto::*; pub use date_utils::*`) — importy w `crates/api` działają bez zmian
- `date_utils.rs` — 14 pub funkcji oparte na `chrono` (parse_date, add_days, week_range, month_grid_range, is_overdue, sort_by_deadline, itd.)
- `chrono = { version = "0.4.44", default-features = false, features = ["alloc"] }` — WASM-safe, bez std::time clock
- `HomeData`, `ContainerChildrenResponse`, `ErrorResponse` itp. w `dto/responses.rs`

### Workers API
- `worker` crate 0.7+ wymagany przez `worker-build`
- `sqlx-d1` musi być w tej samej major wersji `worker` co API crate (inaczej dwa `worker` w drzewie → build error)
- D1 bind: `ctx.env.d1("DB")?`, parametry jako `JsValue` (nie custom `D1Type`)
- `Headers::new()` — nie wymaga `mut`
- `Response::empty()?.with_status(204)` — zwraca `Response`, nie `Result`, trzeba `Ok()`

### Deploy
- CF Pages wymaga `--branch=main` (production branch)
- `CLOUDFLARE_ACCOUNT_ID` env var wymagany (wrangler bierze z env, nie z wrangler.toml)

### API helpers (`crates/api/src/helpers.rs`)
- `check_ownership(d1, table, id, user_id)` — weryfikacja własności zasobu
- `check_item_ownership(d1, item_id, user_id)` — weryfikacja przez JOIN z lists
- `toggle_bool_field(d1, table, column, id, user_id)` — toggle boolean (D1 0/1)
- `next_position(d1, table, filter, params)` — MAX(position) + 1
- `opt_str_to_js(opt)` — Option<String> → JsValue (Some → string, None → NULL)
- `require_param(ctx, name)` — wyciągnij param z RouteContext lub Error
- `get_list_features(d1, list_id)` — lista feature names dla listy

### Tracing / Logging

- Crate `kartoteka-logging` — inicjalizacja przez `kartoteka_logging::init_cf()` w `#[event(start)]`
- Handlery instrumentowane: `#[instrument(skip_all, fields(action = "create_list", list_id = tracing::field::Empty))]`
- Dynamiczne pola: `Span::current().record("list_id", tracing::field::display(&id))` — **bez `&` przed `tracing::field::display`** (clippy `needless_borrows_for_generic_args` blokuje CI)
- Gateway: `log()` z `gateway/src/logger.ts` — ten sam schemat JSON, korelacja przez `X-Request-Id`

## Komendy

```bash
just dev          # API + Gateway + frontend + tunnel lokalnie
just dev-api      # Tylko API worker
just dev-gateway  # Tylko Gateway worker
just dev-frontend # Tylko frontend (Trunk)
just check        # Kompilacja workspace
just build        # Build all (API + frontend + gateway)
just test         # cargo test --workspace
just test-e2e     # Playwright e2e (wymaga just dev)
just lint         # Clippy + fmt check
just fmt          # cargo fmt
just ci           # fmt + lint + audit + machete + test
just deploy       # Deploy prod (migrate + API + gateway + frontend)
just deploy-dev   # Deploy dev environment
```

## Dokumentacja i aktualne wersje bibliotek

Projekt używa szybko ewoluujących bibliotek (Leptos 0.8, gloo-net 0.7, worker 0.7+,
sqlx-d1 0.3+, DaisyUI 5). Przed pisaniem kodu sprawdzaj aktualne API przez context7 MCP:

- `mcp__context7__resolve-library-id` — znajdź ID biblioteki (np. "leptos", "gloo-net")
- `mcp__context7__query-docs` — pobierz aktualną dokumentację

Używaj tego proaktywnie, nie czekaj na błędy kompilacji.

## Testy

- **Unit testy** (`crates/shared/src/tests/`, `crates/shared/src/date_utils.rs`): deserializery D1, typy, serde, date_utils — `cargo test -p kartoteka-shared`
- **Frontend testy** (`crates/frontend/src/`): API helpers (MockClient + tokio::test), state transforms — `cargo test -p kartoteka-frontend --lib`; `[[bin]] test = false` w Cargo.toml (zapobiega kompilacji WASM binary na native target)
- **i18n testy** (`crates/i18n/tests/`): kompletność tłumaczeń PL/EN, parsowanie FTL, pokrycie MCP
- **E2E** (`tests/e2e/`): Playwright, auth flow — `just test-e2e` (wymaga `just dev`)
- **Brak testów**: `crates/api` (wymaga D1/Worker runtime)
- CI: `cargo test --workspace` (shared + frontend + i18n)

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
