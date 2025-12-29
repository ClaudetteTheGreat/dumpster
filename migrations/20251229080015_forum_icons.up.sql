-- Add icon fields to forums table
ALTER TABLE forums
ADD COLUMN icon VARCHAR(32) NOT NULL DEFAULT '📁',
ADD COLUMN icon_new VARCHAR(32) NOT NULL DEFAULT '📂';

-- Add index for common queries
COMMENT ON COLUMN forums.icon IS 'Default icon/emoji for the forum';
COMMENT ON COLUMN forums.icon_new IS 'Icon shown when forum has unread content';
