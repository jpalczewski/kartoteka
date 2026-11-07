-- ── Change log: live updates over SSE (issue #356) ───────────────────────────
-- Every write path (server fns, REST, MCP batch transactions, future jobs) has
-- to be visible to the live-update pump without bookkeeping calls in each of
-- them, so the triggers below record the changes straight from SQLite. The
-- server reads rows with id > last (crates/db/src/changes.rs) and is woken by
-- the per-connection commit hook installed by `create_pool_with_signal`.
--
-- `topic` values are exactly the kartoteka_shared::ChangeTopic serde names:
--   lists, items, tags, containers, settings, comments, relations,
--   time_entries, templates, locations
--
-- Invariants:
--   * user_id is NOT NULL, so a trigger whose owner cannot be resolved skips the
--     row instead of failing: every insert is `INSERT ... SELECT ... WHERE
--     user_id IS NOT NULL`. A child row firing during a cascade delete resolves
--     to NULL because its parent is already gone; the parent's own event covers
--     that delete.
--   * entity_id is the changed row's id. Link tables (composite primary key) and
--     user_settings (keyed by user_id + key) insert NULL.
--   * Triggers are named <table>_change_<insert|update|delete>.
--
-- 018 is reserved by another PR, hence the 019 prefix.

CREATE TABLE IF NOT EXISTS change_log (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,  -- monotonic, used as SSE event id
    user_id   TEXT NOT NULL,
    topic     TEXT NOT NULL,                      -- ChangeTopic snake_case value
    entity_id TEXT,                               -- id of the changed row (NULL for composite-key rows)
    at        TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE INDEX IF NOT EXISTS idx_change_log_user_id ON change_log(user_id, id);
CREATE INDEX IF NOT EXISTS idx_change_log_at ON change_log(at);

-- ── lists ────────────────────────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS lists_change_insert AFTER INSERT ON lists
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'lists', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

-- Skips a pure touch_last_opened_at write: opening a list is a read of the
-- user's data and must not wake the live-update pump. Every other column is
-- compared, so a real change fires even when it arrives together with
-- last_opened_at (and with a bumped updated_at, which is why that column is not
-- compared either).
-- !!! Columns added to lists must be added here, or an UPDATE that touches only
-- !!! the new column stays silent.
CREATE TRIGGER IF NOT EXISTS lists_change_update AFTER UPDATE ON lists
WHEN OLD.user_id        IS NOT NEW.user_id
  OR OLD.name           IS NOT NEW.name
  OR OLD.icon           IS NOT NEW.icon
  OR OLD.description    IS NOT NEW.description
  OR OLD.list_type      IS NOT NEW.list_type
  OR OLD.parent_list_id IS NOT NEW.parent_list_id
  OR OLD.position       IS NOT NEW.position
  OR OLD.archived       IS NOT NEW.archived
  OR OLD.container_id   IS NOT NEW.container_id
  OR OLD.pinned         IS NOT NEW.pinned
  OR OLD.features       IS NOT NEW.features
  OR OLD.location_id    IS NOT NEW.location_id
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'lists', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS lists_change_delete AFTER DELETE ON lists
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'lists', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── containers ───────────────────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS containers_change_insert AFTER INSERT ON containers
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'containers', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

-- Same last_opened_at exclusion as lists_change_update; keep both column lists
-- in sync and extend them when containers gains a column.
CREATE TRIGGER IF NOT EXISTS containers_change_update AFTER UPDATE ON containers
WHEN OLD.user_id            IS NOT NEW.user_id
  OR OLD.name               IS NOT NEW.name
  OR OLD.icon               IS NOT NEW.icon
  OR OLD.description        IS NOT NEW.description
  OR OLD.status             IS NOT NEW.status
  OR OLD.parent_container_id IS NOT NEW.parent_container_id
  OR OLD.position           IS NOT NEW.position
  OR OLD.pinned             IS NOT NEW.pinned
  OR OLD.archived           IS NOT NEW.archived
  OR OLD.location_id        IS NOT NEW.location_id
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'containers', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS containers_change_delete AFTER DELETE ON containers
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'containers', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── items (owner comes from the list) ────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS items_change_insert AFTER INSERT ON items
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT l.user_id, 'items', NEW.id
    FROM lists l
    WHERE l.id = NEW.list_id AND l.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS items_change_update AFTER UPDATE ON items
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT l.user_id, 'items', NEW.id
    FROM lists l
    WHERE l.id = NEW.list_id AND l.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS items_change_delete AFTER DELETE ON items
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT l.user_id, 'items', OLD.id
    FROM lists l
    WHERE l.id = OLD.list_id AND l.user_id IS NOT NULL;
END;

-- ── tags ─────────────────────────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS tags_change_insert AFTER INSERT ON tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'tags', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS tags_change_update AFTER UPDATE ON tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'tags', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS tags_change_delete AFTER DELETE ON tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'tags', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── item_tags (two topics: items + tags) ─────────────────────────────────────
-- The owner is resolved from the item's list, falling back to the tag: during a
-- cascade delete the item (and its list) may already be gone while the tag stays.
CREATE TRIGGER IF NOT EXISTS item_tags_change_insert AFTER INSERT ON item_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT l.user_id FROM items i JOIN lists l ON l.id = i.list_id
              WHERE i.id = NEW.item_id),
            (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'items' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS item_tags_change_update AFTER UPDATE ON item_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT l.user_id FROM items i JOIN lists l ON l.id = i.list_id
              WHERE i.id = NEW.item_id),
            (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'items' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS item_tags_change_delete AFTER DELETE ON item_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT l.user_id FROM items i JOIN lists l ON l.id = i.list_id
              WHERE i.id = OLD.item_id),
            (SELECT g.user_id FROM tags g WHERE g.id = OLD.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'items' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

-- ── list_tags (two topics: lists + tags) ─────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS list_tags_change_insert AFTER INSERT ON list_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT l.user_id FROM lists l WHERE l.id = NEW.list_id),
            (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'lists' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS list_tags_change_update AFTER UPDATE ON list_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT l.user_id FROM lists l WHERE l.id = NEW.list_id),
            (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'lists' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS list_tags_change_delete AFTER DELETE ON list_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT l.user_id FROM lists l WHERE l.id = OLD.list_id),
            (SELECT g.user_id FROM tags g WHERE g.id = OLD.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'lists' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

-- ── container_tags (two topics: containers + tags) ───────────────────────────
CREATE TRIGGER IF NOT EXISTS container_tags_change_insert AFTER INSERT ON container_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT c.user_id FROM containers c WHERE c.id = NEW.container_id),
            (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'containers' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS container_tags_change_update AFTER UPDATE ON container_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT c.user_id FROM containers c WHERE c.id = NEW.container_id),
            (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'containers' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS container_tags_change_delete AFTER DELETE ON container_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT res.user_id, t.topic, NULL
    FROM (
        SELECT COALESCE(
            (SELECT c.user_id FROM containers c WHERE c.id = OLD.container_id),
            (SELECT g.user_id FROM tags g WHERE g.id = OLD.tag_id)
        ) AS user_id
    ) AS res,
    (SELECT 'containers' AS topic UNION ALL SELECT 'tags') AS t
    WHERE res.user_id IS NOT NULL;
END;

-- ── templates ────────────────────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS templates_change_insert AFTER INSERT ON templates
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'templates', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS templates_change_update AFTER UPDATE ON templates
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'templates', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS templates_change_delete AFTER DELETE ON templates
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'templates', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── template_items (owner comes from the template) ───────────────────────────
CREATE TRIGGER IF NOT EXISTS template_items_change_insert AFTER INSERT ON template_items
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT t.user_id, 'templates', NEW.id
    FROM templates t
    WHERE t.id = NEW.template_id AND t.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS template_items_change_update AFTER UPDATE ON template_items
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT t.user_id, 'templates', NEW.id
    FROM templates t
    WHERE t.id = NEW.template_id AND t.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS template_items_change_delete AFTER DELETE ON template_items
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT t.user_id, 'templates', OLD.id
    FROM templates t
    WHERE t.id = OLD.template_id AND t.user_id IS NOT NULL;
END;

-- ── template_tags (owner from the template, falling back to the tag) ─────────
CREATE TRIGGER IF NOT EXISTS template_tags_change_insert AFTER INSERT ON template_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT COALESCE(
        (SELECT t.user_id FROM templates t WHERE t.id = NEW.template_id),
        (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
    ), 'templates', NULL
    WHERE COALESCE(
        (SELECT t.user_id FROM templates t WHERE t.id = NEW.template_id),
        (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
    ) IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS template_tags_change_update AFTER UPDATE ON template_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT COALESCE(
        (SELECT t.user_id FROM templates t WHERE t.id = NEW.template_id),
        (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
    ), 'templates', NULL
    WHERE COALESCE(
        (SELECT t.user_id FROM templates t WHERE t.id = NEW.template_id),
        (SELECT g.user_id FROM tags g WHERE g.id = NEW.tag_id)
    ) IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS template_tags_change_delete AFTER DELETE ON template_tags
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT COALESCE(
        (SELECT t.user_id FROM templates t WHERE t.id = OLD.template_id),
        (SELECT g.user_id FROM tags g WHERE g.id = OLD.tag_id)
    ), 'templates', NULL
    WHERE COALESCE(
        (SELECT t.user_id FROM templates t WHERE t.id = OLD.template_id),
        (SELECT g.user_id FROM tags g WHERE g.id = OLD.tag_id)
    ) IS NOT NULL;
END;

-- ── comments ─────────────────────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS comments_change_insert AFTER INSERT ON comments
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'comments', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS comments_change_update AFTER UPDATE ON comments
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'comments', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS comments_change_delete AFTER DELETE ON comments
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'comments', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── entity_relations ─────────────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS entity_relations_change_insert AFTER INSERT ON entity_relations
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'relations', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS entity_relations_change_update AFTER UPDATE ON entity_relations
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'relations', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS entity_relations_change_delete AFTER DELETE ON entity_relations
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'relations', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── time_entries ─────────────────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS time_entries_change_insert AFTER INSERT ON time_entries
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'time_entries', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS time_entries_change_update AFTER UPDATE ON time_entries
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'time_entries', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS time_entries_change_delete AFTER DELETE ON time_entries
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'time_entries', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── locations ────────────────────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS locations_change_insert AFTER INSERT ON locations
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'locations', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS locations_change_update AFTER UPDATE ON locations
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'locations', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS locations_change_delete AFTER DELETE ON locations
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'locations', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── settings: user_settings (composite key) ──────────────────────────────────
CREATE TRIGGER IF NOT EXISTS user_settings_change_insert AFTER INSERT ON user_settings
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'settings', NULL WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS user_settings_change_update AFTER UPDATE ON user_settings
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'settings', NULL WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS user_settings_change_delete AFTER DELETE ON user_settings
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'settings', NULL WHERE OLD.user_id IS NOT NULL;
END;

-- ── settings: calendar_tokens ────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS calendar_tokens_change_insert AFTER INSERT ON calendar_tokens
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'settings', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS calendar_tokens_change_update AFTER UPDATE ON calendar_tokens
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'settings', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS calendar_tokens_change_delete AFTER DELETE ON calendar_tokens
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'settings', OLD.id WHERE OLD.user_id IS NOT NULL;
END;

-- ── settings: personal_tokens ────────────────────────────────────────────────
CREATE TRIGGER IF NOT EXISTS personal_tokens_change_insert AFTER INSERT ON personal_tokens
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'settings', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS personal_tokens_change_update AFTER UPDATE ON personal_tokens
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT NEW.user_id, 'settings', NEW.id WHERE NEW.user_id IS NOT NULL;
END;

CREATE TRIGGER IF NOT EXISTS personal_tokens_change_delete AFTER DELETE ON personal_tokens
BEGIN
    INSERT INTO change_log (user_id, topic, entity_id)
    SELECT OLD.user_id, 'settings', OLD.id WHERE OLD.user_id IS NOT NULL;
END;
