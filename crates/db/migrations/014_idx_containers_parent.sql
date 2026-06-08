-- Covers the recursive JOIN in is_descendant and direct lookup in children():
--   JOIN subtree s ON c.parent_container_id = s.id  (is_descendant CTE)
--   WHERE parent_container_id = ?  (children direct query)
-- Partial index (WHERE NOT NULL) keeps it compact — root containers
-- have NULL parent and never appear in either query.
CREATE INDEX IF NOT EXISTS idx_containers_parent
    ON containers(parent_container_id)
    WHERE parent_container_id IS NOT NULL;
