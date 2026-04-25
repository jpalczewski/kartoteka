# CLAUDE.md

## Projekt

Kartoteka — aplikacja todo/listy. Leptos 0.8 SSR (Axum) + SQLite + Better Auth.

## Architektura

- **Monorepo**: Cargo workspace — `crates/shared`, `crates/db`, `crates/domain`, `crates/auth`, `crates/mcp`, `crates/oauth`, `crates/jobs`, `crates/i18n`, `crates/frontend-v2`, `crates/server`
- **Server**: Axum (`crates/server`) — obsługuje SSR, `/api/*`, auth middleware
- **Frontend**: Leptos 0.8 SSR w `crates/frontend-v2`, kompilowany przez `cargo leptos`
- **Auth**: Better Auth — cookie-based sessions, email+password, GitHub OAuth optional
- **DB**: SQLite przez `sqlx`, migracje w `crates/db/migrations/`

## Kluczowe konwencje

### Env vars (compile-time / .env)
- `OAUTH_SIGNING_SECRET` — wymagany do dev (min 32 znaków)
- Zarządzane przez `.env` + `set dotenv-load` w justfile

### Auth / Cookie auth
- Sesja via HttpOnly cookie
- Server middleware (`crates/server/src/`) waliduje sesję i udostępnia `UserId` extractor

### SQLite / sqlx
- IDs jako UUID v4 (TEXT), timestampy jako TEXT (`datetime('now')`)
- Migracje w `crates/db/migrations/`, wykonywane przy starcie serwera
- Boolean jako `bool` (sqlx obsługuje natywnie dla SQLite)

### Frontend (Leptos 0.8 SSR)
- Używaj `Resource::new`, nie `LocalResource` — SSR futures muszą być `Send`
- `Resource::get()` zwraca `Option<T>` — pattern: `if let Some(Ok(data)) = resource.get()`
- Server functions przez `#[server]` makro w `crates/frontend-v2/src/server_fns/`
- `use_context` tylko w ciele komponentu — nie w closures
- Non-Copy typy w `Fn` closure → `StoredValue::new()` lub `.clone()` przed wejściem

### Shared crate (`crates/shared/`)
- Struktura modułów: `models/` (Container, List, Item, Tag, Settings), `dto/` (requests.rs, responses.rs), `deserializers.rs` (pub(crate)), `constants.rs`, `date_utils.rs`
- `lib.rs` reeksportuje wszystko flat (`pub use models::*; pub use dto::*; pub use date_utils::*`)
- `date_utils.rs` — 14 pub funkcji oparte na `chrono` (parse_date, add_days, week_range, month_grid_range, is_overdue, sort_by_deadline, itd.)
- `chrono = { version = "0.4.44", default-features = false, features = ["alloc"] }` — WASM-safe, bez std::time clock

### Tracing / Logging

Każdy handler w `crates/server/src/` **musi** mieć `#[instrument]`. W Axum użyj `fields(action = "verb_noun", <entity_id> = %id)` — extractor `Path` daje `id` już na wejściu:

```rust
#[instrument(fields(action = "create_list", list_id = %id))]
pub async fn create(Path(id): Path<String>, ...) -> impl IntoResponse {
    // ...
}
```

- `action` — nazwa operacji w formacie `verb_noun` (`create_list`, `delete_item`, `toggle_item`)
- **Bez `&` przed `tracing::field::display`** — clippy `needless_borrows_for_generic_args` blokuje CI

## Komendy

```bash
just dev          # SSR server + Tailwind watch
just check        # Kompilacja workspace
just build        # Build server + gateway
just test         # cargo test --workspace
just test-e2e     # Playwright e2e (wymaga just dev)
just lint         # Clippy + fmt check
just fmt          # cargo fmt
just ci           # fmt + lint + audit + machete + test
just deploy       # Deploy gateway + auth migrations
just deploy-dev   # Deploy dev environment
```

### Build frontend+server (Leptos SSR)

- **ZAWSZE** `cargo leptos build` — buduje WASM (hydrate) i serwer w spójnych feature'ach.
- **NIGDY** `cargo build -p kartoteka-server` do testów/uruchomienia — default features rozjeżdżają się z WASM i hydration pęka.
- **Do testów**: `cargo leptos build` (debug). `just test-e2e` już to robi.
- **Produkcja**: `cargo leptos build --release`.

## Dokumentacja i aktualne wersje bibliotek

Projekt używa szybko ewoluujących bibliotek (Leptos 0.8, DaisyUI 5). Przed pisaniem kodu sprawdzaj aktualne API przez context7 MCP:

- `mcp__context7__resolve-library-id` — znajdź ID biblioteki (np. "leptos")
- `mcp__context7__query-docs` — pobierz aktualną dokumentację

Używaj tego proaktywnie, nie czekaj na błędy kompilacji.

## Testy

- **Unit testy** (`crates/shared/src/tests/`, `crates/shared/src/date_utils.rs`): deserializery, typy, serde, date_utils — `cargo test -p kartoteka-shared`
- **i18n testy** (`crates/i18n/tests/`): kompletność tłumaczeń PL/EN, parsowanie FTL, pokrycie MCP
- **E2E** (`tests/e2e/`): Playwright, auth flow — `just test-e2e` (wymaga `just dev`)
- CI: `cargo test --workspace`

## CI/CD

- PR workflow (`.github/workflows/ci.yml`): fmt → check → clippy → deny → machete → test
- Security audit (`.github/workflows/security-audit.yml`): weekly cron + on Cargo.lock changes
- Workspace lints w `Cargo.toml` — clippy correctness=deny, suspicious/complexity/perf/style=warn
- `deny.toml` — licencje, advisories, supply chain security
- Lokalne sprawdzenie: `just ci` (uruchamia wszystko naraz)

## Pliki do NIE commitowania

- `.env` — sekrety i konfiguracja
- `target/` — build output
- `.wrangler/`, `build/` — wrangler cache
