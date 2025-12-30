-- First post approval system
-- Requires moderator approval for first posts from new accounts

-- Moderation status enum for posts
CREATE TYPE post_moderation_status AS ENUM ('pending', 'approved', 'rejected');

-- Add moderation status to posts
ALTER TABLE posts ADD COLUMN moderation_status post_moderation_status NOT NULL DEFAULT 'approved';
ALTER TABLE posts ADD COLUMN moderated_at TIMESTAMP;
ALTER TABLE posts ADD COLUMN moderated_by INT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE posts ADD COLUMN rejection_reason TEXT;

-- Add first_post_approved flag to users
-- TRUE = user has had at least one post approved (or feature was disabled when they first posted)
-- FALSE = user's first post still needs approval
ALTER TABLE users ADD COLUMN first_post_approved BOOLEAN NOT NULL DEFAULT FALSE;

-- Set first_post_approved = TRUE for existing users who have any posts
UPDATE users SET first_post_approved = TRUE
WHERE id IN (SELECT DISTINCT user_id FROM posts WHERE user_id IS NOT NULL);

-- Index for finding pending posts efficiently
CREATE INDEX idx_posts_moderation_status ON posts(moderation_status) WHERE moderation_status = 'pending';
CREATE INDEX idx_posts_moderated_by ON posts(moderated_by) WHERE moderated_by IS NOT NULL;

-- Index for finding users who need first post approval
CREATE INDEX idx_users_first_post_pending ON users(first_post_approved) WHERE first_post_approved = FALSE;

-- Add setting for require_first_post_approval
INSERT INTO settings (key, value, value_type, description, category, is_public) VALUES
('require_first_post_approval', 'false', 'bool', 'Require moderator approval for first posts from new accounts', 'moderation', FALSE)
ON CONFLICT (key) DO NOTHING;
