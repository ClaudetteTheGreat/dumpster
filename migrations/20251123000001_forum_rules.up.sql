-- Add rules column to forums table

ALTER TABLE forums ADD COLUMN rules TEXT;

-- Add comment explaining the column
COMMENT ON COLUMN forums.rules IS 'Forum-specific rules displayed at the top of the forum. Supports BBCode formatting.';
