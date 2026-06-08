-- Intended for queries filtering on hard_deadline directly (e.g. range scans).
-- Note: EXPLAIN QUERY PLAN shows by_date's OR condition
--   (i.start_date = ? OR i.deadline = ? OR i.hard_deadline = ?)
-- is NOT covered by this index — SQLite prefers idx_items_list_id and filters
-- the OR in memory within each list. This index would only be chosen by the
-- planner for a standalone hard_deadline lookup without a list_id filter.
CREATE INDEX IF NOT EXISTS idx_items_hard_deadline
    ON items(hard_deadline)
    WHERE hard_deadline IS NOT NULL;
