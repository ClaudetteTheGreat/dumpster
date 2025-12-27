-- Profile wall posts: allow users to post on each other's profiles

-- Add privacy setting to users table for profile posts
ALTER TABLE users ADD COLUMN IF NOT EXISTS allow_profile_posts BOOLEAN NOT NULL DEFAULT TRUE;

-- Create profile_posts table
CREATE TABLE IF NOT EXISTS profile_posts (
    id SERIAL PRIMARY KEY,
    -- The user whose profile this is posted on
    profile_user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The user who wrote the post (NULL if author was deleted)
    author_id INT REFERENCES users(id) ON DELETE SET NULL,
    -- Link to UGC content
    ugc_id INT NOT NULL REFERENCES ugc(id) ON DELETE CASCADE,
    -- When the post was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fetching posts on a profile
CREATE INDEX idx_profile_posts_profile_user ON profile_posts(profile_user_id, created_at DESC);

-- Index for fetching posts by an author
CREATE INDEX idx_profile_posts_author ON profile_posts(author_id);
