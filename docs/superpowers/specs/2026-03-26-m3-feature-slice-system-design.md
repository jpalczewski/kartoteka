# M3: Konfigurowalny Feature Slice System

## Context

Obecne feature slices (`has_quantity`, `has_due_date`) to flat booleany na tabeli `lists`. Nie skaluje się — każdy nowy feature wymaga nowej kolumny, migracji DB i hardcoded warunków w komponentach. M3 zamienia to na elastyczny system: osobna tabela `list_features` z konfiguracją JSON per feature.

## Scope

- Migracja istniejących `has_quantity` / `has_due_date` do nowej tabeli
- Clean cut — usunięcie starych kolumn w tym samym milestone
- Presets per `ListType` (Zakupy → quantity, Terminarz → due_date)
- CRUD API per feature
- Dynamiczny UI renderujący pola na podstawie aktywnych features
- **Nie** w scope: nowe feature types (M4+), feature config UI (prosty toggle wystarczy)

## Database

### Nowa tabela

```sql
CREATE TABLE list_features (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    feature_name TEXT NOT NULL CHECK(feature_name IN ('quantity', 'due_date')),
    config TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (list_id, feature_name)
);
CREATE INDEX idx_list_features_list ON list_features(list_id);
```

`CHECK` constraint ogranicza do znanych features. **Uwaga:** rozszerzenie CHECK w SQLite wymaga recreate table (`CREATE TABLE new AS SELECT...`, DROP, RENAME) — przyszłe migracje dodające nowe feature types muszą to uwzględnić.

### Migracja danych

```sql
INSERT INTO list_features (list_id, feature_name, config)
SELECT id, 'quantity', '{"unit_default": "szt"}' FROM lists WHERE has_quantity = 1;

INSERT INTO list_features (list_id, feature_name, config)
SELECT id, 'due_date', '{}' FROM lists WHERE has_due_date = 1;

ALTER TABLE lists DROP COLUMN has_quantity;
ALTER TABLE lists DROP COLUMN has_due_date;
```

### Query pattern — inline features

Wszystkie SELECT na `lists` dołączają features przez subquery:

```sql
SELECT l.id, l.user_id, l.name, l.description, l.list_type,
       l.parent_list_id, l.position, l.archived, l.created_at, l.updated_at,
       COALESCE(
         (SELECT json_group_array(json_object('name', lf.feature_name, 'config', json(lf.config)))
          FROM list_features lf WHERE lf.list_id = l.id),
         '[]'
       ) as features
FROM lists l
WHERE ...
```

## Shared Types (`crates/shared/src/lib.rs`)

### Nowe typy

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListFeature {
    pub name: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Known feature names
pub const FEATURE_QUANTITY: &str = "quantity";
pub const FEATURE_DUE_DATE: &str = "due_date";
```

### Zmiany w `List`

```rust
pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub list_type: ListType,
    pub parent_list_id: Option<String>,
    pub position: i32,
    #[serde(deserialize_with = "bool_from_number")]
    pub archived: bool,
    // USUNIĘTE: has_quantity, has_due_date
    #[serde(default, deserialize_with = "features_from_json")]
    pub features: Vec<ListFeature>,
    pub created_at: String,
    pub updated_at: String,
}

impl List {
    pub fn has_feature(&self, name: &str) -> bool {
        self.features.iter().any(|f| f.name == name)
    }
}
```

Custom deserializer `features_from_json` — parsuje JSON string z subquery do `Vec<ListFeature>`:

```rust
fn features_from_json<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<ListFeature>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::String(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom),
        serde_json::Value::Array(_) => serde_json::from_value(v).map_err(serde::de::Error::custom),
        serde_json::Value::Null => Ok(vec![]),
        _ => Ok(vec![]),
    }
}
```

Defensywnie obsługuje zarówno String (D1 zwraca TEXT), Array (jeśli driver pre-parsuje), jak i Null.

### Decyzje behawioralne

- **Usunięcie feature** (`remove_feature`): dane na itemach (quantity, due_date) **nie są kasowane** (soft-remove). Frontend ukrywa pola, ale dane przetrwają re-enable.
- **Zmiana ListType** via `UpdateListRequest`: **nie synchronizuje features**. User zarządza features jawnie.
- **add_feature** z istniejącym feature: `INSERT OR REPLACE` — aktualizuje config (idempotentne).
- **Walidacja config**: `add_feature` handler waliduje body.config jako valid JSON przed INSERT.

### Zmiany w request types

```rust
pub struct CreateListRequest {
    pub name: String,
    pub list_type: ListType,
    pub features: Option<Vec<ListFeature>>, // None → default from ListType
}

