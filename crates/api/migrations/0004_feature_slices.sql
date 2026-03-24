-- Feature slices: new columns on lists
ALTER TABLE lists ADD COLUMN parent_list_id TEXT REFERENCES lists(id) ON DELETE CASCADE;
ALTER TABLE lists ADD COLUMN position INTEGER DEFAULT 0;
ALTER TABLE lists ADD COLUMN archived INTEGER DEFAULT 0;
ALTER TABLE lists ADD COLUMN has_quantity INTEGER DEFAULT 0;
ALTER TABLE lists ADD COLUMN has_due_date INTEGER DEFAULT 0;

-- Feature slices: new columns on items
ALTER TABLE items ADD COLUMN quantity INTEGER;
ALTER TABLE items ADD COLUMN actual_quantity INTEGER DEFAULT 0;
ALTER TABLE items ADD COLUMN unit TEXT;
ALTER TABLE items ADD COLUMN due_date TEXT;
ALTER TABLE items ADD COLUMN due_time TEXT;

-- Data migration: rename existing list types
UPDATE lists SET has_quantity = 1 WHERE list_type IN ('shopping', 'packing');
UPDATE lists SET list_type = 'zakupy' WHERE list_type = 'shopping';
UPDATE lists SET list_type = 'pakowanie' WHERE list_type = 'packing';
UPDATE lists SET list_type = 'custom' WHERE list_type = 'project';

-- Indexes
CREATE INDEX idx_lists_parent ON lists(parent_list_id);
CREATE INDEX idx_lists_user_archived ON lists(user_id, archived);
CREATE INDEX idx_items_due_date ON items(due_date);
