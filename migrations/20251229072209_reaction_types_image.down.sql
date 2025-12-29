-- Remove attachment_id from reaction_types
DROP INDEX IF EXISTS idx_reaction_types_attachment;
ALTER TABLE reaction_types DROP COLUMN IF EXISTS attachment_id;
