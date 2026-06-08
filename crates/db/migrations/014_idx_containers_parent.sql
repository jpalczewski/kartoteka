-- Covers the recursive step in list_for_container:
--   SELECT * FROM containers WHERE parent_container_id = sub.id
-- Partial index (WHERE NOT NULL) keeps it compact — root containers
-- have NULL parent and never appear in the recursive join condition.
CREATE INDEX IF NOT EXISTS idx_containers_parent
    ON containers(parent_container_id)
    WHERE parent_container_id IS NOT NULL;
