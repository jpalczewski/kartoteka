# AGENTS.md

Wspólny, tool-agnostyczny przewodnik dla agentów (pi, Claude Code) pracujących w tym repo.

## Projekt

Kartoteka — aplikacja todo/listy. Leptos 0.8 SSR (Axum) + SQLite + `axum-login`/`tower-sessions`. MCP server via `rmcp`.

## Architektura

- **Monorepo**: Cargo workspace — `crates/{auth,db,domain,frontend,i18n,jobs,mcp,oauth,server,shared}`
- **Server**: Axum (`crates/server`) — SSR, `/api/*`, middleware auth
- **Frontend**: Leptos 0.8 SSR w `crates/frontend` — komponenty SSR, routing, server functions (`src/server_fns/`), Tailwind (input + `node_modules`)
- **MCP**: `crates/mcp` używa `rmcp` (Rust MCP SDK), wystawiany pod `/mcp` przez główny serwer — bez osobnego gateway
- **Auth**: `crates/auth` — `axum-login` + `tower-sessions` (SQLite store), hasła `argon2`, 2FA `totp-rs`. OAuth providers w `crates/oauth`. Sesja via HttpOnly cookie
- **DB**: SQLite przez `sqlx`, UUID v4 (TEXT) IDs, timestampy `datetime('now')`. Migracje w `crates/db/migrations/` wykonywane przy starcie serwera

## Kluczowe konwencje

### Env vars
- `OAUTH_SIGNING_SECRET` — wymagany do dev (min 32 znaków), zarządzany przez `.env` + `set dotenv-load` w justfile. `.env` to sekrety — nie commitować.

### Frontend (Leptos 0.8 SSR)
Szczegółowe reguły w `crates/frontend/AGENTS.md`. Skrót:
- Build: **zawsze** przez `cargo leptos build` / `just dev` — nigdy `cargo build -p kartoteka-server`
- Server functions: `#[server(prefix = "/leptos")]` w `src/server_fns/`
- Używaj `Resource::new`, nie `LocalResource` — SSR futures muszą być `Send`
- `Resource::get()` zwraca `Option<T>`; wzorzec: `if let Some(Ok(data)) = resource.get()`
- `use_context` tylko w ciele komponentu (nie w closures typu `Fn`)
- Non-Copy typy w `Fn` closure → `StoredValue::new()` lub `.clone()` przed wejściem

### Shared crate (`crates/shared/`)
- `models/`, `dto/` (requests/responses), `deserializers.rs` (pub(crate)), `constants.rs`, `date_utils.rs`
- `lib.rs` reeksportuje flat (`pub use models::*; pub use dto::*; pub use date_utils::*`)
- `chrono` bez `default-features` (`features = ["alloc", "serde"]`) — WASM-safe
- Daty: zobacz `date_utils.rs` (`parse_date`, `week_range`, `month_grid_range`, `sort_by_deadline`, …)

### Tracing / Logging
Każdy handler w `crates/server/src/` **musi** mieć `#[tracing::instrument]`.

Wzorzec (Axum — Path extractor daje ID na wejściu):

```rust
#[tracing::instrument(skip_all, fields(action = "create_list", list_id = %id))]
pub async fn create(Path(id): Path<String>, ...) -> impl IntoResponse {
    // ...
}
```

- `action` w formacie `verb_noun`
- Entity ID bezpośrednio w polu (`%id`), nie przez `Span::current().record(...)`
- Bez `&` przed `tracing::field::display(...)` (clippy `needless_borrows_for_generic_args` blokuje CI)

### DB / Domain
- Logika DB w `crates/db/src/` (sqlx queries)
- Reguły biznesowe w `crates/domain/src/rules/` (czyste funkcje, testowalne bez DB)

## Komendy

```bash
just setup          # cargo-leptos + wasm target + npm install (crates/frontend)
just dev            # dev-leptos + dev-tailwind w parallel (trap/wait)
just check          # cargo check --workspace
just check-ssr      # cargo check bez WASM — szybszy feedback
just check-fe       # check frontendu: ssr + hydrate (wasm32) + kompilacja testów
just build          # npm install + cargo leptos build --release
just test           # nextest workspace + doctesty
just t <crate>      # nextest jednego crate'a
just c <crate>      # clippy jednego crate'a (frontend: ssr + hydrate/wasm32)
just verify         # pełne sprawdzenie przed pushem (fmt, lint, test, check-fe)
just test-e2e       # Playwright (serwer startuje sam Playwright)
just lint           # Clippy (z testami) + fmt check
just fmt            # cargo fmt
just ci             # fmt + lint + audit + machete + test
just deploy-preview # build AMD64 obraz lokalnie + push :preview do GHCR (bez deploya)
```

