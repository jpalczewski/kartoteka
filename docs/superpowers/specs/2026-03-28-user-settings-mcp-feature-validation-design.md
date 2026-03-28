# User Settings + MCP Feature Validation

## Context

Przy dodawaniu itemów przez MCP, `ensureFeatures` cicho włączało features na liście (np. `deadlines`) gdy item zawierał feature-gated pola. To powodowało:
- Niezgodność config (`{}` vs pełny config z domyślnymi wartościami)
- Brak świadomości usera o zmianie konfiguracji listy

Cel: API odrzuca użycie feature-gated pól gdy feature nie jest włączony. MCP surfuje błąd do Claude który pyta usera. Opcjonalnie: user może wyrazić preferencję "auto-włączaj features" przez generyczny system user settings.

## Scope

- Generyczna tabela `user_settings(user_id, key, value)` + CRUD API
- Pierwsza preferencja: `mcp_auto_enable_features` (bool, default false)
- Settings page: toggle dla tej preferencji
- API: walidacja feature-gated fields w `create_item` / `update_item`
- MCP: usunięcie `ensureFeatures`, nowe narzędzie `enable_list_feature`, czytanie preferencji

**Nie w scope:** inne preferencje (wchodzą do tej samej tabeli bez dodatkowych zmian), walidacja po stronie API dla innych klientów niż MCP (na razie).

## Database

### Migracja `0011_user_settings.sql`

```sql
CREATE TABLE user_settings (
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, key)
);
```

`value` to TEXT — JSON dla złożonych wartości, plain string dla prostych. Bez CHECK constraint na `key` — otwarte na przyszłe ustawienia bez migracji.

## Shared Types

```rust
// user settings CRUD
pub struct UserSetting {
    pub key: String,
    pub value: serde_json::Value,
}

pub struct UpsertSettingRequest {
    pub value: serde_json::Value,
}

// known setting keys
pub const SETTING_MCP_AUTO_ENABLE_FEATURES: &str = "mcp_auto_enable_features";
```

## API Endpoints

### Settings CRUD

| Method | Path | Opis |
|--------|------|------|
| GET | `/api/settings` | Zwraca wszystkie ustawienia usera jako `{ key: value }` map |
| PUT | `/api/settings/:key` | Upsert jednego ustawienia. Body: `{ "value": <json> }` |
| DELETE | `/api/settings/:key` | Usuń ustawienie (wraca do default) |

Handler `GET /api/settings` zwraca flat JSON map `{ "key": value, ... }` — wartości jako natywne JSON (bool, string, number), nie owinięte w `{ "value": ... }`. Zwraca `{}` gdy brak wpisów (nie 404).

### Feature validation w items

W `create_item` i `update_item` — po ownership check, przed INSERT/UPDATE.

**Pobieranie features:** ownership check w item handlerach nie dołącza features subquery.

Dla `create_item` — ownership check zwraca tylko `id`:
```sql
SELECT id FROM lists WHERE id = ?1 AND user_id = ?2
```
`list_id` jest znany z request body — dodajemy osobny SELECT:
```sql
SELECT feature_name FROM list_features WHERE list_id = ?1
```

Dla `update_item` — ownership check łączy `items` z `lists` ale nie zwraca `list_id`. Zmiana ownership query:
```sql
-- było:
SELECT items.id FROM items JOIN lists ON lists.id = items.list_id
WHERE items.id = ?1 AND lists.user_id = ?2
-- po zmianie:
SELECT items.id, items.list_id FROM items JOIN lists ON lists.id = items.list_id
WHERE items.id = ?1 AND lists.user_id = ?2
```
Następnie `SELECT feature_name FROM list_features WHERE list_id = ?1` z uzyskanym `list_id`.

**Helper i pattern użycia:**

