-- Composite index for list_container_items non-recursive path.
-- Existing idx_lists_container covers only container_id; this adds user_id + archived
-- so the WHERE clause filters without a post-scan, reducing JOIN overhead.
CREATE INDEX IF NOT EXISTS idx_lists_container_user
    ON lists(container_id, user_id, archived)
    WHERE container_id IS NOT NULL;
