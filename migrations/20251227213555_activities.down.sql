-- Revert Activity Feed System

-- Drop indexes first
DROP INDEX IF EXISTS idx_activities_target_user;
DROP INDEX IF EXISTS idx_activities_target_post;
DROP INDEX IF EXISTS idx_activities_target_thread;
DROP INDEX IF EXISTS idx_activities_actor_time;
DROP INDEX IF EXISTS idx_activities_user;
DROP INDEX IF EXISTS idx_activities_global;

-- Drop table
DROP TABLE IF EXISTS activities;

-- Drop enum type
DROP TYPE IF EXISTS activity_type;
