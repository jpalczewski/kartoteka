-- Backfill checklist feature for all lists that have checkbox behavior.
-- Lists of type checklist, shopping, or habits/schedule need the checklist feature.
-- json_patch merges {"checklist": {}} into the existing features object.
UPDATE lists
SET features = json_patch(features, '{"checklist": {}}')
WHERE list_type IN ('checklist', 'shopping', 'habits', 'schedule')
  AND json_extract(features, '$.checklist') IS NULL;
