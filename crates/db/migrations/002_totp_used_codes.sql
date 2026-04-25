-- Track accepted TOTP codes to block replay within the validity window.
-- check_totp_code uses skew=1 + step=30s, so a code is valid for ~90s; rows
-- older than that can be GC'd, but keeping a small buffer is cheap.
CREATE TABLE IF NOT EXISTS totp_used_codes (
    user_id TEXT NOT NULL REFERENCES users(id),
    code    TEXT NOT NULL,
    used_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (user_id, code)
);

CREATE INDEX IF NOT EXISTS idx_totp_used_codes_used_at
    ON totp_used_codes(used_at);
