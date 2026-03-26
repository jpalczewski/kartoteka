# M2: Widok kalendarza (miesiąc + tydzień)

## Problem

Brak nawigacji po dniach z lotu ptaka. Jedyny widok cross-list to `/today` — nie widać co jest zaplanowane na przyszłe dni, nie można przeglądać historii. Dodatkowo `list.rs` (525 linii) wymaga rozbicia, a `components/` i `pages/` rosną flat bez organizacji.

## Rozwiązanie

Widok kalendarza z przełącznikiem miesiąc/tydzień + strona widoku dnia. Równolegle: reorganizacja folderów w frontend i wydzielenie reużywalnych komponentów.

---

## Backend

### Nowy endpoint: `GET /api/items/calendar`

**Parametry:**
- `from` (required) — YYYY-MM-DD, początek zakresu
- `to` (required) — YYYY-MM-DD, koniec zakresu
- `detail` (optional) — `counts` (default) | `full`

**Response dla `detail=counts`:**
```json
[
  { "date": "2026-03-24", "total": 5, "completed": 2 },
  { "date": "2026-03-25", "total": 3, "completed": 3 }
]
```
Tylko dni które mają itemy (nie zwraca pustych dni).

**Response dla `detail=full`:**
```json
[
  {
    "date": "2026-03-24",
    "items": [
      { "id": "...", "list_id": "...", "title": "...", ... }
    ]
  }
]
```
Items to `DateItem` — ten sam struct co w `by_date`.

**SQL dla counts:**
```sql
SELECT i.due_date as date,
       COUNT(*) as total,
       CAST(SUM(i.completed) AS INTEGER) as completed
FROM items i
JOIN lists l ON l.id = i.list_id
WHERE l.user_id = ?1 AND l.archived = 0
  AND i.due_date >= ?2 AND i.due_date <= ?3
GROUP BY i.due_date
ORDER BY i.due_date ASC
```

**SQL dla full:** taki sam SELECT jak w `by_date`, ale `WHERE i.due_date >= ?from AND i.due_date <= ?to`.

### Nowe typy w `crates/shared/src/lib.rs`

```rust
/// Helper: D1 returns COUNT/SUM as float — deserialize to u32
fn u32_from_number<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let v: f64 = serde::Deserialize::deserialize(d)?;
    Ok(v as u32)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaySummary {
    pub date: String,
    #[serde(deserialize_with = "u32_from_number")]
    pub total: u32,
    #[serde(deserialize_with = "u32_from_number")]
    pub completed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayItems {
    pub date: String,
    pub items: Vec<DateItem>,
}
```

**Uwaga:** D1 zwraca `COUNT(*)` i `SUM()` jako float — custom deserializer `u32_from_number` analogiczny do istniejącego `bool_from_number`.

### Pliki do modyfikacji

- `crates/shared/src/lib.rs` — dodanie `DaySummary`, `DayItems`
- `crates/api/src/handlers/items.rs` — nowy handler `calendar`
- `crates/api/src/router.rs` — nowa trasa `.get_async("/api/items/calendar", items::calendar)` (zarejestrować obok istniejącego `/api/items/by-date`, przed ewentualnymi wildcard routes)
- `crates/frontend/src/api/items.rs` — nowe funkcje `fetch_calendar_counts`, `fetch_calendar_full`

---

## Frontend — nowe strony

### Trasy

| Route | Komponent | Opis |
|-------|-----------|------|
| `/calendar` | `CalendarPage` | Miesiąc/tydzień z toggle |
| `/calendar/:date` | `CalendarDayPage` | Widok dnia z prev/next |

### `CalendarPage` (`pages/calendar/mod.rs`)

**Stan:**
- `view_mode: RwSignal<ViewMode>` — `Month` | `Week`
- `current_date: RwSignal<String>` — punkt odniesienia (YYYY-MM-DD)
- `data: RwSignal<...>` — dane kalendarza (counts lub full items)
- Filter state: `hidden_lists`, `hidden_tags`, `show_completed` — reuse z `FilterChips`

**Logika:**
- Oblicza `from`/`to` na podstawie `view_mode` i `current_date`
  - Month: pierwszy poniedziałek przed 1. dniem miesiąca → ostatnia niedziela po ostatnim dniu
  - Week: poniedziałek → niedziela tygodnia zawierającego `current_date`
