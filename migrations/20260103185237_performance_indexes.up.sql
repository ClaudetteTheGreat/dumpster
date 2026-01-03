-- Performance optimization indexes

-- Composite index for fetching posts by thread in position order
-- Used by thread page queries that ORDER BY position
CREATE INDEX IF NOT EXISTS idx_posts_thread_position ON posts(thread_id, position);

-- Composite index for unread conversation count query
-- Filters by user_id and is_archived
CREATE INDEX IF NOT EXISTS idx_conv_participants_user_archived
    ON conversation_participants(user_id, is_archived);

-- Index for conversations updated_at (used in unread count join)
CREATE INDEX IF NOT EXISTS idx_conversations_updated ON conversations(updated_at DESC);
