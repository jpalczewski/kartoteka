CREATE TABLE user_settings (
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, key)
);

-- Migrate locale data from old table
INSERT INTO user_settings (user_id, key, value, updated_at)
SELECT user_id, 'locale', '"' || locale || '"', updated_at
FROM user_preferences;

DROP TABLE user_preferences;
