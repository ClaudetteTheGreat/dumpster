-- Remove attachment fields from forums table
DROP INDEX IF EXISTS idx_forums_icon_attachment;
DROP INDEX IF EXISTS idx_forums_icon_new_attachment;
ALTER TABLE forums
DROP COLUMN IF EXISTS icon_attachment_id,
DROP COLUMN IF EXISTS icon_new_attachment_id;
