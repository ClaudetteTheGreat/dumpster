-- Add denormalized post_count column to users table for performance
-- This eliminates the need to COUNT(*) posts on every profile load

ALTER TABLE users ADD COLUMN post_count INTEGER NOT NULL DEFAULT 0;

-- Backfill existing counts
UPDATE users SET post_count = (
    SELECT COUNT(*) FROM posts WHERE posts.user_id = users.id
);

-- Create index for efficient sorting by post count
CREATE INDEX idx_users_post_count ON users(post_count);
