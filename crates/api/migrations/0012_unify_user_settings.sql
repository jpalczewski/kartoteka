CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, key)
);

-- Migrate locale data from old table (if it exists and hasn't been migrated yet)
INSERT OR IGNORE INTO user_settings (user_id, key, value, updated_at)
SELECT user_id, 'locale', '"' || locale || '"', updated_at
FROM user_preferences
WHERE EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='user_preferences');

DROP TABLE IF EXISTS user_preferences;
