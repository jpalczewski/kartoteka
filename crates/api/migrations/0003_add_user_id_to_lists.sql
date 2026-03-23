ALTER TABLE lists ADD COLUMN user_id TEXT NOT NULL DEFAULT '';
CREATE INDEX idx_lists_user_id ON lists(user_id);
