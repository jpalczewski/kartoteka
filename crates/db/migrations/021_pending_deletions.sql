-- ── Pending deletions (the /trash queue) ────────────────────────────────────
-- Marking an entity for deletion writes a row here instead of deleting it, so the
-- user decides later on /trash. MCP agents can only mark; nothing is hard-deleted
-- outside this queue (and the ordinary list/container/item/comment endpoints).
--
-- One row per (type, id): marking twice is a no-op, so the primary key doubles as
-- the uniqueness constraint.

CREATE TABLE IF NOT EXISTS pending_deletions (
    entity_type TEXT NOT NULL CHECK (entity_type IN ('item', 'list', 'container', 'comment')),
    entity_id   TEXT NOT NULL,
    user_id     TEXT NOT NULL REFERENCES users(id),
    marked_at   TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entity_type, entity_id)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_pending_deletions_user ON pending_deletions(user_id, marked_at);

-- The queue never points at a missing entity. Anything that deletes an entity — the
-- queue itself, the ordinary endpoints, an FK cascade (items.list_id ON DELETE
-- CASCADE) or the 017 orphan cleanup for comments — drops its queue row here.
-- Those cascaded/triggered deletes fire these triggers as well.

CREATE TRIGGER IF NOT EXISTS items_cleanup_pending_deletions
AFTER DELETE ON items
BEGIN
    DELETE FROM pending_deletions WHERE entity_type = 'item' AND entity_id = old.id;
END;

-- Deleting a list cascades to its items (items.list_id ON DELETE CASCADE), which
-- fires the item trigger above for each of them.
CREATE TRIGGER IF NOT EXISTS lists_cleanup_pending_deletions
AFTER DELETE ON lists
BEGIN
    DELETE FROM pending_deletions WHERE entity_type = 'list' AND entity_id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS containers_cleanup_pending_deletions
AFTER DELETE ON containers
BEGIN
    DELETE FROM pending_deletions WHERE entity_type = 'container' AND entity_id = old.id;
END;

-- Comments have no foreign key to their entity; the 017 triggers remove the
-- comments of a deleted item/list/container, which fires this one in turn.
CREATE TRIGGER IF NOT EXISTS comments_cleanup_pending_deletions
AFTER DELETE ON comments
BEGIN
    DELETE FROM pending_deletions WHERE entity_type = 'comment' AND entity_id = old.id;
END;
