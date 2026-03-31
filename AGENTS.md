# AGENTS.md

## Repo Basics

- Monorepo: `crates/shared`, `crates/api`, `crates/frontend`, `gateway/`
- API: Cloudflare Workers + D1 (`worker`, `sqlx-d1`)
- Frontend: Leptos 0.8 CSR
- Gateway: TypeScript Worker z MCP i proxy do API

## Tracing / Logging

Każdy handler w `crates/api/src/handlers/` musi mieć `#[instrument]`.

Wzorzec:

```rust
#[instrument(skip_all, fields(action = "create_list", list_id = tracing::field::Empty))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let id = Uuid::new_v4().to_string();
    Span::current().record("list_id", tracing::field::display(&id));
    // ...
}
```

- `skip_all` dla `req` i `ctx`
- `action` w formacie `verb_noun`
- jeśli handler tworzy lub operuje na konkretnym encji ID, dodaj pole jako `tracing::field::Empty` i uzupełnij je przez `Span::current().record(...)` po poznaniu wartości
- bez zbędnego `&` przed `tracing::field::display(...)`

## Workers / D1

- D1 zwraca boolean jako `0.0` / `1.0`; używaj istniejących deserializerów z `crates/shared/src/deserializers.rs`
- D1 bind: `ctx.env.d1("DB")?`
- parametry SQL przekazuj jako `JsValue`
- `Response::empty()?.with_status(204)` zwraca `Response`, więc owiń wynik w `Ok(...)`
- `Headers::new()` nie wymaga `mut`

## Frontend

- W Leptos 0.8 używaj `LocalResource`, nie `Resource`, dla futures opartych o `gloo-net`
- `LocalResource::get()` zwraca `Option<T>` bezpośrednio; typowy wzorzec to `if let Some(Ok(data)) = resource.get()`
- HTTP przechodzi przez `HttpClient` z `crates/frontend/src/api/client.rs`
- przy optymistycznych update'ach używaj snapshot + rollback

## Shared Crate

- `crates/shared` re-eksportuje moduły flat przez `lib.rs`; preferuj importy z `kartoteka_shared::*`
- DTO są w `crates/shared/src/dto/`, modele w `crates/shared/src/models/`
- logika dat powinna reuse'ować helpery z `crates/shared/src/date_utils.rs` i `crates/shared/src/validation.rs`

## Validation / Helpers

- przed dodaniem nowych helperów sprawdź istniejące w `crates/api/src/helpers.rs`
- dla nullable string patchy reuse'uj istniejące konwersje do `JsValue` zamiast dopisywać kolejną lokalną wersję
- dla ownership/list-item relacji preferuj helpery z `helpers.rs`, nie ad-hoc query w handlerach

## Commands

- lokalny smoke test API/shared: `cargo test -p kartoteka-api` i `cargo test -p kartoteka-shared`
- gateway: `cd gateway && npm run typecheck`
- pełniejszy lokalny check: `just ci`
