CREATE TABLE IF NOT EXISTS user_preferences (
  user_id TEXT PRIMARY KEY,
  locale TEXT NOT NULL DEFAULT 'en',
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
