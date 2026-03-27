-- M5: Containers (folders + projects) + pinning + recent tracking

CREATE TABLE containers (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT,  -- NULL=folder, 'active'/'done'/'paused'=project
    parent_container_id TEXT REFERENCES containers(id) ON DELETE SET NULL,
    position INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    last_opened_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_containers_user_id ON containers(user_id);
CREATE INDEX idx_containers_parent ON containers(parent_container_id);

-- Add container_id to lists (nullable FK, orphan on delete)
ALTER TABLE lists ADD COLUMN container_id TEXT REFERENCES containers(id) ON DELETE SET NULL;
CREATE INDEX idx_lists_container_id ON lists(container_id);

-- Pinning + recent tracking on lists
ALTER TABLE lists ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE lists ADD COLUMN last_opened_at TEXT;
