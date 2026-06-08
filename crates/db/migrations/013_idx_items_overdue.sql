-- Composite index for the overdue query:
--   WHERE l.user_id = ? AND i.deadline < ? AND i.completed = 0
-- list_id narrows to the user's lists (via JOIN l.id = i.list_id),
-- deadline enables the range scan, completed filters without post-scan.
CREATE INDEX IF NOT EXISTS idx_items_overdue
    ON items(list_id, deadline, completed)
    WHERE deadline IS NOT NULL AND completed = 0;