```rust
fn check_item_features(
    features: &[String], // nazwy features z listy
    has_date_field: bool,
    has_quantity_field: bool,
) -> Option<Response> {
    if has_date_field && !features.iter().any(|f| f == FEATURE_DEADLINES) {
        return Some(Response::error(
            r#"{"error":"feature_required","feature":"deadlines","message":"This list does not have the 'deadlines' feature enabled. Enable it in list settings or retry without date fields."}"#,
            422,
        ).unwrap());
    }
    if has_quantity_field && !features.iter().any(|f| f == FEATURE_QUANTITY) {
        return Some(Response::error(
            r#"{"error":"feature_required","feature":"quantity","message":"This list does not have the 'quantity' feature enabled. Enable it in list settings or retry without quantity fields."}"#,
            422,
        ).unwrap());
    }
    None
}

// W handlerze:
if let Some(err) = check_item_features(&features, has_date_field, has_quantity_field) {
    return Ok(err);
}
```

**Definicja `has_date_field` i `has_quantity_field`:**

Dla `CreateItemRequest` (wszystkie pola `Option<T>`):
```
has_date_field    = start_date.is_some() || deadline.is_some() || hard_deadline.is_some()
                    || start_time.is_some() || deadline_time.is_some()
has_quantity_field = quantity.is_some() || unit.is_some()
```

Dla `UpdateItemRequest` (daty: `Option<Option<String>>`, quantity/unit: `Option<T>`):
```
has_date_field    = start_date == Some(Some(_)) || deadline == Some(Some(_))
                    || hard_deadline == Some(Some(_)) || start_time == Some(Some(_))
                    || deadline_time == Some(Some(_))
// Some(None) = clear, None = no change — nie triggerują
has_quantity_field = quantity.is_some() || actual_quantity.is_some() || unit.is_some()
// unit: Option<String> (bez podwójnego Option) — Some(_) zawsze triggeruje
```

**`move_item` nie jest objęty walidacją** — itemy z datami/ilościami mogą być przenoszone do list bez tych features. Dane są zachowane; frontend ukrywa pola gdy feature nie jest aktywny. To celowa decyzja (soft-remove semantics z M3).

## MCP Changes

### Usunięcie `ensureFeatures`

Funkcja `ensureFeatures` i jej wywołania usunięte. Zaktualizowane opisy narzędzi:

- `add_item`: _"Add a new item to a list. Returns an error if the list does not have the required feature enabled (use enable_list_feature to enable it first)."_
- `update_item`: _"Update an item. Returns an error if updating feature-gated fields on a list without the required feature enabled."_

### Nowe narzędzie `enable_list_feature`

```ts
server.registerTool("enable_list_feature", {
  description: "Enable a feature on a list. For 'deadlines', optionally configure which date fields are available. Call only after user confirms.",
  inputSchema: {
    list_id: z.string().describe("The list ID"),
    feature: z.enum(["quantity", "deadlines"]).describe("Feature to enable"),
    // deadlines sub-config (optional, defaults: start=false, deadline=true, hard=false)
    has_start_date: z.boolean().optional(),
    has_deadline: z.boolean().optional(),
    has_hard_deadline: z.boolean().optional(),
    // quantity sub-config (optional)
    unit_default: z.string().optional().describe("Default unit label, e.g. 'szt', 'kg'"),
  },
}, async ({ list_id, feature, has_start_date, has_deadline, has_hard_deadline, unit_default }) => {
  const config = feature === "deadlines"
    ? {
        has_start_date: has_start_date ?? false,
        has_deadline: has_deadline ?? true,
        has_hard_deadline: has_hard_deadline ?? false,
      }
    : unit_default
      ? { unit_default }
      : {};
  return callTool(api, "POST", `/api/lists/${list_id}/features/${feature}`, { config });
});

server.registerTool("disable_list_feature", {
  description: "Disable a feature on a list. Item data (quantities, dates) is preserved.",
  inputSchema: {
    list_id: z.string(),
    feature: z.enum(["quantity", "deadlines"]),
  },
}, ({ list_id, feature }) =>
  callTool(api, "DELETE", `/api/lists/${list_id}/features/${feature}`));
```

### Auto-enable logic w `add_item` / `update_item`

MCP server jest **stateless per-request** (CF Workers, `sessionIdGenerator: undefined`). Brak persystentnego kontekstu sesji — żadne dane nie przeżywają między requestami.

Auto-enable implementowane jako **on-demand fetch** wewnątrz tool handlerów, gdy API zwróci 422:

