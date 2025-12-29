-- Add attachment_id to reaction_types for custom images
ALTER TABLE reaction_types
ADD COLUMN attachment_id INT REFERENCES attachments(id) ON DELETE SET NULL;

-- Add index for the foreign key
CREATE INDEX idx_reaction_types_attachment ON reaction_types(attachment_id);
