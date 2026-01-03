-- Remove creator_id from conversations table
DROP INDEX IF EXISTS idx_conversations_creator;
ALTER TABLE conversations DROP COLUMN IF EXISTS creator_id;