pub struct UpdateListRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub list_type: Option<ListType>,
    pub archived: Option<bool>,
    // USUNIĘTE: has_quantity, has_due_date
}
```

### ListType presets

```rust
impl ListType {
    pub fn default_features(&self) -> Vec<ListFeature> {
        match self {
            Self::Zakupy | Self::Pakowanie => vec![
                ListFeature { name: FEATURE_QUANTITY.into(), config: json!({"unit_default": "szt"}) },
            ],
            Self::Terminarz => vec![
                ListFeature { name: FEATURE_DUE_DATE.into(), config: json!({}) },
            ],
            _ => vec![],
        }
    }
}
```

## API Endpoints

### Zmienione endpointy

Wszystkie handlery w `lists.rs` które robią SELECT na lists — dodają subquery features (patrz query pattern wyżej). Dotyczy: `list_all`, `create`, `get_one`, `update`, `delete` (return after), `list_sublists`, `list_archived`, `toggle_archive`, `reset`, `create_sublist`.

### `create` — insertuje list + default features

1. INSERT INTO lists (bez has_quantity/has_due_date)
2. Determine features: `request.features.unwrap_or_else(|| list_type.default_features())`
3. INSERT INTO list_features per feature

### Nowe endpointy — CRUD features per lista

| Method | Path | Opis |
|--------|------|------|
| POST | `/api/lists/:id/features/:name` | Dodaj/update feature (body: `{ "config": {} }`). Używa `INSERT OR REPLACE` dla idempotentności. Waliduje config jako valid JSON. |
| DELETE | `/api/lists/:id/features/:name` | Usuń feature. Dane na itemach (quantity, due_date) zostają — soft-remove. |

Router additions:
```rust
.post_async("/api/lists/:id/features/:name", lists::add_feature)
.delete_async("/api/lists/:id/features/:name", lists::remove_feature)
```

## Frontend

### Shared / API layer (`crates/frontend/src/api/lists.rs`)

- `CreateListRequest` / `UpdateListRequest` — dopasowane do nowych shared types
- Nowe: `add_feature(list_id, name, config)`, `remove_feature(list_id, name)`

### Komponent changes — feature props

Zamiast `has_quantity: bool` + `has_due_date: bool` → `features: Vec<ListFeature>` (lub `RwSignal<Vec<ListFeature>>`).

Helper na frontendzie:
```rust
fn has_feature(features: &[ListFeature], name: &str) -> bool {
    features.iter().any(|f| f.name == name)
}
```

### Pliki do zmiany

| Plik | Zmiana |
|------|--------|
| `pages/home.rs` | Create form: feature toggles zamiast has_quantity/has_due_date checkboxes, auto-preset per ListType |
| `pages/list/mod.rs` | `list_has_quantity`/`list_has_due_date` → `list_features: RwSignal<Vec<ListFeature>>`, pass to children |
| `pages/list/normal_view.rs` | Props: `features` zamiast `has_quantity` |
| `components/items/add_item_input.rs` | Props: `features` → dynamicznie pokazuj pola quantity/due_date |
| `components/items/item_row.rs` | Props: `features` zamiast `has_quantity` |
| `components/lists/sublist_section.rs` | Props: `features` zamiast `has_quantity`/`has_due_date` |
| `components/lists/list_header.rs` | Opcjonalnie: feature toggle UI w settings listy |

### List page — feature management UI

W `list_header.rs` lub nowy komponent — toggles do włączania/wyłączania features na istniejącej liście. Wywołuje `add_feature`/`remove_feature` API i odświeża `list_features` signal.

### Home page — create form

Przy wyborze `ListType` — auto-ustawia checkboxy features (presets). User może override.

## Pliki do modyfikacji — pełna lista

### Backend
- `crates/shared/src/lib.rs` — nowe typy, zmiana List/CreateListRequest/UpdateListRequest
- `crates/api/migrations/0007_list_features.sql` — nowa migracja
- `crates/api/src/handlers/lists.rs` — wszystkie handlery + nowe add_feature/remove_feature
- `crates/api/src/router.rs` — nowe routes

### Frontend
- `crates/frontend/src/api/lists.rs` — request types + nowe API calls
- `crates/frontend/src/pages/home.rs` — create form
- `crates/frontend/src/pages/list/mod.rs` — feature signals
- `crates/frontend/src/pages/list/normal_view.rs` — props
- `crates/frontend/src/components/items/add_item_input.rs` — props
- `crates/frontend/src/components/items/item_row.rs` — props
- `crates/frontend/src/components/lists/sublist_section.rs` — props
- `crates/frontend/src/components/lists/list_header.rs` — feature toggle UI

## Verification

1. `just check` — kompilacja workspace
2. `just lint` — clippy + fmt
3. `just dev` — lokalne testowanie:
   - Utwórz listę Zakupy → automatycznie ma feature quantity
   - Utwórz listę Terminarz → automatycznie ma feature due_date
   - Utwórz listę Custom → brak features
   - Dodaj feature quantity do Custom → pola quantity pojawiają się w item form
   - Usuń feature → pola znikają
   - Istniejące listy (po migracji) mają poprawne features
4. `just deploy` — deploy + smoke test
