-- Remove rules column from forums table

ALTER TABLE forums DROP COLUMN IF EXISTS rules;
