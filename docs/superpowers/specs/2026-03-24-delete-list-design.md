# Design: Usuwanie list z potwierdzeniem + system toastów

**Data:** 2026-03-24
**Status:** Zatwierdzony (po spec review)

---

## Zakres

Dodanie możliwości usuwania list z:
- potwierdzeniem przez DaisyUI modal (pokazuje nazwę listy i liczbę elementów)
- przyciskiem usuwania w `ListCard` (strona główna) i `ListPage` (widok detail)
- globalnym systemem toastów (sukces / błąd)
- przekierowaniem na `/` po usunięciu z widoku detail

Cascade delete jest zapewniony przez SQLite `ON DELETE CASCADE` w migracji — brak zmian w API.

---

## 1. Naprawa `api::delete_list`

Aktualny `api::delete_list` nie sprawdza HTTP status code — zawsze zwraca `Ok(())`. Należy to naprawić **jako pierwszy krok** przed budowaniem logiki usuwania:

```rust
pub async fn delete_list(id: &str) -> Result<(), String> {
    let resp = del(&format!("{API_BASE}/lists/{id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(format!("Błąd serwera: {}", resp.status()))
    }
}
```

---

## 2. System toastów

### Typy i context (`app.rs`)

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum ToastKind { Success, Error }

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u32,
    pub message: String,
    pub kind: ToastKind,
}

#[derive(Clone, Copy)]
pub struct ToastContext {
    pub toasts: RwSignal<Vec<Toast>>,
    next_id: RwSignal<u32>,
}

