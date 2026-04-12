# Plan 3: Frontend SSR — Design Spec

Parent: `docs/superpowers/specs/2026-04-12-cloudflare-exit-v2-design.md`
Depends on: Plan 1 (crates/db), Plan 2 (crates/server)

## Goal

Migrate `crates/frontend` from Leptos CSR (Trunk) to Leptos 0.8 SSR (cargo-leptos) with hydration. Server functions replace gloo-net HTTP calls. Route-by-route migration. Build toolchain changes from Trunk to cargo-leptos.

## Architecture

### Dual compilation

`crates/frontend` compiles twice:
- **SSR** (`feature = "ssr"`) — imported by `crates/server`, renders HTML on server
- **Hydrate** (`feature = "hydrate"`) — compiled to WASM, hydrates HTML in browser

cargo-leptos orchestrates both builds via workspace metadata:

```toml
# root Cargo.toml
[[workspace.metadata.leptos]]
name = "kartoteka"
bin-package = "kartoteka-server"
lib-package = "kartoteka-frontend"
site-root = "target/site"
site-pkg-dir = "pkg"
site-addr = "127.0.0.1:3000"
reload-port = 3001
bin-features = ["ssr"]
lib-features = ["hydrate"]
assets-dir = "crates/frontend/public"
style-file = "crates/frontend/style/main.css"
hash-files = true
wasm-opt-features = ["-Oz", "--enable-bulk-memory"]
```

Build: `cargo leptos build --release --precompress` generates .gz + .br for all static assets. Hashed filenames enable immutable caching.

### Tailwind 4 + DaisyUI 5