## Weryfikacja — tylko przez `just`, bez własnych kombinacji flag

Każda nowa kombinacja `--features` / `--all-targets` / `--tests` rekompiluje całe drzewo na współdzielonym hoście — używaj wyłącznie gotowych przepisów.

| Kiedy | Komenda | Co robi |
| --- | --- | --- |
| w trakcie pracy | `just t <crate>` | nextest jednego crate'a |
| w trakcie pracy, lint | `just c <crate>` | clippy jednego crate'a (frontend: ssr + hydrate/wasm32) |
| zmiana w `crates/frontend` | `just check-fe` | check ssr + hydrate (wasm32) + kompilacja testów frontendu |
| przed pushem, raz | `just verify` | fmt, clippy (z testami), nextest workspace, check-fe; to samo robi lefthook pre-push tam, gdzie lefthook jest zainstalowany |
| tylko na prośbę | `just test-e2e` | Playwright, port z `E2E_PORT` |

- Nigdy `cargo test` — używaj nextest (`just test` / `just t <crate>`)
- Nie dodawaj sam `--all-targets` — robi to `just lint`
- Po rebase bez konfliktów w twoich crate'ach nie odpalaj nic lokalnie — pokrywa to CI
- Z konfliktami: `just t <crate>` dla dotkniętych crate'ów i `just verify` raz na końcu
- Flaky test: max 3 powtórzenia przez `just t <crate> -E 'test(name)'`, potem zgłoś — nie zapętlaj

## Testy

- Unit (`crates/shared/`): `just t shared`
- i18n (`crates/i18n/tests/`): kompletność PL/EN, parsowanie FTL, pokrycie MCP
- E2E (`tests/e2e/`): Playwright, 9 spec files — `just test-e2e`
  - `just test-e2e` robi `build-css` + `cargo leptos build`, potem `npx playwright test`; serwer aplikacji
    startuje sam Playwright (`tests/playwright.config.ts`), więc nie trzeba uruchamiać `just dev`
  - binarka: `target/debug/kartoteka` (albo `$CARGO_TARGET_DIR`), nadpisywalna przez `$KARTOTEKA_BIN`;
    port: 3030, nadpisywalny przez `$E2E_PORT` (kilka checkoutów obok siebie)
  - `tests/test.db` jest kasowany **przed** startem serwera (migracje lecą przy starcie), a
    `LEPTOS_HASH_FILES=true` jest ustawiane w env serwera — bez tego hydracja nie działa
  - `@playwright/test` jest przypięty do `1.58.2` (nie `^`), bo ta wersja używa
    `chromium-headless-shell-1208`, który jest zainstalowany na maszynach dev/CI; podbicie do
    `1.60+` wymaga innej rewizji przeglądarki (`npx playwright install chromium-headless-shell`)
  - helpery (`registerAndLogin`, `createList`, `addItem`, …) są w `tests/e2e/helpers.ts`; nowe
    selektory opieraj na `data-testid`, nie na przetłumaczonych placeholderach/etykietach

## Praca równoległa (wiele wątków i PR-ów naraz)

- Merge tylko przez rebase/squash (merge commity wyłączone); jeden wątek-koordynator trzyma kolejność merge'y do develop — zgłoś mu swój PR, nie merguj poza kolejnością
- Zarezerwuj numer migracji u koordynatora przed jej utworzeniem (była już kolizja 017)
- Pliki tymczasowe: `.scratch/` w worktree (gitignored), nigdy `/tmp`
- Procesy: `pgrep -x kartoteka`, nie `pkill -f` (zabija twoją własną powłokę)
- Nie buduj w głównym checkoutcie `~/kartoteka`, tylko w swoim worktree

## Git / Commits

- Conventional Commits: `feat: ...`, `fix: ...`, `chore: ...`

## Deployment / CI

- CI: `.github/workflows/` — `ci.yml` (fmt, clippy+testy, deny, machete), CodeQL, obrazy Docker (`docker-build.yml` reusable); szczegóły w plikach workflow.
- Główna aplikacja: binarka + site budowane natywnie w CI (`docker-build.yml`, `cargo leptos build --release`), pakowane przez `Dockerfile.runtime`, wypychane do GHCR (`:develop`, `:preview`, `:latest` + tag wersji). Repo nie robi deploya; `Dockerfile` (multi-stage) zostaje do buildów lokalnych
- Preview: `just deploy-preview` (AMD64 build via Colima, tylko push do GHCR)
