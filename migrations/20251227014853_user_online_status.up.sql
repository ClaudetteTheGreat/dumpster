-- Add online status tracking columns to users table
ALTER TABLE users ADD COLUMN last_activity_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE users ADD COLUMN show_online BOOLEAN NOT NULL DEFAULT TRUE;

-- Create index for efficient online user queries
CREATE INDEX idx_users_last_activity ON users(last_activity_at DESC) WHERE last_activity_at IS NOT NULL;
