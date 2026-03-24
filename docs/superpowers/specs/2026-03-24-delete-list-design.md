# Design: Usuwanie list z potwierdzeniem + system toastów

**Data:** 2026-03-24
**Status:** Zatwierdzony

---

## Zakres

Dodanie możliwości usuwania list z:
- potwierdzeniem przez DaisyUI modal (pokazuje nazwę listy i liczbę elementów)
- przyciskiem usuwania w `ListCard` (strona główna) i `ListPage` (widok detail)
- globalnym systemem toastów (sukces / błąd)
- przekierowaniem na `/` po usunięciu z widoku detail

Brak zmian w API — `api::delete_list` już istnieje po stronie frontendu.

---

## 1. System toastów

### Typy (`app.rs`)

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
    pub fn push(&self, message: String, kind: ToastKind) { ... }  // dodaje toast, auto-dismiss po 3s
    pub fn dismiss(&self, id: u32) { ... }
}
```

### Integracja

- `App` wywołuje `provide_context(ToastContext::new())` **przed** `<Router>` — context dostępny we wszystkich stronach, przeżywa nawigację
- Nowy komponent `components/toast_container.rs` renderowany w `App` obok `<Nav>` i `<main>`
- Auto-dismiss: każdy toast usuwa się po 3000ms przez `set_timeout`
- Użycie w komponentach: `use_context::<ToastContext>().unwrap().push(msg, kind)`

### Wygląd

DaisyUI `<div class="toast toast-end z-50">`. Toast success: `alert-success`. Toast error: `alert-error`.

---

## 2. ConfirmDeleteModal

Nowy komponent `components/confirm_delete_modal.rs`.

### Sygnatura

```rust
#[component]
pub fn ConfirmDeleteModal(
    list_name: String,
    list_id: String,
    show: RwSignal<bool>,
    on_confirm: Callback<()>,
) -> impl IntoView
```

### Zachowanie

- Gdy `show` przejdzie na `true`, komponent asynchronicznie fetchuje item count przez `api::fetch_items(&list_id)`
- Podczas ładowania wyświetla "Wczytywanie szczegółów…"
- Po załadowaniu: "Czy na pewno chcesz usunąć listę **{list_name}**? Zawiera {n} elementów. Operacja jest nieodwracalna."
- [Anuluj] (btn-ghost) → `show.set(false)`
- [Usuń listę] (btn-error) → wywołuje `on_confirm`, rodzic odpowiada za API call i toast
- Implementacja: `<dialog class="modal" open=move || show.get()>` — reaktywny atrybut bez JS interop
- Tło (`modal-backdrop`) zamyka modal przez `show.set(false)`
- Modal renderowany na poziomie strony, nie wewnątrz `ListCard`

---

## 3. Przycisk usuwania w ListCard

### Zmiany w `components/list_card.rs`

- Nowy prop: `#[prop(optional)] on_delete: Option<Callback<String>>`
- Karta (`<div>`) dostaje `relative` w klasach
- Przycisk renderowany gdy `on_delete.is_some()`:

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

- `stop_propagation` blokuje nawigację karty
- Zawsze widoczny, ale subtelny (opacity-40), pełny przy hover — działa na mobile

---

## 4. Przepływ usuwania z HomePage

### Zmiany w `pages/home.rs`

- Lokalny `RwSignal<Option<(String, String)>>` — `pending_delete`: `(list_id, list_name)`
- `ListCard` otrzymuje `on_delete` callback: `|list_id| pending_delete.set(Some((list_id, list.name.clone())))`
- `ConfirmDeleteModal` renderowany warunkowo gdy `pending_delete.is_some()`
- `on_confirm` callback:
  1. Odczytuje `list_id` z `pending_delete`
  2. Optimistic: usuwa listę z lokalnego sygnału `lists`
  3. `api::delete_list(&list_id).await`
  4. Sukces → `toast.push("Lista usunięta", ToastKind::Success)`
  5. Błąd → cofnięcie optimistic update + `toast.push("Błąd usuwania", ToastKind::Error)`
  6. `pending_delete.set(None)`

---

## 5. Przepływ usuwania z ListPage

### Zmiany w `pages/list.rs`

- Lokalny `RwSignal<bool>` — `show_delete_modal`
- Przycisk `[🗑 Usuń listę]` w nagłówku obok tytułu "Lista" (btn-ghost btn-sm)
- `ConfirmDeleteModal` renderowany gdy `show_delete_modal.get()`
- `on_confirm` callback:
  1. `api::delete_list(&list_id).await`
  2. Sukces → `toast.push("Lista usunięta", ToastKind::Success)` + `navigate("/", Default::default())`
  3. Błąd → `toast.push("Błąd usuwania", ToastKind::Error)`
  4. `show_delete_modal.set(false)`

---

## Pliki

| Akcja | Plik |
|-------|------|
| Utwórz | `crates/frontend/src/components/toast_container.rs` |
| Utwórz | `crates/frontend/src/components/confirm_delete_modal.rs` |
| Modyfikuj | `crates/frontend/src/app.rs` |
| Modyfikuj | `crates/frontend/src/components/mod.rs` |
| Modyfikuj | `crates/frontend/src/components/list_card.rs` |
| Modyfikuj | `crates/frontend/src/pages/home.rs` |
| Modyfikuj | `crates/frontend/src/pages/list.rs` |

Brak zmian w API ani w `shared`.
