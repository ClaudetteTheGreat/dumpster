-- Revert performance indexes
DROP INDEX IF EXISTS idx_posts_thread_position;
DROP INDEX IF EXISTS idx_conv_participants_user_archived;
DROP INDEX IF EXISTS idx_conversations_updated;
