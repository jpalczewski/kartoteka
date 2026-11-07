# /trash — the review page for pending deletions (#226)

trash-title = Kosz
trash-subtitle = Elementy oznaczone do usunięcia. Usunięcie jest trwałe i nie można go cofnąć.
trash-loading-error = Nie udało się wczytać kosza: { $detail }
trash-empty = Kosz jest pusty — nic nie czeka na usunięcie.
trash-section-containers = Kontenery
trash-section-lists = Listy
trash-section-items = Elementy
trash-section-comments = Komentarze
trash-select-all = Zaznacz wszystkie
trash-selected = Zaznaczono: { $count }
trash-action-unmark = Przywróć
trash-action-delete = Usuń
trash-action-unmark-selected = Przywróć zaznaczone
trash-action-delete-selected = Usuń zaznaczone
trash-action-delete-all = Opróżnij kosz
trash-marked-just-now = przed chwilą
trash-marked-unknown = —
trash-marked-minutes = { $count ->
        [one] { $count } minutę temu
        [few] { $count } minuty temu
        *[other] { $count } minut temu
    }
trash-marked-hours = { $count ->
        [one] { $count } godzinę temu
        [few] { $count } godziny temu
        *[other] { $count } godzin temu
    }
trash-marked-days = { $count ->
        [one] { $count } dzień temu
        [few] { $count } dni temu
        *[other] { $count } dni temu
    }
trash-cascade-lists = { $count ->
        [one] { $count } podlista
        [few] { $count } podlisty
        *[other] { $count } podlist
    }
trash-cascade-containers = { $count ->
        [one] { $count } podkontener
        [few] { $count } podkontenery
        *[other] { $count } podkontenerów
    }
trash-cascade-items = { $count ->
        [one] { $count } element
        [few] { $count } elementy
        *[other] { $count } elementów
    }
trash-cascade-badge = +{ $details }
trash-confirm-delete-title = Usuń trwale
trash-confirm-delete-message = Czy na pewno trwale usunąć { $count ->
        [one] { $count } pozycję
        [few] { $count } pozycje
        *[other] { $count } pozycji
    } z kosza? Tej operacji nie można cofnąć.
trash-confirm-delete-cascade = Usunięcie zabierze także: { $details }.
trash-confirm-delete-button = Usuń trwale
