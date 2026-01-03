-- Add creator_id to conversations table
ALTER TABLE conversations ADD COLUMN creator_id INT REFERENCES users(id) ON DELETE SET NULL;

-- Create index for efficient lookup
CREATE INDEX idx_conversations_creator ON conversations(creator_id);

-- Backfill creator_id from first participant (by joined_at) for existing conversations
UPDATE conversations c
SET creator_id = (
    SELECT cp.user_id
    FROM conversation_participants cp
    WHERE cp.conversation_id = c.id
    ORDER BY cp.joined_at ASC
    LIMIT 1
);
