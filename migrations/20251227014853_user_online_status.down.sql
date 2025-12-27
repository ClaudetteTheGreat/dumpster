-- Remove online status tracking columns
DROP INDEX IF EXISTS idx_users_last_activity;
ALTER TABLE users DROP COLUMN IF EXISTS show_online;
ALTER TABLE users DROP COLUMN IF EXISTS last_activity_at;
