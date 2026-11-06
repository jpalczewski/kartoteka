-- ── Calendar feed tokens (iCal subscribe URLs) ────────────────────────────────
-- token_hash holds the hex SHA-256 of the secret token; the plaintext token is
-- only ever shown to the user once, at creation time.
CREATE TABLE IF NOT EXISTS calendar_tokens (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    list_id      TEXT REFERENCES lists(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    token_hash   TEXT NOT NULL UNIQUE,
    last_used_at TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE INDEX IF NOT EXISTS idx_calendar_tokens_user ON calendar_tokens(user_id);
