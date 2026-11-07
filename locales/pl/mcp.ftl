# Strona zgody OAuth
oauth-consent-title = Autoryzuj dostęp
oauth-consent-client-requests = Aplikacja { $client } prosi o dostęp do Twojego konta Kartoteka.
oauth-consent-scope-label = Wymagane uprawnienia:
oauth-consent-warning = Zatwierdź tylko jeśli ufasz tej aplikacji. Możesz cofnąć dostęp później w Ustawieniach.
oauth-consent-approve = Zatwierdź
oauth-consent-deny = Odrzuć
oauth-consent-scope-mcp = Odczyt i modyfikacja Twoich list, elementów, tagów, komentarzy i śledzenia czasu.

# Opisy nowych narzędzi MCP
mcp-tool-create_item-desc = Utwórz nowy element w liście. recurrence_rule powtarza element: Nd, Nw lub Nm (np. 7d, 2w, 1m), a następne wystąpienie powstaje po ukończeniu elementu. recurrence_mode to fixed (od zaplanowanej daty, domyślnie) lub floating (od daty ukończenia). Reguła wymaga terminu lub daty rozpoczęcia.
mcp-tool-update_item-desc = Zaktualizuj pola istniejącego elementu. Użyj tablicy "clear" aby wyzerować pola (np. clear: ["deadline", "icon"]); wyzerowanie recurrence_rule zeruje też recurrence_mode. recurrence_rule powtarza element: Nd, Nw lub Nm (np. 7d, 2w, 1m), recurrence_mode to fixed (od zaplanowanej daty, domyślnie) lub floating (od daty ukończenia). Ukończenie powtarzalnego elementu zwraca jego następne wystąpienie jako "spawned".
mcp-tool-toggle_item-desc = Przełącz stan ukończenia elementu. Ukończenie powtarzalnego elementu tworzy jego następne wystąpienie, zwrócone jako "spawned". Lista musi mieć włączoną funkcję checklist, a ukończenie kończy się błędem, gdy element ma nierozwiązane blokady.
mcp-tool-set_item_status-desc = Ustaw status elementu: "todo", "in_progress" lub "done". "done" to to samo co ukończenie elementu (blokują je nierozwiązane blokery), pozostałe dwa otwierają go ponownie.
mcp-tool-set_container_status-desc = Przełącz kontener między folderem (null) a projektem ("active", "done", "paused"). Folder z podkontenerami nie może stać się projektem.
mcp-tool-search_items-desc = Pełnotekstowe wyszukiwanie w elementach i komentarzach.
mcp-tool-add_comment-desc = Dodaj komentarz do elementu, listy lub kontenera. Pomiń author_name gdy piszesz w imieniu użytkownika (jego głos). Ustaw na swoje imię (np. "Claude") gdy komentarz to Twoja własna obserwacja, sugestia lub analiza.
mcp-tool-add_relation-desc = Utwórz relację blocks lub relates_to między dwoma elementami.
mcp-tool-remove_relation-desc = Usuń istniejącą relację między dwoma elementami.
mcp-tool-get_relations-desc = Pobierz wszystkie relacje encji (dwukierunkowo), każda z tytułem drugiej strony, a dla elementów z informacją czy jest ukończona. entity_type: "item", "list" lub "container".
mcp-tool-start_timer-desc = Rozpocznij pomiar czasu dla elementu (automatycznie zatrzymuje bieżący timer). mode: "stopwatch" (domyślnie) lub "pomodoro".
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
mcp-err-already-exists = { $kind } o tej nazwie już tu istnieje (id: { $id }). Użyj istniejącego albo wybierz inną nazwę.
mcp-err-not-empty = { $kind } nadal zawiera pozycje (liczba: { $items }), więc nie można go usunąć.
mcp-err-has-children = { $kind } nadal ma listy ({ $lists }) i kontenery ({ $containers }), więc nie można go usunąć.
mcp-err-forbidden = Dostęp zabroniony.
mcp-err-internal = Błąd wewnętrzny.
mcp-err-bad-uri = Nieprawidłowy URI zasobu: { $uri }.
mcp-tool-list_lists-desc = Pobierz wszystkie listy.
mcp-tool-get_list-desc = Pobierz szczegóły konkretnej listy po ID.
mcp-tool-list_items-desc = Pobierz elementy listy z opcjonalną paginacją kursorową.
mcp-tool-list_container_items-desc = Pobierz wszystkie elementy z kontenera (folderu/projektu) ze wszystkich jego list z paginacją kursorową. Używaj tego narzędzia, gdy chcesz przejrzeć całą zawartość kontenera jednym wywołaniem zamiast pobierać każdą listę osobno. Ustaw recursive=true, aby uwzględnić również elementy z zagnieżdżonych podkontenerów (przeszukiwanie depth-first).
mcp-tool-list_containers-desc = Pobierz wszystkie kontenery.
mcp-tool-get_container-desc = Pobierz szczegóły konkretnego kontenera po ID.
mcp-tool-list_tags-desc = Pobierz wszystkie tagi.
mcp-tool-create_tag-desc = Utwórz nowy tag z nazwą, opcjonalnym kolorem i opcjonalnym rodzicem.
mcp-tool-create_tags-desc = Utwórz wiele tagów atomowo. Użyj parent_tag_ref, aby zagnieździć tag pod tagiem utworzonym wcześniej w tej samej batchy (ustaw client_ref na rodzicu). Referencje tylko do przodu.
mcp-tool-get_tag_entities-desc = Pobierz elementy i listy powiązane z tagiem. entity_type: "item" lub "list" do filtrowania; pomiń, aby pobrać oba.
mcp-tool-assign_tag-desc = Przypisz tag do encji. entity_type: "item", "list" lub "container".
mcp-tool-unassign_tag-desc = Usuń tag z encji. entity_type: "item", "list" lub "container".
mcp-tool-get_today-desc = Pobierz elementy na dziś.
mcp-tool-get_time_summary-desc = Pobierz śledzony czas. Z entity_type ("item", "list" lub "container") i entity_id zwraca sumę sekund oraz wartość h:mm, zsumowaną po podlistach i podkontenerach i z uwzględnieniem działającego timera; bez argumentów zwraca ostatnie wpisy czasu.
mcp-tool-create_list-desc = Utwórz nową listę. list_type: checklist (domyślny), shopping, schedule, log lub notes.
mcp-tool-get_item-desc = Pobierz szczegóły konkretnego elementu po ID.
mcp-tool-list_templates-desc = Pobierz wszystkie zapisane szablony list.
mcp-tool-list_overdue-desc = Pobierz wszystkie elementy po terminie, które nie są jeszcze ukończone.
mcp-tool-get_active_timer-desc = Pobierz aktualnie działający timer, jeśli istnieje.
mcp-tool-create_container-desc = Utwórz pojedynczy folder (bez status) lub projekt (status: "active", "done" lub "paused"). Aby zbudować przestrzeń roboczą, wywołaj create_containers (batch) + create_lists + create_items sekwencyjnie.
mcp-tool-create_items-desc = Dodaj wiele elementów do jednej listy atomowo. Podaj list_id raz na poziomie głównym; wszystkie elementy trafiają do tej listy w podanej kolejności.
mcp-tool-create_lists-desc = Utwórz wiele list atomowo. Użyj container_ref aby odwołać się do kontenera z poprzedniego wywołania create_containers (ustaw na nim client_ref). Użyj parent_list_ref aby zagnieździć pod listą z tego samego batcha. Referencje tylko do wcześniejszych elementów.
mcp-tool-create_containers-desc = Utwórz wiele folderów/projektów atomowo. Użyj parent_container_ref aby zagnieździć pod kontenerem z wcześniej w tym samym batchu (ustaw client_ref na rodzicu). Tylko referencje do wcześniejszych elementów w tablicy.

mcp-tool-move_list_to_container-desc = Przenieś listę do kontenera lub odepnij ją od bieżącego (container_id: null).
mcp-tool-move_container_to_parent-desc = Wnieś kontener do folderu-rodzica lub odepnij do poziomu głównego (parent_container_id: null). Tylko foldery (bez status) mogą być rodzicem.
mcp-tool-toggle_mark_for_delete-desc = Oznacz encję do usunięcia lub odznacz ją. entity_type: "item", "list", "container" lub "comment". Nic nie zostaje usunięte: encja trafia do kolejki usuwania użytkownika, którą użytkownik przegląda w aplikacji na /trash i tam decyduje. Ponowne wywołanie na oznaczonej encji ją odznacza. To jedyny sposób usuwania przez MCP.
mcp-tool-list_marked_for_delete-desc = Pobierz wszystko, co czeka w kolejce usuwania użytkownika (typ encji, etykieta, gdzie się znajduje, a dla list i kontenerów także co jeszcze zniknie razem z nimi). Nic nie zostaje usunięte. Wywołaj to przed przełączeniem, aby nie odznaczyć czegoś, co już oznaczyłeś.