cargo-leptos nie wspiera natywnie Tailwind 4 (issue #522 open). Workaround:
- `style-file = "style/main.css"` — cargo-leptos traktuje jako statyczny CSS
- Tailwind 4 CLI odpalany osobno: `npx @tailwindcss/cli -i style/input.css -o style/main.css --watch`
- W `just dev`: dwa procesy — `cargo leptos watch` + tailwind watch
- W CI: `npx @tailwindcss/cli -i style/input.css -o style/main.css` przed `cargo leptos build`

Plik `style/input.css` bez zmian — `@import "tailwindcss"`, `@source "../src/**/*.rs"`, `@plugin "daisyui"`, neon-night theme.

### Server functions

Zastępują `crates/frontend/src/api/` (gloo-net HTTP calls). Każdy obecny API call staje się `#[server]` function.

```rust
// Przed (CSR):
pub async fn fetch_lists() -> Result<Vec<List>, String> {
    get(&format!("{API_BASE}/lists")).send().await?.json().await
}

// Po (SSR) — all access through domain::
#[server]
pub async fn fetch_lists() -> Result<Vec<List>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth: AuthSession = extract_with_state(&expect_context::<AppState>()).await?;
    let user = auth.user.ok_or(ServerFnError::new("unauthorized"))?;
    Ok(domain::lists::list_all(&pool, &user.id).await?)
}

#[server]
pub async fn create_list(req: CreateListRequest) -> Result<List, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth: AuthSession = extract_with_state(&expect_context::<AppState>()).await?;
    let user = auth.user.ok_or(ServerFnError::new("unauthorized"))?;
    Ok(domain::lists::create(&pool, &user.id, &req).await?)
}
```

Server functions żyją w `crates/frontend/src/server_fns/` za `#[cfg(feature = "ssr")]`. Kompilują się tylko w server build, nie w WASM.

### Resource changes

```rust
// Przed (CSR):
let lists = LocalResource::new(|| api::fetch_lists());

// Po (SSR):
let lists = Resource::new(|| (), |_| fetch_lists());
```

`Resource` (nie `LocalResource`) — dane serializowane z server do client przy initial render. Brak ponownego fetch po hydration.

### Browser-only code

Kod wymagający `web_sys` (clipboard, localStorage, window.location) musi mieć `#[cfg(feature = "hydrate")]` guardy:

```rust
#[cfg(feature = "hydrate")]
fn copy_to_clipboard(text: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.navigator().clipboard().write_text(text);
    }
}
```

### i18n (leptos-fluent SSR)

Zmiana z localStorage na cookie:

```rust
leptos_fluent! {
    locales: "../../locales",
    set_language_to_cookie: true,
    initial_language_from_cookie: true,
    initial_language_from_navigator: true,
    initial_language_from_accept_language_header: true,  // SSR
    cookie_name: "lang",
}
```

Feature flags w `crates/frontend/Cargo.toml`:
```toml
[features]
hydrate = ["leptos/hydrate", "leptos-fluent/hydrate"]
ssr = ["leptos/ssr", "leptos-fluent/ssr", "leptos-fluent/axum", "dep:kartoteka-db"]
```

FTL files bez zmian.

### Timezone

User's timezone stored in `user_settings` (key: `timezone`, default: `"UTC"`). 

- **Settings page:** timezone picker (dropdown with common timezones, e.g. `Europe/Warsaw`)
- **Date display:** server functions use `domain::` which resolves dates in user's timezone via `chrono-tz`. Frontend receives already-resolved dates — no client-side timezone conversion needed.
- **"Today" view / calendar:** `domain::items::by_date` resolves "today" using user's timezone from settings. Frontend just passes `"today"` or a date string.

### Data flow: always through domain::

Server functions always call `domain::`, never `db::` directly. This ensures centralized access control and a single expansion point for future features (field filtering, audit logging, sharing).

Same rule applies to REST handlers and MCP tools.

## Crate structure changes

### frontend/Cargo.toml

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
kartoteka-shared = { path = "../shared", version = "0.1.1" }
kartoteka-i18n = { path = "../i18n", version = "0.1.1" }
kartoteka-domain = { path = "../domain", optional = true }
leptos = { version = "0.8", default-features = false }
leptos_router = { version = "0.8", default-features = false }
leptos_meta = { version = "0.8" }
leptos-fluent = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", default-features = false }

# Hydrate-only
wasm-bindgen = { version = "0.2", optional = true }
web-sys = { version = "0.3", optional = true, features = ["Storage", "Window", "Navigator", "Clipboard", "Location"] }
console_error_panic_hook = { version = "0.1", optional = true }
gloo-timers = { version = "0.3", optional = true, features = ["futures"] }

# SSR-only
leptos_axum = { version = "0.8", optional = true }
axum = { version = "0.8", optional = true }
axum-login = { version = "0.18", optional = true }
sqlx = { version = "0.8", optional = true, features = ["sqlite"] }

[features]
hydrate = [
    "leptos/hydrate",
    "leptos-fluent/hydrate",
    "dep:wasm-bindgen",
    "dep:web-sys",
    "dep:console_error_panic_hook",
    "dep:gloo-timers",
]
ssr = [
    "leptos/ssr",
    "leptos_router/ssr",
    "leptos_meta/ssr",
    "leptos-fluent/ssr",
    "leptos-fluent/axum",
    "dep:leptos_axum",
    "dep:axum",
    "dep:axum-login",
    "dep:sqlx",
    "dep:kartoteka-domain",
]
```

### Usunięcia

- `gloo-net` — zastąpiony przez server functions
- `send_wrapper` — niepotrzebny w SSR (Resource jest Send)
- `wasm-bindgen-futures` — niepotrzebny (server functions)
- `js-sys` — tylko jeśli hydrate-only code go potrzebuje
- `crates/frontend/src/api/` — cały katalog znika
- `crates/frontend/Trunk.toml` — zastąpiony przez cargo-leptos metadata
- `crates/frontend/index.html` — zastąpiony przez `shell()` function

### Nowy plik: shell()

```rust
// crates/frontend/src/lib.rs
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="pl" data-theme="neon-night">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
                <link rel="stylesheet" href="/pkg/kartoteka.css"/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
```

## File structure (new)

```
crates/frontend/src/
  lib.rs               — shell(), hydrate(), mod declarations
  app.rs               — App component, Router, routes, ToastContext
  server_fns/          — NEW: #[server] functions (replace api/)
    mod.rs
    containers.rs
    items.rs
    lists.rs
    tags.rs
    settings.rs
    preferences.rs
    home.rs
    auth.rs            — get_session (server-side)
  pages/               — bez zmian w strukturze
    home.rs            — refactor: rozbić na sekcje (521 LOC)
    list/
    calendar/
    container.rs
    item_detail.rs     — refactor: add comments section
    container.rs       — add comments section
    list/              — add comments section
    today.rs
    tags/
    login.rs           — zmiana: server function zamiast gloo-net
    signup.rs          — zmiana: server function zamiast gloo-net
    settings.rs        — language, timezone, MCP URL, admin: registration toggle
    oauth_consent.rs
  components/          — bez zmian w strukturze, drobne refaktory
    common/
      date_utils.rs    — USUNĄĆ, przeniesione do crates/shared (chrono)
    items/
      add_item_input.rs — refactor: wydzielić item_fields.rs (431 LOC)
      item_fields.rs   — NEW: quantity, unit, date fields (reużywalne)
    lists/
    tags/
    calendar/
    filters/
    comments/          — NEW: reusable comments section
      comment_list.rs  — chronological display, user vs assistant styling
      add_comment.rs   — input form
    home/              — NEW: sekcje home page
      pinned_section.rs
      recent_section.rs
      root_section.rs
    nav.rs
    sync_locale.rs
  state/               — z Leptos 0.8 branch
    mod.rs
    transforms.rs      — with_item_toggled, without_item
```

### Co znika

- `api/` — cały katalog (750 LOC gloo-net calls)
- `main.rs` — zastąpiony przez `lib.rs` z `shell()` + `hydrate()`
- `Trunk.toml` — zastąpiony przez cargo-leptos
- `index.html` — zastąpiony przez `shell()`
- `components/common/date_utils.rs` — przeniesiony do `crates/shared` (chrono, z Leptos 0.8 branch)

### Refaktory przy okazji migracji

1. **`add_item_input.rs` (431 LOC)** → wydzielić `item_fields.rs` (quantity, unit, dates) — reużywalne w `item_detail.rs`
2. **`home.rs` (521 LOC)** → rozbić na `components/home/{pinned,recent,root}_section.rs`
3. **`date_utils.rs`** → już w `crates/shared` (Leptos 0.8 branch, chrono)
4. **state transforms** → z Leptos 0.8 branch (`with_item_toggled`, `without_item`)

### New feature: "All items" view

New page `/all` — cross-list view of all items across all lists. Like "Today" but without date filter. Grouped by list, searchable, filterable by tag. Uses `domain::items::list_all_for_user(pool, user_id, filters)` — new domain function.

### Czego NIE ruszamy

- `list/mod.rs` (489 LOC) — natural complexity, split byłby sztuczny
- Struktura components/items/, tags/, calendar/ — działa dobrze
- Wszystkie komponenty common/ — małe i reużywalne
- Routing (te same ścieżki)
- CSS/DaisyUI/neon-night theme

## Migration strategy

Route-by-route. Nie trzeba migrować wszystkiego naraz.

1. Shell + App + Router — działa jako SSR z pustymi stronami
2. Home page — pierwszy pełny route z server functions
3. Lists page — najbardziej złożony route
4. Reszta route po route
5. Auth pages (login/signup) — server functions zamiast gloo-net
6. Usunięcie api/ i gloo-net dependency

Każdy route po migracji jest w pełni SSR — server-rendered HTML + hydration na kliencie.

## Dev experience

```bash
# Dwa procesy (w just dev):
cargo leptos watch                                          # Axum + WASM hot reload
npx @tailwindcss/cli -i style/input.css -o style/main.css --watch  # Tailwind 4
```

Zastępuje obecne 3 procesy (wrangler API + wrangler gateway + trunk).

## Testing

- Server functions testowalne na native target (nie WASM)
- State transforms testowalne natywnie (z Leptos 0.8 branch — 74 testy)
- Components: brak unit testów (Leptos CSR i SSR nie mają dobrego testing story)
- E2E: Playwright jak teraz, ale na `localhost:3000` zamiast Trunk proxy

## What this plan does NOT include

- MCP server (Plan 4)
- OAuth provider (Plan 4)
- Deploy (Plan 5)
- SSR streaming / islands (future optimization)
