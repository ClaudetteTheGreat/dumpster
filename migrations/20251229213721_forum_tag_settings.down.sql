-- Remove tag settings from forums table
ALTER TABLE forums
DROP COLUMN tags_enabled,
DROP COLUMN restrict_tags;
