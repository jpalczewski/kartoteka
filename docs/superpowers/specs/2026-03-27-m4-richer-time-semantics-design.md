# M4: Bogatsza semantyka czasu — Design Spec

## Context

Kartoteka ma obecnie jedno pole daty na itemach (`due_date` + `due_time`), co wystarcza do prostego terminarza, ale nie pozwala na rozróżnienie między "kiedy zacząć", "kiedy chcę skończyć" i "absolutny termin". M4 wprowadza richer time model z trzema typami dat, budując na feature slice system z M3.

## Decyzje projektowe

1. **due_date → deadline** — rename istniejącego pola. Czysta semantyka bez duplikacji.
2. **hard_deadline to osobna data** (nie boolean flag) — pozwala mieć soft deadline 10.04 i hard deadline 15.04 na tym samym itemie.
3. **start_date + start_time** — spójne z deadline/deadline_time. "Zacznij o 9:00".
4. **Multi-date display** — item pojawia się na kalendarzu/widoku Dziś na każdej swojej dacie z badge (start/deadline/hard).
5. **Clearing dates** — `UpdateItemRequest` używa `Option<Option<String>>` dla pól dat (wzór z `UpdateTagRequest.parent_tag_id`). `None` = nie zmieniaj, `Some(None)` = wyczyść, `Some(Some(v))` = ustaw.

## Model danych

### Nowe kolumny na `items`

| Kolumna | Typ | Opis |
|---------|-----|------|
| `start_date` | TEXT | Data startu (YYYY-MM-DD) |
| `start_time` | TEXT | Godzina startu (HH:MM) |
| `deadline` | TEXT | Soft deadline — rename z `due_date` |
| `deadline_time` | TEXT | Godzina deadline — rename z `due_time` |
| `hard_deadline` | TEXT | Absolutny/twardy termin (YYYY-MM-DD) |

### Migracje

Rozdzielone na dwa pliki dla atomowości:

**`0008_richer_dates.sql`** — zmiany na tabeli `items`:
```sql
ALTER TABLE items ADD COLUMN start_date TEXT;
ALTER TABLE items ADD COLUMN start_time TEXT;
ALTER TABLE items ADD COLUMN hard_deadline TEXT;
ALTER TABLE items RENAME COLUMN due_date TO deadline;
ALTER TABLE items RENAME COLUMN due_time TO deadline_time;
```

**`0009_deadlines_feature.sql`** — migracja feature slice:
```sql
-- Migrate feature name and config
UPDATE list_features SET feature_name = 'deadlines', config = '{"has_start_date": false, "has_deadline": true, "has_hard_deadline": false}'
WHERE feature_name = 'due_date';

-- Safety: remove any unknown feature names before recreating table
DELETE FROM list_features WHERE feature_name NOT IN ('quantity', 'deadlines');

-- Recreate list_features with updated CHECK constraint
CREATE TABLE list_features_new (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    feature_name TEXT NOT NULL CHECK(feature_name IN ('quantity', 'deadlines')),
    config TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (list_id, feature_name)
);
INSERT INTO list_features_new SELECT * FROM list_features;
DROP TABLE list_features;
ALTER TABLE list_features_new RENAME TO list_features;
CREATE INDEX idx_list_features_list ON list_features(list_id);
```

## Feature slice: `"deadlines"`

### Stała

```rust
pub const FEATURE_DEADLINES: &str = "deadlines";
// Remove: pub const FEATURE_DUE_DATE: &str = "due_date";
```

### Config schema

```json
{
  "has_start_date": true,
  "has_deadline": true,
  "has_hard_deadline": false
}
```

### Default features per ListType

Update `ListType::default_features()`:
- `Terminarz` → `FEATURE_DEADLINES` with config `{"has_start_date": false, "has_deadline": true, "has_hard_deadline": false}`
- Other types → unchanged

## Shared structs changes

### `crates/shared/src/lib.rs`

**Item struct:**
- `due_date: Option<String>` → `deadline: Option<String>`
- `due_time: Option<String>` → `deadline_time: Option<String>`
- Add: `start_date: Option<String>`
- Add: `start_time: Option<String>`
- Add: `hard_deadline: Option<String>`