- Fetch: `detail=counts` dla Month, `detail=full` dla Week
- Renderuje `CalendarNav` + `MonthGrid` lub `WeekView`

### `CalendarDayPage` (`pages/calendar/day.rs`)

**Reuse z TodayPage:**
- Fetch `by-date?date=:date&include_overdue=false` (bez overdue — to nie "dziś")
- Renderuje grupy per lista z `DateItemRow`
- Nawigacja prev/next day (strzałki)
- Nagłówek: `format_polish_date` + dzień tygodnia

---

## Frontend — nowe komponenty

### `MonthGrid` (`components/calendar/month_grid.rs`)

**Props:** `counts: Vec<DaySummary>`, `year: i32`, `month: u32`, `today: String`

**Render:**
- Nagłówki: Pn Wt Śr Cz Pt Sb Nd
- Kratki 7×5/6, dni spoza miesiąca wyszarzone
- Każda kratka: numer dnia + indicator
  - Brak itemów: brak kropki
  - Wszystkie completed: zielona kropka
  - Częściowo: żółta kropka
  - Overdue (date < today && completed < total): czerwona kropka
- Dzisiejszy dzień: ring/outline highlight
- Klik → nawigacja do `/calendar/YYYY-MM-DD`

### `WeekView` (`components/calendar/week_view.rs`)

**Props:** `days: Vec<DayItems>`, `today: String`, `all_tags: Vec<Tag>`, `item_tag_links: Vec<ItemTagLink>`, `items_signal: RwSignal<Vec<DayItems>>`, filter signals

**Dane tagów:** `CalendarPage` fetchuje `all_tags` i `item_tag_links` (tak jak TodayPage) i przekazuje do `WeekView`. `WeekView` filtruje linki per item i przekazuje do `DateItemRow`.

**Signal architecture:** `CalendarPage` trzyma `RwSignal<Vec<DayItems>>`. Optimistic toggle callback: `items_signal.update(|days| { if let Some(day) = days.iter_mut().find(|d| d.date == target_date) { if let Some(item) = day.items.iter_mut().find(|i| i.id == item_id) { item.completed = !item.completed; } } })`.

**Render:**
- 7 kolumn (desktop), stack (mobile)
- Nagłówek per kolumna: "Pn 24 mar" (dzień tygodnia + short date)
- Dzisiejsza kolumna wizualnie wyróżniona
- Lista itemów per dzień z `DateItemRow` — pełne checkbox, tytuł, tagi, czas
- Optimistic toggle via nested signal update (jak opisano wyżej)

### `CalendarNav` (`components/calendar/calendar_nav.rs`)

**Props:** `current_date`, `view_mode`, on_prev, on_next

**Render:**
- `< [tytuł] >` — prev/next
  - Month: "Marzec 2026"
  - Week: "24-30 mar 2026"
- Toggle: "Miesiąc | Tydzień" (btn-group)
- Przycisk "Dziś" — reset do bieżącej daty

### `FilterChips` (`components/filters/filter_chips.rs`)

**Extracted z TodayPage.** Reużywalny komponent filtrów lista/tag/completed.

**Props:** `unique_lists: Vec<(String, String)>`, `relevant_tags: Vec<Tag>`, `hidden_lists: RwSignal<HashSet<String>>`, `hidden_tags: RwSignal<HashSet<String>>`, `show_completed: RwSignal<bool>`

**Uwaga:** `unique_lists` i `relevant_tags` są obliczane przez caller (nie wewnątrz komponentu) — dzięki temu `FilterChips` nie zależy od `Vec<DateItem>`. W widoku miesiąca `FilterChips` nie jest renderowany (brak danych itemów). Alternatywnie: osobny fetch list użytkownika do filtrowania w widoku miesiąca (backlog, nie M2).

---

## Refaktoryzacja

### 1. Reorganizacja folderów

**`pages/` — nowa struktura:**
```
pages/
  mod.rs
  home.rs
  login.rs
  settings.rs
  today.rs
  tags/
    mod.rs          ← TagsPage (z pages/tags.rs)
    detail.rs       ← TagDetailPage (z pages/tag_detail.rs)
  list/
    mod.rs          ← ListPage (z pages/list.rs)
    date_view.rs    ← render_date_view
    normal_view.rs  ← render_normal_view + NormalViewProps
  calendar/
    mod.rs          ← CalendarPage (nowe)
    day.rs          ← CalendarDayPage (nowe)
```

