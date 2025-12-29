-- Add tag settings to forums table
ALTER TABLE forums
ADD COLUMN tags_enabled BOOLEAN NOT NULL DEFAULT TRUE,
ADD COLUMN restrict_tags BOOLEAN NOT NULL DEFAULT FALSE;

-- tags_enabled: Whether users can add tags to threads in this forum
-- restrict_tags: If true, users can only use tags that already exist (admin-created)
--                If false, users can create new tags on-the-fly

COMMENT ON COLUMN forums.tags_enabled IS 'Whether tags are allowed on threads in this forum';
COMMENT ON COLUMN forums.restrict_tags IS 'If true, only existing/admin-created tags can be used';
