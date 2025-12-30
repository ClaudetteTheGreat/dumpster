-- Remove thread_template column from forums table
ALTER TABLE forums DROP COLUMN IF EXISTS thread_template;
