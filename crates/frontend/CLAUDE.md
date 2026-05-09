# Frontend — Leptos 0.8 SSR

Crate `kartoteka-frontend` — komponenty, server functions, routing, i18n.

## Leptos 0.8 — kluczowe wzorce

### Resources
- Używaj `Resource::new`, nie `LocalResource` — SSR futures muszą być `Send`
- `Resource::new(key_fn, async_fn)` — refetch gdy `key_fn` zwróci nową wartość; do statycznych danych użyj `|| ()` jako klucza
- `resource.get()` zwraca `Option<Result<T, _>>` — wzorzec: `if let Some(Ok(data)) = resource.get()`
- `resource.get_untracked()` — odczyt bez subskrypcji reaktywności (np. w `spawn_local`)

### Suspense vs Transition
- `<Suspense>` — pokazuje fallback przy każdym ładowaniu (pierwsze i kolejne)
- `<Transition>` — pokazuje fallback tylko przy pierwszym ładowaniu; przy odświeżeniu trzyma stare dzieci

### Children
- `Children` — przekazywane raz; `ChildrenFn` (`Arc<dyn Fn() -> AnyView>`) — gdy children wołane wielokrotnie (np. w reactive closure `move ||`)
- `ChildrenFn` wymagane w komponentach które renderują children wewnątrz `move ||` lub `Suspense`

### Closures i sygnały
- Sygnały (`ReadSignal`, `WriteSignal`, `RwSignal`, `Memo`) są `Copy` — można je przenosić do wielu `move` closures
- Struktury z `use_location()`, `use_context()` itp. **nie są** `Copy` — wyciągnij potrzebne pola przed closurem:
  ```rust
  let location = use_location();
  let pathname = location.pathname; // Memo<String> — Copy
  let search = location.search;    // Memo<String> — Copy
  let full_path = move || { /* używa pathname i search */ };
  ```
- Non-Copy typy w `Fn` closure → `StoredValue::new()` lub `.clone()` przed wejściem
- `use_context` tylko w ciele komponentu — nie w closures ani `spawn_local`

### Server functions
- `#[server(prefix = "/leptos")]` w `src/server_fns/`
- Ekstrakcja Axum danych: `leptos_axum::extract::<T>().await`
- SSR redirect: `leptos_axum::redirect("/path")` — ustawia header `Location` przed HTML
- Callbacki z async: `leptos::task::spawn_local(async move { ... })`
- Capture `i18n.tr("key")` **przed** `spawn_local` — `I18n` nie jest `Send`

### Routing (leptos_router)
- Layout routes z `<Outlet>`: używaj `<ParentRoute>`, nie `<Route children=...>`
- `<Outlet>` renderuje dopasowane child route w miejscu wywołania

## i18n (leptos-fluent + Fluent)

### Użycie
- `move_tr!("key")` — reaktywna wartość (zwraca closure `impl Fn() -> String`); używaj w `view!`
- `move_tr!("key", { "param" => value })` — z parametrami Fluent (`{ $param }` w .ftl)
- `i18n.tr("key")` — one-shot `String`; używaj w `spawn_local` / callbackach gdzie `move_tr!` nie działa
- `expect_context::<I18n>()` na początku komponentu, przed closurami

### Pliki .ftl
- Wszystkie `.ftl` w katalogu locale (`locales/en/`, `locales/pl/`) **mergowane w jeden bundle** przez `fluent-templates`
- **Duplikat klucza w dwóch plikach → runtime panic**: `Failed to add FTL resources: Overriding { kind: Message, id: "..." }`
- Każdy klucz musi istnieć w dokładnie jednym pliku na locale
- Pliki FTL embedowane **w compile time** — zmiana `.ftl` wymaga przebudowania (nie hot-reload)
- Parametry Fluent: `key = Tekst z { $param }` → `move_tr!("key", { "param" => value })`

### Testy i18n
- `cargo test -p kartoteka-i18n` — sprawdza parzystość kluczy EN/PL, parsowanie FTL, pokrycie MCP
- Uruchamiaj po każdej zmianie w `locales/`