```ts
// pseudo-kod wewnątrz add_item / update_item
// Używamy apiCall zamiast callTool — potrzebujemy dostępu do surowej odpowiedzi
// zanim sformatujemy ToolResult, żeby parsować JSON bez prefiksu "API error 422: "
const res = await apiCall(api, "POST", `/api/lists/${list_id}/items`, fields);
if (!res.ok) {
  if (res.status === 422) {
    const body = await res.json() as { error?: string; feature?: string; message?: string };
    if (body.error === "feature_required" && body.feature) {
      const settings = await apiCall(api, "GET", "/api/settings")
        .then(r => r.json()).catch(() => ({}));
      if (settings["mcp_auto_enable_features"] === true) {
        const config = body.feature === "deadlines"
          ? { has_start_date: false, has_deadline: true, has_hard_deadline: false }
          : {};
        await apiCall(api, "POST", `/api/lists/${list_id}/features/${body.feature}`, { config });
        return callTool(api, "POST", `/api/lists/${list_id}/items`, fields); // retry
      }
      return errorResult(
        `${body.message} Options: (1) enable with enable_list_feature, (2) retry without the field.`
      );
    }
  }
  return errorResult(`API error ${res.status}: ${await res.text()}`);
}
return jsonResult(await res.json());
```

Analogicznie w `update_item`. Każda zmiana `mcp_auto_enable_features` w UI natychmiast obowiązuje (czytamy przy każdej 422 — brak problemu ze stale state).

## Frontend — Settings Page

W `pages/settings.rs` — nowa sekcja "Zachowanie AI":

```
┌─────────────────────────────────────────┐
│ Zachowanie AI                           │
│                                         │
│ [toggle] Automatycznie włączaj funkcje  │
│          list gdy AI ich potrzebuje     │
│          (np. terminy, ilości)          │
└─────────────────────────────────────────┘
```

Toggle wywołuje `PUT /api/settings/mcp_auto_enable_features` z `{ "value": true/false }`. Ładuje aktualną wartość przez `GET /api/settings` przy mount.

## Przepływ po zmianie

**Scenariusz: user prosi Claude o dodanie itemu z deadline do listy bez deadlines**

1. Claude wywołuje `add_item` z `deadline: "2026-04-01"`
2. API zwraca 422: `{"error":"feature_required","feature":"deadlines",...}`
3. MCP zwraca `isError: true` z tym komunikatem do Claude
4. **Jeśli `auto_enable = false`:** Claude mówi userowi: "Ta lista nie ma włączonego feature 'deadlines'. Mam go włączyć, czy dodać bez deadline?"
5. **Jeśli `auto_enable = true`:** Claude wywołuje `enable_list_feature` (deadlines, defaults), następnie retry `add_item`

## Pliki do modyfikacji

### Backend
- `crates/api/migrations/0011_user_settings.sql` — nowa migracja
- `crates/shared/src/lib.rs` — `UserSetting`, `UpsertSettingRequest`, stałe kluczy
- `crates/api/src/handlers/` — nowy `settings.rs`
- `crates/api/src/handlers/items.rs` — `validate_item_features` helper, wywołanie w create/update
- `crates/api/src/router.rs` — nowe routes `/api/settings`

### Frontend
- `crates/frontend/src/api/mod.rs` (lub nowy `api/settings.rs`) — fetch/upsert settings
- `crates/frontend/src/pages/settings.rs` — toggle AI preferences

### Gateway / MCP
- `gateway/src/mcp/api.ts` — usuń `ensureFeatures`
- `gateway/src/mcp/tools/items.ts` — usuń wywołania `ensureFeatures`
- `gateway/src/mcp/tools/lists.ts` — dodaj `enable_list_feature`, `disable_list_feature`

## Verification

0. Zweryfikować że `0011_user_settings.sql` nie koliduje z istniejącą migracją (sprawdzić `crates/api/migrations/`)
1. `just check` — kompilacja workspace
2. `just lint`
3. Ręczne testy:
   - `add_item` z `deadline` na liście bez deadlines → 422 z czytelnym błędem
   - `add_item` bez date fields na liście bez deadlines → OK
   - `enable_list_feature deadlines` przez MCP → lista ma feature, retry działa
   - Settings toggle → zapis do DB, MCP czyta przy kolejnej sesji
   - `auto_enable = true` → MCP automatycznie włącza i retry bez pytania
