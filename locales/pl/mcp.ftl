# Listy
tool-list-lists = Wyświetl wszystkie listy użytkownika
tool-create-list = Utwórz nową listę, opcjonalnie w kontenerze albo jako podlistę
tool-update-list = Zaktualizuj nazwę, opis, typ lub status archiwizacji listy
tool-move-list = Przenieś listę do kontenera lub usuń z kontenera
tool-get-list-sublists = Pobierz bezpośrednie podlisty listy
tool-set-list-placement = Przenieś jedną lub wiele list między root, kontenerami i listami-rodzicami

# Elementy
tool-get-items = Pobierz elementy z konkretnej listy, opcjonalnie filtrując po ukończeniu, obecności deadline albo zakresie dat
tool-search-items = Wyszukaj elementy globalnie po tytule, opisie i tagach
tool-search-entities = Wyszukaj globalnie elementy, listy i kontenery po nazwie lub opisie
tool-next-cursor-page = Pobierz następną stronę dla wcześniej zwróconego kursora paginacji
tool-add-item = Dodaj jeden lub wiele elementów do listy
tool-update-item = Zaktualizuj istniejący element
tool-toggle-item = Ustaw stan ukończenia jednego lub wielu elementów
tool-move-item = Przenieś jeden lub wiele elementów do innej listy

# Kontenery
tool-list-containers = Wyświetl wszystkie kontenery użytkownika
tool-create-container = Utwórz nowy kontener (folder lub projekt)
tool-get-container = Pobierz kontener z metrykami postępu
tool-get-container-children = Pobierz podkontenery i listy wewnątrz kontenera
tool-get-home = Pobierz panel główny: przypięte, ostatnie, kontenery i listy

# Tagi
tool-list-tags = Wyświetl wszystkie tagi użytkownika
tool-create-tag = Utwórz nowy tag
tool-assign-tag = Przypisz tag do elementu lub listy
tool-remove-tag = Usuń tag z elementu lub listy
tool-set-tag-links = Przypisz lub usuń wiele powiązań tagów dla elementów albo list jednym wywołaniem
tool-get-tagged-items = Pobierz wszystkie elementy z konkretnym tagiem
tool-get-tag-entities = Pobierz elementy i listy powiązane z tagiem, opcjonalnie filtrując po typie encji

# Kalendarz
tool-get-calendar = Pobierz elementy z datami w zakresie dat
tool-get-today = Pobierz wszystkie elementy na dziś, w tym zaległe

# Funkcje list
tool-enable-list-feature = Włącz funkcję na liście. Dla 'deadlines' opcjonalnie skonfiguruj dostępne pola dat. Dla 'quantity' opcjonalnie ustaw domyślną jednostkę. Wywołaj tylko po potwierdzeniu przez użytkownika (chyba że ustawienie mcp_auto_enable_features jest włączone).
tool-disable-list-feature = Wyłącz funkcję na liście. Dane elementów (ilości, daty) są zachowane — ukryte w UI, ale nie usunięte.

# Strona zgody OAuth
oauth-consent-title = Autoryzuj dostęp
oauth-consent-client-requests = Aplikacja { $client } prosi o dostęp do Twojego konta Kartoteka.
oauth-consent-scope-label = Wymagane uprawnienia:
oauth-consent-warning = Zatwierdź tylko jeśli ufasz tej aplikacji. Możesz cofnąć dostęp później w Ustawieniach.
oauth-consent-approve = Zatwierdź
oauth-consent-deny = Odrzuć
oauth-consent-scope-mcp = Odczyt i modyfikacja Twoich list, elementów, tagów, komentarzy i śledzenia czasu.

# Opisy nowych narzędzi MCP
mcp-tool-create_item-desc = Utwórz nowy element w liście.
mcp-tool-update_item-desc = Zaktualizuj pola istniejącego elementu. Użyj tablicy "clear" aby wyzerować pola (np. clear: ["deadline", "description"]).
mcp-tool-search_items-desc = Pełnotekstowe wyszukiwanie w elementach i komentarzach.
mcp-tool-add_comment-desc = Dodaj komentarz do elementu, listy lub kontenera. Pomiń author_name gdy piszesz w imieniu użytkownika (jego głos). Ustaw na swoje imię (np. "Claude") gdy komentarz to Twoja własna obserwacja, sugestia lub analiza.
mcp-tool-add_relation-desc = Utwórz relację blocks lub relates_to między dwoma elementami.
mcp-tool-remove_relation-desc = Usuń istniejącą relację między dwoma elementami.
mcp-tool-start_timer-desc = Rozpocznij pomiar czasu dla elementu (automatycznie zatrzymuje bieżący timer).
mcp-tool-stop_timer-desc = Zatrzymaj bieżący timer.
mcp-tool-log_time-desc = Zaloguj retrospektywny wpis czasu z czasem początkowym i długością.
mcp-tool-create_list_from_template-desc = Utwórz nową listę z jednego lub więcej szablonów.
mcp-tool-save_as_template-desc = Zapisz istniejącą listę jako szablon wielokrotnego użytku.

# Opisy zasobów MCP
mcp-res-lists-desc = Minimalne projekcje list dla odkrywania (id, nazwa, kontener, przypięcie, archiwizacja, liczba elementów).
mcp-res-containers-desc = Minimalne projekcje kontenerów (id, nazwa, parent_id, status, przypięcie).
mcp-res-tags-desc = Projekcje tagów z paginacją (id, nazwa, tag_type, parent_id).
mcp-res-today-desc = Elementy z dzisiejszą datą, w strefie czasowej użytkownika.
mcp-res-time-summary-desc = Zagregowane śledzenie czasu: dziś, tydzień, top-10 na listę.
mcp-res-list-detail-desc = Pełen obiekt listy z cechami i liczbą elementów.
mcp-res-list-items-desc = Elementy z listy, paginowane za pomocą nieprzezroczystych kursorów.
mcp-res-container-detail-desc = Kontener z projekcjami dzieci.

# Komunikaty błędów MCP
mcp-err-unauthorized = Brak autoryzacji: token bearer nieobecny lub nieprawidłowy.
mcp-err-not-found = Nie znaleziono: { $entity }.
mcp-err-validation = Walidacja nie powiodła się: { $reason }.
mcp-err-feature-required = Lista nie ma cechy: { $feature }.
mcp-err-forbidden = Dostęp zabroniony.
mcp-err-internal = Błąd wewnętrzny.
mcp-err-bad-uri = Nieprawidłowy URI zasobu: { $uri }.
mcp-tool-list_lists-desc = Pobierz wszystkie listy.
mcp-tool-get_list-desc = Pobierz szczegóły konkretnej listy po ID.
mcp-tool-list_items-desc = Pobierz elementy listy z opcjonalną paginacją kursorową.
mcp-tool-list_containers-desc = Pobierz wszystkie kontenery.
mcp-tool-get_container-desc = Pobierz szczegóły konkretnego kontenera po ID.
mcp-tool-list_tags-desc = Pobierz wszystkie tagi.
mcp-tool-get_today-desc = Pobierz elementy na dziś.
mcp-tool-get_time_summary-desc = Pobierz wszystkie wpisy czasu.
mcp-tool-create_list-desc = Utwórz nową listę. list_type: checklist (domyślny), shopping, habit lub custom.