**`components/` — nowa struktura:**
```
components/
  mod.rs
  nav.rs
  common/
    mod.rs
    toast_container.rs
    confirm_delete_modal.rs
    editable_title.rs
    editable_description.rs
    editable_color.rs
    date_utils.rs        ← wydzielone z date_item_row.rs
  items/
    mod.rs
    item_row.rs
    date_item_row.rs     ← tylko komponent DateItemRow
    item_actions.rs
    add_item_input.rs
    add_input.rs
  tags/
    mod.rs
    tag_badge.rs
    tag_list.rs
    tag_selector.rs
    tag_tree.rs
    tag_filter_bar.rs
  lists/
    mod.rs
    list_card.rs
    list_header.rs
    list_tag_bar.rs
    sublist_section.rs
    add_group_input.rs
  calendar/
    mod.rs
    month_grid.rs
    week_view.rs
    calendar_nav.rs
  filters/
    mod.rs
    filter_chips.rs
```

### 2. Wydzielenie date utilities

Z `components/date_item_row.rs` do `components/common/date_utils.rs`:
- `get_today_string`, `get_today`
- `polish_month_abbr`, `format_date_short`
- `days_in_month`, `date_to_days`, `relative_date`
- `current_time_hhmm`
- `is_overdue`, `is_upcoming`, `sort_by_due_date`

Z `pages/today.rs` do `components/common/date_utils.rs`:
- `format_polish_date`

**Nowe funkcje w `date_utils.rs` (potrzebne dla kalendarza):**
- `day_of_week(date_str: &str) -> u32` — 0=Pn, 6=Nd (via `date_to_days` % 7 z odpowiednim offsetem, bo `date_to_days` liczy od epoch)
- `add_days(date_str: &str, n: i32) -> String` — dodaje/odejmuje N dni
- `month_grid_range(year: i32, month: u32) -> (String, String)` — zwraca (first_monday, last_sunday) dla siatki miesiąca
- `week_range(date_str: &str) -> (String, String)` — zwraca (monday, sunday) tygodnia zawierającego datę
- `prev_month(year: i32, month: u32) -> (i32, u32)` / `next_month(...)` — nawigacja miesiąca
- `polish_day_of_week(dow: u32) -> &'static str` — "Pn", "Wt", "Śr", ...
- `polish_month_name(month: u32) -> &'static str` — "Styczeń", "Luty", ...

Wszystkie oparte na istniejących `days_in_month` i `date_to_days`. Nie potrzeba `chrono` — pure integer arithmetic z `js_sys::Date` tylko do `get_today_string`.

### 3. Rozbicie `list.rs` (525 linii)

- `render_date_view` → `pages/list/date_view.rs`
- `render_normal_view` + `NormalViewProps` → `pages/list/normal_view.rs`
- `make_item_tag_toggle`, `make_list_tag_toggle`, `make_move_callback` → zostają w `pages/list/mod.rs` (używane przez oba widoki)

### 4. Extract FilterChips z TodayPage

Logika filter chips (hidden_lists/hidden_tags toggle, show_completed, chip rendering) → `components/filters/filter_chips.rs`. TodayPage, CalendarPage i CalendarDayPage reużywają.

---

## Nawigacja (Nav)

Dodanie linku "Kalendarz" w `components/nav.rs` obok istniejących linków.

---

## Weryfikacja

1. `just check` — kompilacja workspace
2. `just lint` — clippy + fmt
3. `just dev` — lokalne testowanie:
   - Sprawdzić `/calendar` — widok miesiąca renderuje siatkę z kropkami
   - Przełączyć na tydzień — itemy widoczne per dzień
   - Kliknąć dzień → `/calendar/:date` z itemami i nawigacją prev/next
   - Prev/next miesiąc i tydzień działają
   - Filtry lista/tag działają na kalendarzu
   - Istniejące strony (Today, List, Tags) nadal działają po reorganizacji
4. Ręczne testy edge case: pusty miesiąc, miesiąc z 1 itemem, przełom miesiąca
