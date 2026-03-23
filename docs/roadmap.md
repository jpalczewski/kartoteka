# Kartoteka — Roadmapa projektu

## Context

Kartoteka to osobista aplikacja todo/listy — miks Todoist, Nozbe, Amazing Marvin, listy zakupów i checklist pakowania. Obecny stan to działające MVP (Rust API na CF Workers + Leptos CSR frontend + Hanko auth) z podstawowym CRUD list i itemów. Brakuje: tagów, typów list w UI, szablonów checklist, terminów, redesignu UI.

Cel roadmapy: zbudować elastyczne, konfigurowalne narzędzie pod siebie, feature-first (najpierw wartość, potem estetyka).

---

## Faza 1: System tagów + rozbudowa typów list

**Cel**: Fundament pod filtrowanie i organizację — tagi hierarchiczne z kolorami + pełne wsparcie typów list w UI.

### Backend
- Nowa tabela `tags` (id TEXT PK, name, color TEXT, category TEXT, parent_tag_id TEXT nullable FK, user_id TEXT, created_at)
- Tabele łączące: `item_tags` (item_id, tag_id), `list_tags` (list_id, tag_id)
- Migracja `0002_tags.sql`
- Endpointy: CRUD tagów, przypisywanie/usuwanie tagów z items/list, filtrowanie items/list po tagach
- Kategorie tagów: kontekst (dom/praca/siłownia), priorytet (wysoki/średni/niski), custom

### Frontend
- Selektor tagów z kolorami przy tworzeniu/edycji itema i listy
- Widok filtrowany po tagach (sidebar lub top bar)
- Zarządzanie tagami w ustawieniach (CRUD, przypisywanie kolorów, hierarchia)
- Wybór `ListType` przy tworzeniu listy (ikony + labele zamiast raw debug format)

---

## Faza 2: Reużywalne checklisty (szablony)

**Cel**: System szablonów checklist — "kosmetyczka", "elektronika", "ubrania" jako podzbiory listy pakowania.

### Backend
- Tabela `templates` (id, name, list_type, user_id, created_at)
- Tabela `template_items` (id, template_id, title, position)
- Endpointy: CRUD szablonów, tworzenie listy z szablonów
- Endpoint "reset checklist" — odznacz wszystkie itemy w liście

### Frontend
- Tworzenie szablonu z istniejącej listy ("Zapisz jako szablon")
- Przy tworzeniu listy pakowania — wybór szablonów do dołączenia (multi-select)
- Przycisk "Reset wszystko" na liście typu Packing
- Zarządzanie szablonami (edycja, usuwanie)

### Logika
- Szablon = snapshot itemów — edycja szablonu nie zmienia istniejących list
- Tworzenie listy z wielu szablonów = merge itemów z zachowaniem pozycji

---

## Faza 3: Todo z terminami i powtarzalnością

**Cel**: Rozszerzenie itemów o daty, priorytety, powtarzalność — zamienia prostą checklistę w pełny todo.

### Backend
- Migracja ALTER `items`: dodanie `due_date` (TEXT nullable), `priority` (INTEGER 0-3), `recurrence` (TEXT nullable), `completed_at` (TEXT nullable)
- Logika powtarzalności: po completion itema z recurrence — tworzenie nowego z przesuniętą datą
- Format recurrence: "daily", "weekly:mon,fri", "monthly:15", "yearly:03-23"
- Endpoint widoku "dziś" / "nadchodzące" (filtr po due_date)

### Frontend
- Datepicker (due_date)
- Wskaźnik priorytetu (kolorowy badge/ikona)
- Widok "Dziś" — wszystkie itemy z due_date = today (cross-list)
- Widok "Nadchodzące" — kalendarzowy przegląd
- Sortowanie po dacie / priorytecie
- Ustawianie powtarzalności

---

## Faza 4: Redesign "Neonowa Noc"

**Cel**: Pełny redesign UI z estetyką neonowej nocy — ciemne tło, neonowe akcenty, glow effects.

### Design System
- Paleta: tło (#0a0a1a, #12122a, #1a1a3e), neon cyan (#00f5ff), neon magenta (#ff00ff), neon fiolet (#8b5cf6), neon zielony (#39ff14)
- CSS custom properties (design tokens) dla kolorów, spacing, border-radius, shadows
- Neon glow: `box-shadow` i `text-shadow` z neonowymi kolorami
- Typografia: monospace lub geometric sans-serif dla nagłówków

### Komponenty
- Karty list z neonowym border-glow (kolor zależny od typu listy)
- Animacje hover (glow intensyfikacja, subtle transform)
- Neonowe checkbox/toggle
- Gradient accent na przyciskach
- Nav bar z neonowym underline na aktywnej stronie
- Tag badges z neonowym kolorem

---

## Faza 5: Projekty i sub-taski

**Cel**: Grupowanie zadań w projekty, hierarchia zadań z progress tracking.

### Backend
- Dodanie `parent_item_id` (TEXT nullable FK) do `items` — sub-taski
- Progress na poziomie projektu (obliczany: completed sub-items / total)
- Opcjonalnie: widok Kanban (kolumny statusów)

### Frontend
- Rozwinięcie/zwinięcie sub-tasków pod parent itemem
- Pasek postępu na liście typu Project
- Indent wizualny sub-tasków
- Drag & drop do reorganizacji (opcjonalnie)

---

## Faza 6: PWA / Offline (opcjonalnie, średni priorytet)

**Cel**: Działanie bez internetu — service worker, cache, sync.

### Zakres
- Service worker z cache-first dla statycznych assetów
- IndexedDB jako lokalny store
- Kolejka operacji offline → sync po powrocie online
- Conflict resolution (last-write-wins lub merge)
- PWA ikony (brak w obecnym manifest.json)

---

## Weryfikacja (per faza)

1. `just check` — kompilacja workspace
2. `just lint` — clippy + fmt
3. `just dev` — lokalne testowanie (API + frontend)
4. Ręczne testy: tworzenie/edycja/usuwanie w UI
5. `just deploy` — deploy na CF i smoke test na produkcji

---

## Backlog (do wplecenia w fazy)

- [ ] CORS — restrict do właściwej domeny (obecne TODO w router.rs)
- [ ] Error handling — retry logic, lepsze komunikaty błędów w UI
- [ ] Loading states — skeleton loaders
- [ ] Testy — unit + integration (Rust + WASM)
- [ ] Item description — wyświetlanie/edycja w UI (pole istnieje w DB)
- [ ] Item reordering — drag & drop (pole position istnieje w DB)
