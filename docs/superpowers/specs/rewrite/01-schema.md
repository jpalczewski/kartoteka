# Schema: Consolidated SQLite Migration

Parent: `00-overview.md`

One consolidated migration (not 10 incremental D1 migrations). Clean cut — no data migration from D1. All tables STRICT where applicable.

```sql
-- Auth
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    name TEXT,
    avatar_url TEXT,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE auth_methods (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    provider TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    credential TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(provider, provider_id)
);

CREATE TABLE totp_secrets (
    user_id TEXT PRIMARY KEY REFERENCES users(id),
    secret TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE personal_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'full',
    last_used_at TEXT,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Server config
CREATE TABLE server_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

INSERT OR IGNORE INTO server_config (key, value) VALUES ('registration_enabled', 'true');

-- OAuth
CREATE TABLE oauth_clients (
    client_id TEXT PRIMARY KEY,
    client_name TEXT,
    redirect_uris TEXT NOT NULL,
    grant_types TEXT NOT NULL,
    token_endpoint_auth_method TEXT DEFAULT 'none',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE oauth_authorization_codes (
    code TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
    user_id TEXT NOT NULL REFERENCES users(id),
    redirect_uri TEXT NOT NULL,
    code_challenge TEXT NOT NULL,
    code_challenge_method TEXT NOT NULL DEFAULT 'S256',
    scopes TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE oauth_refresh_tokens (
    token TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
    user_id TEXT NOT NULL REFERENCES users(id),
    scopes TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- App data
CREATE TABLE containers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    icon TEXT,
    description TEXT,
    status TEXT,
    parent_container_id TEXT REFERENCES containers(id),
    position INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    last_opened_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE lists (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    icon TEXT,
    description TEXT,
    list_type TEXT NOT NULL DEFAULT 'checklist',
    parent_list_id TEXT REFERENCES lists(id),
    position INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    container_id TEXT REFERENCES containers(id),
    pinned INTEGER NOT NULL DEFAULT 0,
    last_opened_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE items (
    id TEXT PRIMARY KEY,
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    completed INTEGER NOT NULL DEFAULT 0,
    position INTEGER NOT NULL DEFAULT 0,
    quantity INTEGER,
    actual_quantity INTEGER,
    unit TEXT,
    start_date TEXT,
    start_time TEXT,
    deadline TEXT,
    deadline_time TEXT,
    hard_deadline TEXT,
    estimated_duration INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    icon TEXT,
    color TEXT,
    parent_tag_id TEXT REFERENCES tags(id),
    tag_type TEXT NOT NULL DEFAULT 'tag',
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE item_tags (
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_id)
);

CREATE TABLE list_tags (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (list_id, tag_id)
);

CREATE TABLE container_tags (
    container_id TEXT NOT NULL REFERENCES containers(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (container_id, tag_id)
);

CREATE TABLE list_features (
    list_id TEXT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    feature_name TEXT NOT NULL,
    config TEXT NOT NULL DEFAULT '{}',
    PRIMARY KEY (list_id, feature_name)
);

CREATE TABLE user_settings (
    user_id TEXT NOT NULL REFERENCES users(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, key)
);

-- Comments (polymorphic)
CREATE TABLE comments (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    content TEXT NOT NULL,
    author_type TEXT NOT NULL DEFAULT 'user',
    author_name TEXT,
    user_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Relations (polymorphic)
CREATE TABLE entity_relations (
    id TEXT PRIMARY KEY,
    from_type TEXT NOT NULL,
    from_id TEXT NOT NULL,
    to_type TEXT NOT NULL,
    to_id TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(from_type, from_id, to_type, to_id, relation_type)
) STRICT;

-- Time tracking
CREATE TABLE time_entries (
    id TEXT PRIMARY KEY,
    item_id TEXT REFERENCES items(id) ON DELETE SET NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    description TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    duration INTEGER,
    source TEXT NOT NULL,
    mode TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Templates
CREATE TABLE templates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL,
    icon TEXT,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE template_items (
    id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    quantity INTEGER,
    unit TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE template_tags (
    template_id TEXT NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (template_id, tag_id)
);

-- FTS5
CREATE VIRTUAL TABLE items_fts USING fts5(title, description, content=items, content_rowid=rowid);
CREATE VIRTUAL TABLE comments_fts USING fts5(content, content=comments, content_rowid=rowid);

CREATE TRIGGER items_fts_insert AFTER INSERT ON items BEGIN
    INSERT INTO items_fts(rowid, title, description) VALUES (new.rowid, new.title, new.description);
END;
CREATE TRIGGER items_fts_update AFTER UPDATE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, title, description) VALUES ('delete', old.rowid, old.title, old.description);
    INSERT INTO items_fts(rowid, title, description) VALUES (new.rowid, new.title, new.description);
END;
CREATE TRIGGER items_fts_delete AFTER DELETE ON items BEGIN
    INSERT INTO items_fts(items_fts, rowid, title, description) VALUES ('delete', old.rowid, old.title, old.description);
END;
CREATE TRIGGER comments_fts_insert AFTER INSERT ON comments BEGIN
    INSERT INTO comments_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER comments_fts_update AFTER UPDATE ON comments BEGIN
    INSERT INTO comments_fts(comments_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
    INSERT INTO comments_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER comments_fts_delete AFTER DELETE ON comments BEGIN
    INSERT INTO comments_fts(comments_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
END;

-- Indexes
CREATE INDEX idx_items_list_id ON items(list_id);
CREATE INDEX idx_items_deadline ON items(deadline) WHERE deadline IS NOT NULL;
CREATE INDEX idx_items_start_date ON items(start_date) WHERE start_date IS NOT NULL;
CREATE INDEX idx_lists_user_id ON lists(user_id);
CREATE INDEX idx_lists_pinned ON lists(user_id, pinned) WHERE pinned = 1;
CREATE INDEX idx_lists_container ON lists(container_id) WHERE container_id IS NOT NULL;
CREATE INDEX idx_containers_user_id ON containers(user_id);
CREATE INDEX idx_containers_pinned ON containers(user_id, pinned) WHERE pinned = 1;
CREATE INDEX idx_auth_methods_user_provider ON auth_methods(user_id, provider);
CREATE INDEX idx_tags_user_id ON tags(user_id);
CREATE INDEX idx_tags_user_type ON tags(user_id, tag_type);
CREATE INDEX idx_comments_entity ON comments(entity_type, entity_id);
CREATE INDEX idx_relations_from ON entity_relations(from_type, from_id);
CREATE INDEX idx_relations_to ON entity_relations(to_type, to_id);
CREATE INDEX idx_time_entries_item ON time_entries(item_id) WHERE item_id IS NOT NULL;
CREATE INDEX idx_time_entries_user_unassigned ON time_entries(user_id) WHERE item_id IS NULL;
CREATE INDEX idx_template_items_template ON template_items(template_id);
CREATE INDEX idx_personal_tokens_user ON personal_tokens(user_id);

-- Sessions: auto-created by tower-sessions SqliteStore::migrate()
```
