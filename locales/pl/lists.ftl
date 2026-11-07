# Typy list (presety)
lists-type-checklist = Checklist
lists-type-shopping = Zakupy
lists-type-schedule = Terminarz
lists-type-log = Dziennik
lists-type-notes = Notatki

# Statusy kontenerów
lists-container-status-active = aktywny
lists-container-status-done = ukończony
lists-container-status-paused = wstrzymany

# Zakładki trybu tworzenia
lists-mode-list = Lista
lists-mode-folder = Folder
lists-mode-project = Projekt

# Placeholdery
lists-new-list-placeholder = Nazwa nowej listy...
lists-new-folder-placeholder = Nazwa nowego folderu...
lists-new-project-placeholder = Nazwa nowego projektu...

# Przełączniki funkcji
lists-feature-checklist = Zadania
lists-feature-quantities = Ilości
lists-feature-deadlines = Terminy
lists-feature-time-tracking = Śledzenie czasu
lists-feature-location = Lokalizacja

# Przyciski nagłówka listy
lists-header-settings-button = Ustawienia (koło zębate)
lists-header-reset-button = Reset
lists-header-archive-button = Archiwizuj
lists-header-delete-button = Usuń listę

# Panel ustawień funkcji
lists-features-label = Funkcje:
lists-deadlines-dates-label = Pola dat:
lists-deadlines-start = Start
lists-deadlines-deadline = Termin
lists-deadlines-hard = Twardy termin

# Wyświetlanie postępu
lists-progress = { $done }/{ $total } ✓

# Dodaj grupę
lists-add-group-placeholder = Nazwa grupy...
lists-add-group-button = + Dodaj grupę

# Etykiety aria
lists-delete-list-aria = Usuń listę
lists-delete-container-aria = Usuń kontener
lists-pin-container-aria = Przypnij

# Pasek postępu kontenera
lists-tasks-progress = { $completed }/{ $total } zadań

# Modal potwierdzenia usunięcia (listy i kontenery)
lists-confirm-delete-title = Usuń listę
lists-confirm-delete-container-title = Usuń kontener
lists-confirm-delete-loading = Wczytywanie szczegółów…
lists-confirm-delete-message = Czy na pewno chcesz usunąć listę „{ $name }"? Operacja jest nieodwracalna.
lists-confirm-delete-container-message = Czy na pewno chcesz usunąć kontener „{ $name }"? Operacja jest nieodwracalna.
lists-confirm-delete-button = Usuń listę
lists-confirm-delete-container-button = Usuń kontener
lists-confirm-delete-blocked = Lista „{ $name }" wciąż zawiera { $count ->
        [one] { $count } element
        [few] { $count } elementy
        *[other] { $count } elementów
    } — usuń je albo zamiast tego zarchiwizuj listę.
lists-confirm-delete-container-blocked = Kontener „{ $name }" wciąż zawiera { $count ->
        [one] { $count } element
        [few] { $count } elementy
        *[other] { $count } elementów
    } — usuń je albo zamiast tego zarchiwizuj kontener.
lists-confirm-delete-descendant-lists = To usunie także { $count ->
        [one] { $count } pustą podlistę
        [few] { $count } puste podlisty
        *[other] { $count } pustych podlist
    }.
lists-confirm-delete-descendant-containers = To usunie także { $count ->
        [one] { $count } pusty podkontener
        [few] { $count } puste podkontenery
        *[other] { $count } pustych podkontenerów
    }.
lists-confirm-delete-descendant-both = To usunie także { $lists ->
        [one] { $lists } pustą podlistę
        [few] { $lists } puste podlisty
        *[other] { $lists } pustych podlist
    } i { $containers ->
        [one] { $containers } pusty podkontener
        [few] { $containers } puste podkontenery
        *[other] { $containers } pustych podkontenerów
    }.
lists-confirm-delete-preview-error = Nie udało się wczytać szczegółów usunięcia. { $detail }

# Drag and drop
lists-dnd-drop-here = Upuść tutaj
lists-dnd-drop-at-end = Upuść na końcu
lists-dnd-reorder-aria = Przeciągnij, aby zmienić kolejność
lists-dnd-reorder-group-aria = Przeciągnij, aby zmienić kolejność grupy

# Strona kontenera i list
lists-container-status-option-active = Aktywny
lists-container-status-option-done = Ukończony
lists-container-status-option-paused = Wstrzymany
lists-completed-lists-progress = Listy ukończone: { $completed }/{ $total }
lists-containers-heading = Kontenery
lists-lists-heading = Listy
lists-container-empty = Brak list w tym kontenerze.
lists-empty = Lista jest pusta
lists-inline-error = Błąd: { $detail }
lists-main-move-target = { $name } (główna)

lists-expand-all = Rozwiń wszystkie
lists-collapse-all = Zwiń wszystkie

# Podgląd listy
lists-preview-open-full = Otwórz pełny widok
lists-preview-new-item-placeholder = Nowy element...

# UI przenoszenia listy / kontenera do rodzica
lists-move-to-container = Przenieś do kontenera
lists-detach-from-container = Odepnij od kontenera
lists-move-to-parent = Wnieś do kontenera
lists-detach-from-parent = Odepnij od rodzica


# Toasty
lists-toast-container-deleted = Kontener usunięty
lists-toast-list-deleted = Lista usunięta
lists-toast-list-archived = Lista zarchiwizowana
lists-toast-list-reset = Lista zresetowana

# Akcje kart list i kontenerów
lists-pin = Przypnij
lists-unpin = Odepnij
lists-archive-aria = Archiwizuj
lists-restore-aria = Przywróć
lists-archive-container-aria = Archiwizuj kontener
lists-reset-completed = Resetuj ukończone
