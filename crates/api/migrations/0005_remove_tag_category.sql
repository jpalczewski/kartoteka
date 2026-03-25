-- Remove tag categories — tags are now just tags, organized by hierarchy
-- Drop index first (it references the category column)
DROP INDEX IF EXISTS idx_tags_user_cat;
ALTER TABLE tags DROP COLUMN category;
