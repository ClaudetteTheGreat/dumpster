-- User follows/followers system

-- Create the user_follows table
CREATE TABLE IF NOT EXISTS user_follows (
    id SERIAL PRIMARY KEY,
    -- The user who is following
    follower_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The user being followed
    following_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- When the follow relationship was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Prevent duplicate follows and self-follows
    CONSTRAINT unique_follow UNIQUE (follower_id, following_id),
    CONSTRAINT no_self_follow CHECK (follower_id != following_id)
);

-- Index for finding who a user follows
CREATE INDEX idx_user_follows_follower ON user_follows(follower_id, created_at DESC);

-- Index for finding who follows a user
CREATE INDEX idx_user_follows_following ON user_follows(following_id, created_at DESC);

-- Add follower/following counts to users table for efficient display
ALTER TABLE users ADD COLUMN IF NOT EXISTS follower_count INT NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS following_count INT NOT NULL DEFAULT 0;

-- Trigger function to update follower/following counts
CREATE OR REPLACE FUNCTION update_follow_counts()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        -- Increment following_count for the follower
        UPDATE users SET following_count = following_count + 1
        WHERE id = NEW.follower_id;
        -- Increment follower_count for the person being followed
        UPDATE users SET follower_count = follower_count + 1
        WHERE id = NEW.following_id;
    ELSIF TG_OP = 'DELETE' THEN
        -- Decrement following_count for the follower
        UPDATE users SET following_count = GREATEST(0, following_count - 1)
        WHERE id = OLD.follower_id;
        -- Decrement follower_count for the person being followed
        UPDATE users SET follower_count = GREATEST(0, follower_count - 1)
        WHERE id = OLD.following_id;
    END IF;
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Create trigger for follow count updates
CREATE TRIGGER trigger_update_follow_counts
AFTER INSERT OR DELETE ON user_follows
FOR EACH ROW EXECUTE FUNCTION update_follow_counts();
