-- Tags table
CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#888888',
    category TEXT NOT NULL DEFAULT 'custom',
    parent_tag_id TEXT REFERENCES tags(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Junction: items <-> tags
CREATE TABLE item_tags (
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_id)
);

-- Junction: lists <-> tags
CREATE TABLE list_tags (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (list_id, tag_id)
);

CREATE INDEX idx_tags_user ON tags(user_id);
CREATE INDEX idx_tags_user_cat ON tags(user_id, category);
CREATE INDEX idx_item_tags_item ON item_tags(item_id);
CREATE INDEX idx_item_tags_tag ON item_tags(tag_id);
CREATE INDEX idx_list_tags_list ON list_tags(list_id);
CREATE INDEX idx_list_tags_tag ON list_tags(tag_id);
