# Kartoteka — Roadmapa projektu (v2)

## Context

Kartoteka to osobista aplikacja todo/listy — miks Todoist, Nozbe, Amazing Marvin, listy zakupów i checklist pakowania. Architektura: Rust API na CF Workers + Leptos CSR frontend + Hanko auth.

Cel roadmapy: zbudować elastyczne, konfigurowalne narzędzie pod siebie, feature-first z refaktoryzacją wplecioną w każdy milestone.

### Co już jest (po roadmapie v1)

- **Tagi** — pełne CRUD, hierarchia, kategorie, kolory, przypisywanie do items/list, strona tagów, tag detail, badge, selector
- **Feature slices v1** — sublists (parent_list_id), archiwum, has_quantity/has_due_date per lista, quantity/unit/due_date/due_time na itemach, reset listy, move item
- **ListType** — Checklist, Zakupy, Pakowanie, Terminarz, Custom
- **Terminarz** — due_date + due_time, date_item_row, grupowanie overdue/upcoming/done
- **UI** — toast system, confirm delete modal, sublist section, list header, add_group_input, item_actions

### Znane problemy w kodzie

- `list.rs` — 480 linii, za dużo logiki w jednym komponencie
- `api.rs` — 34 funkcje w jednym pliku, brak modularyzacji
- Feature slices jako flat booleany (`has_quantity`, `has_due_date`) — nie skaluje się
- Brak cross-list query (każdy widok operuje na jednej liście)
- Brak opisu na listach (items mają, listy nie)
- Daty jako plain stringi bez spójnego modelu

---

## Kluczowe decyzje architektoniczne

### Container (folder/projekt)

Ujednolicony typ `Container` — folder i projekt to ten sam byt, rozróżniany przez `status`:

- `status = NULL` → folder (czysta organizacja wizualna)
- `status = 'active'/'done'/'paused'` → projekt (logiczna jednostka pracy)

Projekt to kontener który łączy różne typy list (Terminarz, Zakupy, Checklist) w jedną logiczną całość. Może żyć w folderze, mieć własne podfoldery, mieć opis i daty.

### Feature slices v2

Z flat booleans na tabelę `list_features(list_id, feature_name, config TEXT/JSON)`. D1 wspiera JSON query (`->>`, `json_extract`). Każdy ficzer to obiekt z konfiguracją, np. time tracking: `{"mode": "pomodoro", "default_duration": 25}`.

---

## ~~M1: Cross-list query + widok "Dziś"~~ ✅

**Dostarcza:** Dashboard "co mam dziś" — pierwszy widok cross-list w apce.

- ✅ Backend: `GET /api/items/by-date?date=YYYY-MM-DD` z opcjonalnymi filtrami list/tag. Queryuje `due_date` (jedyne pole daty w obecnym schemacie). M3 rozszerzy ten endpoint o nowe typy dat.
- ✅ Frontend: strona `/today`, itemy pogrupowane per lista źródłowa, toggle widoczności per lista/tag
- ✅ Migracja: dodanie `description TEXT` do tabeli `lists` (rozwiązuje znany problem braku opisu na listach)

**Refaktor:** ✅ Wydzielenie query helpera w API (reużywalny across endpoints). ✅ Wydzielenie `api.rs` frontendu na moduły (`api/lists.rs`, `api/items.rs`, `api/tags.rs`). ✅ Rozbicie `list.rs` na mniejsze komponenty (`EditableDescription`, `ListTagBar`, `TagFilterBar`).

---

## M2: Widok kalendarza (miesiąc + tydzień)

**Dostarcza:** Nawigacja po dniach z lotu ptaka.

- Backend: `GET /api/items/calendar?from=YYYY-MM-DD&to=YYYY-MM-DD` — per-day counts + item summaries
- Frontend: widok miesiąca (kratki z kropkami/liczbą, klik → widok dnia via M1 `/api/items/by-date`), widok tygodniowy, przełącznik
- Filtrowanie per lista/tag (reużywa toggle z M1)

**Refaktor:** Wydzielenie komponentów date-related z `list.rs` do osobnych plików. Rozbicie `list.rs` (480 linii) na mniejsze komponenty.

---

## M3: Konfigurowalny feature slice system

**Dostarcza:** Feature jako obiekt z konfiguracją zamiast flat booleans. Fundament pod wszystkie przyszłe ficzery.

- Backend: tabela `list_features(list_id, feature_name, config TEXT/JSON)`
- Migracja istniejących flag:
  - `has_quantity` → feature `"quantity"`, config `{"unit_default": "szt"}`
  - `has_due_date` → feature `"due_date"`, config `{}`
- Endpointy: CRUD features per lista
- Frontend: UI konfiguracji features przy tworzeniu/edycji listy (toggle + config per ficzer), dynamiczny item form renderuje pola na podstawie aktywnych features

