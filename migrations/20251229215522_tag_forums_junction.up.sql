-- Create junction table for tags to forums (many-to-many)
CREATE TABLE tag_forums (
    id SERIAL PRIMARY KEY,
    tag_id INT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    forum_id INT NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    UNIQUE(tag_id, forum_id)
);

CREATE INDEX idx_tag_forums_tag_id ON tag_forums(tag_id);
CREATE INDEX idx_tag_forums_forum_id ON tag_forums(forum_id);

-- Add is_global column to tags (global tags are available in all forums)
ALTER TABLE tags ADD COLUMN is_global BOOLEAN NOT NULL DEFAULT FALSE;

-- Migrate existing forum-specific tags to junction table
INSERT INTO tag_forums (tag_id, forum_id)
SELECT id, forum_id FROM tags WHERE forum_id IS NOT NULL;

-- Mark tags with NULL forum_id as global
UPDATE tags SET is_global = TRUE WHERE forum_id IS NULL;

-- Drop the old forum_id column and its constraint
ALTER TABLE tags DROP COLUMN forum_id;

COMMENT ON TABLE tag_forums IS 'Junction table linking tags to specific forums';
COMMENT ON COLUMN tags.is_global IS 'If true, tag is available in all forums';
