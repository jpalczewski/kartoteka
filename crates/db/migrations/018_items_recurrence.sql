-- ── Recurring items (contract types only; behaviour lands separately) ─────────
-- recurrence_rule holds the compact form "7d" / "2w" / "1m" (TEXT, decoded by
-- the RecurrenceRule FromStr impl). recurrence_mode is 'fixed' | 'floating';
-- anything else is rejected by the CHECK constraint.
ALTER TABLE items ADD COLUMN recurrence_rule TEXT;
ALTER TABLE items ADD COLUMN recurrence_mode TEXT CHECK (recurrence_mode IN ('fixed', 'floating'));
