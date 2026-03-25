-- Remove tag categories — tags are now just tags, organized by hierarchy
ALTER TABLE tags DROP COLUMN category;
DROP INDEX IF EXISTS idx_tags_user_cat;