impl ToastContext {
    pub fn new() -> Self { ... }
    pub fn push(&self, message: String, kind: ToastKind) { ... }  // dodaje + auto-dismiss 3s via set_timeout
    pub fn dismiss(&self, id: u32) { ... }
}
```

`set_timeout` to `leptos::leptos_dom::helpers::set_timeout` (lub re-eksport z `leptos`).

### Integracja w `app.rs`

`provide_context(ToastContext::new())` musi być wywołane **przed** `<Router>`. `ToastContainer` renderowany **poza** `<Routes>` (żeby nie unmountował się przy nawigacji):

```rust
pub fn App() -> impl IntoView {
    let toast_ctx = ToastContext::new();
    provide_context(toast_ctx);
    view! {
        <Router>
            <Nav/>
            <ToastContainer/>   // poza Routes, wewnątrz Router
            <main class="container">
                <Routes ...>
```

### Komponent `components/toast_container.rs`

DaisyUI `<div class="toast toast-end z-50">`. Iteruje po `toasts`, każdy z `class="alert alert-success"` lub `alert-error`. Każdy toast ma przycisk ✕ do ręcznego zamknięcia.

---

## 3. ConfirmDeleteModal

Nowy komponent `components/confirm_delete_modal.rs`.

### Sygnatura

```rust
#[component]
pub fn ConfirmDeleteModal(
    list_name: String,
    list_id: String,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView
```

Brak `show: RwSignal<bool>` w propsach — widoczność kontrolowana przez **warunkowe renderowanie** w rodzicu (`{move || condition.then(|| view! { <ConfirmDeleteModal .../> })}`). Dzięki temu każde zamontowanie modala to świeży fetch.

### Zachowanie

- Przy zamontowaniu asynchronicznie fetchuje items przez `api::fetch_items(&list_id)` (lokalny `RwSignal<FetchState>` z wariantami `Loading | Loaded(usize) | Error`)
- Stan Loading: "Wczytywanie szczegółów…"
- Stan Error: "Nie udało się pobrać szczegółów." (i tak można usunąć)
- Stan Loaded: "Czy na pewno chcesz usunąć listę **{list_name}**? Zawiera {n} elementów. Operacja jest nieodwracalna."
- Przycisk [Usuń listę] (btn-error): wywoływany callback `on_confirm`. Przycisk **wyłączony** (`disabled`) gdy `deleting` sygnał jest `true` — rodzic ustawia deleting przez czas API call.
- Przycisk [Anuluj] (btn-ghost): wywołuje `on_cancel`
- Backdrop (`modal-backdrop`) → wywołuje `on_cancel`

### W `ListPage`

`ListPage` już ma załadowane items (`items: RwSignal<Vec<Item>>`). Modal dostaje count z `items.read().len()` zamiast fetchować — jako dodatkowy optional prop `item_count: Option<usize>`. Gdy `Some`, pomija fetch.

---

## 4. Przycisk usuwania w ListCard

### Zmiany w `components/list_card.rs`

- Nowy prop: `#[prop(optional)] on_delete: Option<Callback<String>>`
- Główny `<div class="card ...">` dostaje `relative` w klasach
- Przycisk renderowany gdy `on_delete.is_some()`, **poza** wrapperem `on:click=stop_propagation` dla tagów:

```rust
<button
    type="button"
    aria-label="Usuń listę"
    class="btn btn-ghost btn-xs absolute top-2 right-2 opacity-40 hover:opacity-100"
    on:click=move |ev| {
        ev.stop_propagation();
        if let Some(cb) = &on_delete { cb.run(list.id.clone()); }
    }
>
    "🗑"
</button>
```

Zawsze widoczny, subtelny (opacity-40), pełny przy hover — działa na mobile.

---

## 5. Przepływ usuwania z HomePage

### Zmiany w `pages/home.rs`

**Architektura list signal:** Aktualnie `lists` jest `LocalResource`. Należy dodać `lists_data: RwSignal<Vec<List>>` synchronizowany z resource (ten sam pattern co `list_tag_links`), żeby umożliwić optimistic update.

```rust
let lists_res = LocalResource::new(move || { let _ = refresh.get(); api::fetch_lists() });
let lists_data = RwSignal::new(Vec::<List>::new());
Effect::new(move |_| {
    if let Some(data) = lists_res.get() {
        if let Ok(lists) = data.as_deref() { lists_data.set(lists.to_vec()); }
    }
});
```

**Logika usuwania:**
- `pending_delete: RwSignal<Option<(String, String)>>` — `(list_id, list_name)`
- `deleting: RwSignal<bool>` — blokuje przycisk w modalu podczas API call
- `ListCard` otrzymuje `on_delete` callback → `pending_delete.set(Some(...))`
- Modal renderowany warunkowo:

```rust
{move || pending_delete.get().map(|(lid, lname)| view! {
    <ConfirmDeleteModal
        list_id=lid.clone()
        list_name=lname
        on_confirm=Callback::new(move |_| {
            deleting.set(true);
            spawn_local(async move {
                // Optimistic update
                let removed = lists_data.read().iter().find(|l| l.id == lid).cloned();
                lists_data.update(|ls| ls.retain(|l| l.id != lid));

                match api::delete_list(&lid).await {
                    Ok(()) => toast.push("Lista usunięta".into(), ToastKind::Success),
                    Err(e) => {
                        // Rollback
                        if let Some(list) = removed { lists_data.update(|ls| ls.push(list)); }
                        toast.push(format!("Błąd: {e}"), ToastKind::Error);
                    }
                }
                deleting.set(false);
                pending_delete.set(None);
            });
        })
        on_cancel=Callback::new(move |_| pending_delete.set(None))
    />
})}
```

---

## 6. Przepływ usuwania z ListPage

### Zmiany w `pages/list.rs`

- `show_delete: RwSignal<bool>`
- `deleting: RwSignal<bool>`
- Przycisk `[🗑 Usuń listę]` (btn-ghost btn-sm) w nagłówku obok "Lista"
- Modal z `item_count=Some(items.read().len())` — bez dodatkowego fetcha

```rust
on_confirm=Callback::new(move |_| {
    deleting.set(true);
    let lid = list_id();
    spawn_local(async move {
        match api::delete_list(&lid).await {
            Ok(()) => {
                toast.push("Lista usunięta".into(), ToastKind::Success);
                navigate("/", Default::default());
            }
            Err(e) => {
                toast.push(format!("Błąd: {e}"), ToastKind::Error);
                deleting.set(false);
                show_delete.set(false);
            }
        }
    });
})
```

Po sukcesie `deleting` nie jest resetowane bo komponent przestaje istnieć po nawigacji.

---

## Pliki

| Akcja | Plik |
|-------|------|
| Modyfikuj | `crates/frontend/src/api.rs` — fix delete_list status check |
| Utwórz | `crates/frontend/src/components/toast_container.rs` |
| Utwórz | `crates/frontend/src/components/confirm_delete_modal.rs` |
| Modyfikuj | `crates/frontend/src/app.rs` — provide_context + ToastContainer |
| Modyfikuj | `crates/frontend/src/components/mod.rs` — nowe moduły |
| Modyfikuj | `crates/frontend/src/components/list_card.rs` — on_delete prop + przycisk |
| Modyfikuj | `crates/frontend/src/pages/home.rs` — lists_data RwSignal + modal + delete logic |
| Modyfikuj | `crates/frontend/src/pages/list.rs` — modal + delete logic |
