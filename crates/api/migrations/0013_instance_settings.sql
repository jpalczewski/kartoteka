-- Instance-wide settings (key-value store, JSON values)
CREATE TABLE instance_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Default: registration is open
INSERT INTO instance_settings (key, value) VALUES ('registration_mode', '"open"');

-- Users table: mirrors Better Auth user IDs, stores app-level fields
-- Designed as future primary users table (issue #24 Rust binary migration)
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT,
    is_admin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Invitation codes with reservation to prevent race conditions
CREATE TABLE invitation_codes (
    id TEXT PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    created_by TEXT NOT NULL,
    used_by TEXT,
    reserved_by_email TEXT,
    reserved_until TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    used_at TEXT,
    expires_at TEXT
);
