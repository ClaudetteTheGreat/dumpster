-- Add attachment fields for forum icons (images/SVGs)
ALTER TABLE forums
ADD COLUMN icon_attachment_id INT REFERENCES attachments(id) ON DELETE SET NULL,
ADD COLUMN icon_new_attachment_id INT REFERENCES attachments(id) ON DELETE SET NULL;

-- Add indexes for the foreign keys
CREATE INDEX idx_forums_icon_attachment ON forums(icon_attachment_id);
CREATE INDEX idx_forums_icon_new_attachment ON forums(icon_new_attachment_id);

COMMENT ON COLUMN forums.icon_attachment_id IS 'Custom image/SVG for default forum icon';
COMMENT ON COLUMN forums.icon_new_attachment_id IS 'Custom image/SVG for forum icon when new content exists';
