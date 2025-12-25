DROP INDEX IF EXISTS idx_forums_display_order;
DROP INDEX IF EXISTS idx_forums_parent_id;
ALTER TABLE forums DROP COLUMN IF EXISTS display_order;
ALTER TABLE forums DROP COLUMN IF EXISTS parent_id;
