-- ── Backfill sublists with their parent list's type and features (issue #341) ──
-- Before this release a sublist created without an explicit type or feature list went
-- through `resolve_list_type(None, None)` and was stored as a plain checklist:
-- `list_type = 'checklist'` with the checklist default feature set `{"checklist": {}}`
-- (features are a JSON object keyed by feature name, see migrations 004 and 006).
-- New sublists now inherit the parent's setup (domain::lists::create), so existing
-- default sublists are backfilled here. Nothing else is rewritten: a sublist the user
-- configured (another type, or any feature set that differs from the default) is left
-- untouched, and so is `last_opened_at` (written only by touch_last_opened).
--
-- Multi-level: the recursive walk starts at every default sublist and climbs through
-- default ancestors — those are being fixed by this same statement, so they are
-- transparent as a source — until it reaches the nearest *configured* ancestor, which
-- becomes the source of the copied type/features. A single recursive CTE is used rather
-- than repeated top-down UPDATEs so the outcome does not depend on update order or on
-- how deep the nesting goes; the depth bound only guards against a malformed (cyclic)
-- parent chain (a cycle never reaches a configured ancestor and is left alone).
-- The CTE is materialised (window function in `source`) before the UPDATE runs, so the
-- rows this statement writes cannot feed back into the walk.
WITH RECURSIVE walk(
    list_id,
    ancestor_id,
    ancestor_type,
    ancestor_features,
    ancestor_configured,
    depth
) AS (
    SELECT l.id,
           p.id,
           p.list_type,
           p.features,
           -- 1 = configured ancestor, its setup is copied; 0 = another creation default
           -- that the walk has to look through.
           CASE
               WHEN p.list_type = 'checklist'
                AND json(p.features) = json('{"checklist":{}}')
               THEN 0
               ELSE 1
           END,
           1
    FROM lists l
    JOIN lists p ON p.id = l.parent_list_id
    WHERE l.parent_list_id IS NOT NULL
      AND l.list_type = 'checklist'
      AND json(l.features) = json('{"checklist":{}}')
    UNION ALL
    -- Climb one level: the parent of the current ancestor.
    SELECT w.list_id,
           p.id,
           p.list_type,
           p.features,
           CASE
               WHEN p.list_type = 'checklist'
                AND json(p.features) = json('{"checklist":{}}')
               THEN 0
               ELSE 1
           END,
           w.depth + 1
    FROM walk w
    JOIN lists cur ON cur.id = w.ancestor_id
    JOIN lists p ON p.id = cur.parent_list_id
    WHERE w.ancestor_configured = 0
      AND w.depth < 64
),
source(list_id, parent_list_type, parent_features, depth_rank) AS (
    SELECT w.list_id,
           w.ancestor_type,
           w.ancestor_features,
           ROW_NUMBER() OVER (PARTITION BY w.list_id ORDER BY w.depth)
    FROM walk w
    WHERE w.ancestor_configured = 1
)
UPDATE lists
SET list_type = (
        SELECT src.parent_list_type
        FROM source src
        WHERE src.list_id = lists.id AND src.depth_rank = 1
    ),
    features = (
        SELECT src.parent_features
        FROM source src
        WHERE src.list_id = lists.id AND src.depth_rank = 1
    ),
    -- Every list write path except set_features bumps updated_at; this changes list_type too.
    updated_at = datetime('now')
WHERE parent_list_id IS NOT NULL
  AND list_type = 'checklist'
  AND json(features) = json('{"checklist":{}}')
  AND EXISTS (
      SELECT 1 FROM source src WHERE src.list_id = lists.id AND src.depth_rank = 1
  );