**DateItem struct:**
- Same renames and additions as Item
- Keep: `list_name`, `list_type`
- Add: `date_type: Option<String>` — populated by API when querying with `date_field=all`, `None` when querying specific field

**`From<DateItem> for Item` impl:**
- Update field names: `due_date` → `deadline`, `due_time` → `deadline_time`
- Forward new fields: `start_date`, `start_time`, `hard_deadline`
- Drop `date_type`, `list_name`, `list_type` (not in Item)

**CreateItemRequest:**
- `due_date` → `deadline`, `due_time` → `deadline_time`
- Add: `start_date`, `start_time`, `hard_deadline`

**UpdateItemRequest:**
- Rename: `due_date` → `deadline`, `due_time` → `deadline_time`
- Add: `start_date`, `start_time`, `hard_deadline`
- Change all date fields to `Option<Option<String>>` to support clearing (pattern from `UpdateTagRequest.parent_tag_id`)
- `None` = don't change, `Some(None)` = clear to NULL, `Some(Some(v))` = set value

### New: `DateField` enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DateField {
    StartDate,
    Deadline,
    HardDeadline,
}

impl DateField {
    pub fn column_name(&self) -> &'static str {
        match self {
            Self::StartDate => "start_date",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }

    pub fn time_column_name(&self) -> Option<&'static str> {
        match self {
            Self::StartDate => Some("start_time"),
            Self::Deadline => Some("deadline_time"),
            Self::HardDeadline => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::StartDate => "start",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }
}
```

## API changes

### `crates/api/src/handlers/items.rs`

#### `create` / `update`
- Handle new fields: `start_date`, `start_time`, `hard_deadline`
- Rename `due_date`/`due_time` → `deadline`/`deadline_time` in SQL
- `update`: handle `Option<Option<String>>` for date fields — `None` = skip, `Some(None)` = SET NULL, `Some(Some(v))` = SET value

#### `by_date` endpoint
`GET /api/items/by-date?date=YYYY-MM-DD&date_field=deadline&include_overdue=true`

- New param `date_field`: `deadline` (default), `start_date`, `hard_deadline`, `all`
- When `date_field` is a single field: same logic as before, using that column
- When `date_field=all`: UNION ALL query, adds `date_type` column
- `include_overdue` applies to the selected `date_field` (for `all`, applies to each sub-query independently — only deadline has overdue semantics)

#### `calendar` endpoint
`GET /api/items/calendar?from=YYYY-MM-DD&to=YYYY-MM-DD&date_field=deadline&detail=counts`

- New param `date_field`: same as `by_date`
- When `date_field=all` with `detail=counts`: use `COUNT(DISTINCT i.id)` to avoid double-counting items that share dates across fields
- When `date_field=all` with `detail=full`: UNION ALL with `date_type`, items may appear multiple times (intended for badge display)

### SQL for `date_field=all` (by_date)

Use explicit column list (not `SELECT *`) to match `DateItem` struct and avoid D1 column name collisions:

```sql
SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position,
       i.quantity, i.actual_quantity, i.unit,
       i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline,
       i.created_at, i.updated_at,
       l.name as list_name, l.list_type,
       'start' as date_type
FROM items i JOIN lists l ON l.id = i.list_id
WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date = ?2

UNION ALL

SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position,
       i.quantity, i.actual_quantity, i.unit,
       i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline,
       i.created_at, i.updated_at,
       l.name as list_name, l.list_type,
       'deadline' as date_type
FROM items i JOIN lists l ON l.id = i.list_id
WHERE l.user_id = ?1 AND l.archived = 0
  AND (i.deadline = ?2 OR (i.deadline < ?2 AND i.completed = 0))

UNION ALL

SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position,
       i.quantity, i.actual_quantity, i.unit,
       i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline,
       i.created_at, i.updated_at,
       l.name as list_name, l.list_type,
       'hard_deadline' as date_type