**Refaktor:** Usunięcie flat booleans (`has_quantity`, `has_due_date`) z `List` struct i kolumn w DB. Dynamiczny UI zamiast hardcoded warunków w komponentach. Wydzielenie feature-specific komponentów (quantity input, date picker) jako pluggable modules.

---

## M4: Bogatsza semantyka czasu

**Dostarcza:** Różne typy dat — start_date, deadline, hard_deadline. Buduje na feature slice system z M3.

- Backend: nowe kolumny na items (`start_date`, `deadline`, `hard_deadline`)
- Nowy feature slice `"deadlines"` z config `{"has_start_date": true, "has_deadline": true, "has_hard_deadline": false}` — włączany per lista przez M3 system
- Cross-list query (M1) rozszerzony o nowe typy dat (`?date_field=deadline`)
- Frontend: dynamiczne pola w formularzu itema (zależne od config feature slice), wizualne rozróżnienie per typ daty (kolor/ikona)
- Kalendarz (M2) i "Dziś" (M1) rozumieją nowe typy dat

**Refaktor:** Ujednolicenie obsługi dat w shared (due_date + due_time jako raw stringi → spójny model dat).

---

## M5: Containers (foldery + projekty)

**Dostarcza:** Foldery i projekty jako unified Container.

- Backend: tabela `containers`, `container_id TEXT` na listach (nullable FK)
- Progress na projektach: computed server-side — `GET /api/containers/:id` zwraca `completed_items / total_items` across all list w kontenerze (nie stored, obliczany przy każdym request)
- Endpointy: CRUD containers, przenoszenie list między kontenerami
- Frontend: home page jako drzewiasta struktura, tworzenie folderów/projektów, status projektu z progress barem, breadcrumbs nawigacja

**Refaktor:** Home page z flat → tree. Wydzielenie nawigacji/breadcrumbs.

---

## M6: Szablony checklist

**Dostarcza:** Reużywalne checklisty — "kosmetyczka", "elektronika" jako podzbiory listy pakowania.

- Backend: `templates` + `template_items`, CRUD, tworzenie listy z wielu szablonów (merge)
- Frontend: "Zapisz jako szablon", wybór szablonów przy tworzeniu listy
- Szablon = snapshot — edycja szablonu nie zmienia istniejących list

**Refaktor:** Wydzielenie logiki tworzenia list/itemów do shared service layer.

---

## M7: Ikony

**Dostarcza:** SVG ikony (Lucide) na kontenerach, listach, tagach, itemach.

- Backend: `icon TEXT` na tabelach
- Frontend: icon picker, przycisk "random"

**Refaktor:** Ujednolicenie wizualne — wspólny komponent `IconLabel` dla elementów z ikoną + kolorem + nazwą.

---

## M8: Redesign "Neonowa Noc"

**Dostarcza:** Pełny redesign UI z neonową estetyką.

- Paleta: tło (#0a0a1a, #12122a, #1a1a3e), neon cyan (#00f5ff), magenta (#ff00ff), fiolet (#8b5cf6), zielony (#39ff14)
- CSS custom properties (design tokens), neon glow, typografia mono/geometric
- Karty z border-glow, animacje hover, neonowe checkbox/toggle, gradient buttons

**Refaktor:** Design tokens zamiast hardcoded DaisyUI classes. Ujednolicenie komponentów pod nowy system.

---

## M9: PWA / Offline

**Dostarcza:** Działanie bez internetu.

- Service worker, IndexedDB, sync queue, conflict resolution (last-write-wins), PWA ikony

**Refaktor:** Wydzielenie warstwy data access w frontend (api:: calls → abstrakcja API/local cache).

---

## Backlog

- [ ] Time tracking — pomodoro vs stoper jako feature slice (wymaga M3). Storage: `time_entries(id, item_id, started_at, ended_at, mode)`. UI: timer per item, raporty czasu. Odłożony bo wymaga osobnego designu UX.
- [ ] Recurrence (powtarzalność itemów) — odłożone do ustabilizowania modelu dat (po M4)
- [ ] Widok dnia v2 (timeline godzinowy z blokami czasowymi)
- [ ] Czas trwania na elementach terminarza (estimated duration vs time tracking = actual time spent)
- [ ] CORS — restrict do właściwej domeny
- [ ] Error handling — retry logic, lepsze komunikaty
- [ ] Loading states — skeleton loaders
- [ ] Testy — unit + integration
- [ ] Drag & drop reordering — pozycja (`position`) istnieje w DB, brak UI
- [ ] Kanban (opcjonalnie na projektach)

## Weryfikacja (per milestone)

1. `just check` — kompilacja workspace
2. `just lint` — clippy + fmt
3. `just dev` — lokalne testowanie
4. Ręczne testy w UI
5. `just deploy` — deploy + smoke test
