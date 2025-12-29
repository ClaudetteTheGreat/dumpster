-- Remove icon fields from forums table
ALTER TABLE forums
DROP COLUMN IF EXISTS icon,
DROP COLUMN IF EXISTS icon_new;
