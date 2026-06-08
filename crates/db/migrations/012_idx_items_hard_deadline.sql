-- Covers the third OR branch in by_date:
--   AND (i.start_date = ? OR i.deadline = ? OR i.hard_deadline = ?)
-- Without this index, SQLite falls back to a full items scan whenever
-- hard_deadline is the only matching column.
CREATE INDEX IF NOT EXISTS idx_items_hard_deadline
    ON items(hard_deadline)
    WHERE hard_deadline IS NOT NULL;
