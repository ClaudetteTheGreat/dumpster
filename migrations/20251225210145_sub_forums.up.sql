-- Add sub-forum support to forums table
-- parent_id references another forum (NULL for top-level forums)
-- display_order controls ordering within the parent

ALTER TABLE forums ADD COLUMN parent_id INT REFERENCES forums(id) ON DELETE CASCADE;
ALTER TABLE forums ADD COLUMN display_order INT NOT NULL DEFAULT 0;

-- Index for efficient child forum lookups
CREATE INDEX idx_forums_parent_id ON forums(parent_id);

-- Index for ordering
CREATE INDEX idx_forums_display_order ON forums(display_order);
