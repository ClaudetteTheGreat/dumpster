-- Revert first post approval system

-- Remove setting
DELETE FROM settings WHERE key = 'require_first_post_approval';

-- Remove indexes
DROP INDEX IF EXISTS idx_users_first_post_pending;
DROP INDEX IF EXISTS idx_posts_moderated_by;
DROP INDEX IF EXISTS idx_posts_moderation_status;

-- Remove column from users
ALTER TABLE users DROP COLUMN IF EXISTS first_post_approved;

-- Remove columns from posts
ALTER TABLE posts DROP COLUMN IF EXISTS rejection_reason;
ALTER TABLE posts DROP COLUMN IF EXISTS moderated_by;
ALTER TABLE posts DROP COLUMN IF EXISTS moderated_at;
ALTER TABLE posts DROP COLUMN IF EXISTS moderation_status;

-- Drop enum
DROP TYPE IF EXISTS post_moderation_status;
