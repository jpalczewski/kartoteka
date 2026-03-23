# CLAUDE.md

## Projekt

Kartoteka — aplikacja todo/listy na Cloudflare Workers (Rust) + Leptos CSR frontend + Hanko auth.

## Architektura

- **Monorepo**: Cargo workspace (`crates/shared`, `crates/api`, `crates/frontend`) + `mcp/` (TypeScript)
- **API**: CF Worker z `worker` crate (0.7+), D1 database, `sqlx-d1` (0.3+)
- **Frontend**: Leptos 0.7 CSR, kompilowany do WASM przez Trunk, serwowany z CF Pages
- **Auth**: Hanko Cloud — `hanko-init.js` (generowany z template) bridge'uje JS SDK → localStorage → Rust/WASM
- **MCP**: scaffold w `mcp/`, TypeScript, `@cloudflare/workers-oauth-provider` (nie zaimplementowany jeszcze)

## Kluczowe konwencje

### Env vars (compile-time)
- `API_BASE_URL` — URL API workera (dev: `/api`, prod: pełny URL)
- `HANKO_API_URL` — URL Hanko Cloud projektu
- Oba wymagane do kompilacji frontendu i API
- Zarządzane przez `.env` + `set dotenv-load` w justfile

### hanko-init.js
- **Nie commitować** — generowany przez `just _gen-hanko` z `hanko-init.js.template`
- Template w `crates/frontend/hanko-init.js.template`, placeholder: `__HANKO_API_URL__`
- Bridge JS (Hanko SDK) ↔ Rust (localStorage): klucze `hanko_token`, `hanko_user_email`
- Eksponuje `window.__hankoLogout` dla WASM

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
just check        # Kompilacja workspace
just deploy       # Deploy wszystkiego
just lint         # Clippy + fmt check
```

## Pliki do NIE commitowania

- `.env` — sekrety i konfiguracja
- `crates/frontend/hanko-init.js` — generowany z template
- `crates/frontend/dist/` — build output
- `.wrangler/`, `build/` — wrangler cache
