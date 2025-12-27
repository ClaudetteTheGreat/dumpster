-- Revert user follows/followers system

-- Drop trigger
DROP TRIGGER IF EXISTS trigger_update_follow_counts ON user_follows;

-- Drop function
DROP FUNCTION IF EXISTS update_follow_counts();

-- Drop indexes
DROP INDEX IF EXISTS idx_user_follows_following;
DROP INDEX IF EXISTS idx_user_follows_follower;

-- Drop user_follows table
DROP TABLE IF EXISTS user_follows;

-- Remove count columns from users
ALTER TABLE users DROP COLUMN IF EXISTS following_count;
ALTER TABLE users DROP COLUMN IF EXISTS follower_count;
