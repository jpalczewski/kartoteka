# AGENTS.md

## Repo Basics

- Monorepo: `crates/shared`, `crates/db`, `crates/domain`, `crates/auth`, `crates/mcp`, `crates/oauth`, `crates/jobs`, `crates/i18n`, `crates/frontend-v2`, `crates/server`
- API + Frontend: Leptos 0.8 SSR (`crates/server` + `crates/frontend-v2`), Axum, SQLite via `sqlx`
- Auth: Better Auth (cookie-based, email+password + GitHub OAuth)
- DB: SQLite, migracje w `crates/db/migrations/`

## Tracing / Logging

Każdy handler w `crates/server/src/` musi mieć `#[instrument]`.

Wzorzec (Axum — Path extractor daje ID na wejściu):

```rust
#[instrument(fields(action = "create_list", list_id = %id))]
pub async fn create(Path(id): Path<String>, ...) -> impl IntoResponse {
    // ...
}
```

- `action` w formacie `verb_noun`
- Entity ID bezpośrednio w polu (`%id`), nie przez `Span::current().record(...)`
- bez zbędnego `&` przed `tracing::field::display(...)`

## Frontend (Leptos 0.8 SSR)

- Używaj `Resource::new`, nie `LocalResource` — SSR futures muszą być `Send`
- `Resource::get()` zwraca `Option<T>`; wzorzec: `if let Some(Ok(data)) = resource.get()`
- Server functions przez `#[server]` makro w `crates/frontend-v2/src/server_fns/`
- `use_context` tylko w ciele komponentu (nie w closures typu `Fn`)
- Non-Copy typy w `Fn` closure → `StoredValue::new()` lub `.clone()` przed wejściem

## Shared Crate

- `crates/shared` re-eksportuje moduły flat przez `lib.rs`; importuj z `kartoteka_shared::*`
- DTO w `crates/shared/src/dto/`, modele w `crates/shared/src/models/`
- Helpery dat w `crates/shared/src/date_utils.rs`

## DB / Domain

- Logika DB w `crates/db/src/` (sqlx queries)
- Reguły biznesowe w `crates/domain/src/rules/` (czyste funkcje, testowalne bez DB)

## Commands

- lokalny check: `just check`
- testy: `cargo test --workspace`
- pełny CI check: `just ci`
- lokalny dev (SSR + Tailwind): `just dev`

## Git / Commits

- Conventional Commits: `feat: ...`, `fix: ...`, `chore: ...`
