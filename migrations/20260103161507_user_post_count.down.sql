-- Remove denormalized post_count column
DROP INDEX IF EXISTS idx_users_post_count;
ALTER TABLE users DROP COLUMN IF EXISTS post_count;
