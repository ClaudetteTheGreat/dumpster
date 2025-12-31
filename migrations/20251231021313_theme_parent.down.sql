-- Remove parent theme support
DROP INDEX IF EXISTS idx_themes_parent_id;
ALTER TABLE themes DROP COLUMN IF EXISTS parent_id;
