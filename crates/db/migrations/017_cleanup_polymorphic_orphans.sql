-- ── Polymorphic orphans ──────────────────────────────────────────────────────
-- comments and entity_relations reference their target by (type, id) instead of
-- a foreign key, so deleting an item/list/container left comments and relations
-- pointing at a row that no longer exists. Those stale rows kept showing up in
-- relations/comments listings, so clean them up and keep them cleaned up.

-- (a) Delete the orphans that already exist. Rows with an unknown entity_type
-- are left alone: nothing in the app creates them, but they are not ours to
-- interpret.
DELETE FROM comments
WHERE (entity_type = 'item' AND entity_id NOT IN (SELECT id FROM items))
   OR (entity_type = 'list' AND entity_id NOT IN (SELECT id FROM lists))
   OR (entity_type = 'container' AND entity_id NOT IN (SELECT id FROM containers));

DELETE FROM entity_relations
WHERE (from_type = 'item' AND from_id NOT IN (SELECT id FROM items))
   OR (from_type = 'list' AND from_id NOT IN (SELECT id FROM lists))
   OR (from_type = 'container' AND from_id NOT IN (SELECT id FROM containers))
   OR (to_type = 'item' AND to_id NOT IN (SELECT id FROM items))
   OR (to_type = 'list' AND to_id NOT IN (SELECT id FROM lists))
   OR (to_type = 'container' AND to_id NOT IN (SELECT id FROM containers));

-- (b) Keep the tables clean from now on. A relation can point at the deleted
-- row from either side, so both sides are matched.
CREATE TRIGGER IF NOT EXISTS items_cleanup_polymorphic_orphans
AFTER DELETE ON items
BEGIN
    DELETE FROM comments
    WHERE entity_type = 'item' AND entity_id = old.id;
    DELETE FROM entity_relations
    WHERE (from_type = 'item' AND from_id = old.id)
       OR (to_type = 'item' AND to_id = old.id);
END;

-- Deleting a list cascades to its items (items.list_id ON DELETE CASCADE), which
-- fires the item trigger above for each of them.
CREATE TRIGGER IF NOT EXISTS lists_cleanup_polymorphic_orphans
AFTER DELETE ON lists
BEGIN
    DELETE FROM comments
    WHERE entity_type = 'list' AND entity_id = old.id;
    DELETE FROM entity_relations
    WHERE (from_type = 'list' AND from_id = old.id)
       OR (to_type = 'list' AND to_id = old.id);
END;

CREATE TRIGGER IF NOT EXISTS containers_cleanup_polymorphic_orphans
AFTER DELETE ON containers
BEGIN
    DELETE FROM comments
    WHERE entity_type = 'container' AND entity_id = old.id;
    DELETE FROM entity_relations
    WHERE (from_type = 'container' AND from_id = old.id)
       OR (to_type = 'container' AND to_id = old.id);
END;
