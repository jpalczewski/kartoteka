-- Backfill existing never-expiring personal access tokens with a 90-day TTL.
-- After this migration, all tokens have an expiry and validate_jwt enforces it.
UPDATE personal_tokens
SET expires_at = strftime('%Y-%m-%dT%H:%M:%fZ', datetime('now', '+90 days'))
WHERE expires_at IS NULL;
