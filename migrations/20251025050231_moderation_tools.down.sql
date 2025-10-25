-- Rollback moderation tools

-- Drop indexes
DROP INDEX IF EXISTS idx_threads_locked;
DROP INDEX IF EXISTS idx_threads_pinned;
DROP INDEX IF EXISTS idx_mod_log_target;
DROP INDEX IF EXISTS idx_mod_log_created;
DROP INDEX IF EXISTS idx_mod_log_moderator;
DROP INDEX IF EXISTS idx_user_bans_expires;
DROP INDEX IF EXISTS idx_user_bans_user;

-- Drop thread moderation columns
ALTER TABLE threads DROP COLUMN IF EXISTS is_announcement;
ALTER TABLE threads DROP COLUMN IF EXISTS is_pinned;
ALTER TABLE threads DROP COLUMN IF EXISTS is_locked;

-- Drop tables
DROP TABLE IF EXISTS mod_log;
DROP TABLE IF EXISTS user_bans;
