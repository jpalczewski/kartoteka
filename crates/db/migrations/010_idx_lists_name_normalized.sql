-- Expression index for list name dedup check in create_list.
-- Turns LOWER(TRIM(name)) scan into an index lookup; covers full WHERE clause.
CREATE INDEX IF NOT EXISTS idx_lists_name_normalized
    ON lists(user_id, LOWER(TRIM(name)), container_id, parent_list_id);
