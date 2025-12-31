-- Add parent theme support for theme inheritance
ALTER TABLE themes ADD COLUMN parent_id INT REFERENCES themes(id) ON DELETE SET NULL;

-- Create index for parent lookups
CREATE INDEX idx_themes_parent_id ON themes(parent_id) WHERE parent_id IS NOT NULL;