FROM items i JOIN lists l ON l.id = i.list_id
WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline = ?2

ORDER BY completed ASC, list_name ASC, deadline_time ASC, position ASC
```

### SQL for `date_field=all` (calendar counts)

Use `COUNT(DISTINCT i.id)` to count unique items per day:

```sql
SELECT date, COUNT(DISTINCT id) as total, CAST(SUM(completed) AS INTEGER) / COUNT(*) as completed
FROM (
    SELECT i.id, i.start_date as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id
    WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date >= ?2 AND i.start_date <= ?3
    UNION ALL
    SELECT i.id, i.deadline as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id
    WHERE l.user_id = ?1 AND l.archived = 0 AND i.deadline >= ?2 AND i.deadline <= ?3
    UNION ALL
    SELECT i.id, i.hard_deadline as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id
    WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline >= ?2 AND i.hard_deadline <= ?3
)
GROUP BY date ORDER BY date ASC
```

## Frontend changes

### Multi-date display: duplicate items handling

When `date_field=all`, the same item can appear 2-3 times (once per date type). Key decisions:

1. **Optimistic toggle**: find ALL occurrences with same `id` in the items signal, toggle all at once
2. **Overdue classification**: based on `date_type` field, not always `deadline`. Item appearing due to `start_date` checks `start_date` for overdue, not `deadline`.
3. **DateItemRow**: accepts `date_type: Option<DateField>` prop to select which date to display in the date slot. When `date_type` is set, show that date field (not always `deadline`).

### Components to modify

| File | Change |
|------|--------|
| `components/items/add_item_input.rs` | Dynamic date fields based on `deadlines` feature config. Show start_date+start_time, deadline+deadline_time, hard_deadline based on config booleans. |
| `components/items/item_row.rs` | Show date badges (color-coded): start=blue, deadline=amber, hard_deadline=red |
| `components/items/date_item_row.rs` | Accept `date_type: Option<DateField>` prop. Display the date from the matching field. Add color-coded badge. Rename due_date → deadline. |
| `components/common/date_utils.rs` | Rename `sort_by_due_date` → `sort_by_deadline`. Add `sort_by_date_field(items, field: DateField)`. Rename `is_overdue` refs from `due_date` → `deadline`. Add `is_overdue_for_field(item, field: DateField)`. |
| `pages/today.rs` | Query with `date_field=all`. Update `render_groups` to handle `date_type`. Update toggle logic to find ALL items with same `id`. Overdue classification per `date_type`. Update inline `UpdateItemRequest` struct construction. |
| `pages/calendar/month_grid.rs` | Use `date_field=all` for calendar counts. |
| `pages/calendar/day.rs` | Use `date_field=all` for day detail. Show badges. Update inline `UpdateItemRequest` construction. |
| `pages/list/date_view.rs` | Rename due_date → deadline. Update sort calls. |
| `pages/list/normal_view.rs` | Update field names in item display. |
| `api/items.rs` | Rename due_date→deadline in API calls, add new fields to request/response handling. |
| `api/lists.rs` | Update feature name references (due_date → deadlines). |

### Badge design

- **Start** — blue outline badge, icon: play/calendar-start
- **Deadline** — amber/yellow badge, icon: clock
- **Hard deadline** — red badge, icon: alert-triangle

### Feature config UI

In list create/edit, when toggling `deadlines` feature:
- Checkboxes for: Start date, Deadline, Hard deadline
- At least one must be checked when feature is enabled

## Verification

1. `just check` — workspace compiles
2. `just lint` — clippy + fmt pass
3. `just dev` — local testing:
   - Create list with `deadlines` feature (various configs)
   - Add item with start_date, deadline, hard_deadline
   - Verify Today page shows item on each date with correct badge
   - Verify Calendar shows item on multiple dates with correct counts (no double-counting in counts view)
   - Verify existing Terminarz lists still work (migrated due_date → deadline)
   - Verify date clearing works (set a date, then remove it via update)
   - Verify overdue classification works per date_type
4. Test migration on dev D1 database (0008 then 0009 separately)
5. `just ci` — full CI pipeline
