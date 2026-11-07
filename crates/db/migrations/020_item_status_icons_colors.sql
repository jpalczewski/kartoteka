-- ── Item status, item icons, list/container colours ──────────────────────────
-- `items.completed` keeps its meaning (the checkbox); `items.status` is the richer
-- three-state view shown by the status picker. The invariant is
--     completed = 1  <=>  status = 'done'
-- and it is enforced for every write path (including the ones outside
-- crates/db/src/items.rs) by the triggers at the bottom of this migration.

ALTER TABLE items ADD COLUMN status TEXT NOT NULL DEFAULT 'todo'
    CHECK (status IN ('todo', 'in_progress', 'done'));

-- NULL = no icon. Format is `lucide:<name>`, `emoji:<e>` or a legacy bare emoji.
ALTER TABLE items ADD COLUMN icon TEXT;

-- NULL = neutral (no accent colour). Format is `#rrggbb`.
ALTER TABLE lists ADD COLUMN color TEXT;
ALTER TABLE containers ADD COLUMN color TEXT;

-- Backfill: everything already checked off is `done`.
UPDATE items SET status = 'done' WHERE completed = 1;

-- ── Invariant safety net ─────────────────────────────────────────────────────
-- Any later `completed` write keeps `status` in sync, so a caller cannot leave
-- the two disagreeing even if it only touches `completed` (relations.rs, a bulk
-- uncheck, …).
--
-- Deliberately UPDATE only: an AFTER INSERT trigger that rewrote the new row
-- would race the FTS5 external-content triggers (items_fts_*) — trigger order is
-- not defined, and the fts 'delete' step of an already-inserted row corrupts the
-- index. Inserts therefore rely on the column default ('todo') plus the explicit
-- `status` writes in crates/db/src/items.rs; a new row is only ever completed
-- afterwards, through an UPDATE this trigger covers.
CREATE TRIGGER IF NOT EXISTS items_status_sync_update
AFTER UPDATE OF completed ON items
WHEN new.completed <> (new.status = 'done')
BEGIN
    UPDATE items
    SET status = CASE WHEN new.completed = 1 THEN 'done' ELSE 'todo' END
    WHERE id = new.id;
END;
