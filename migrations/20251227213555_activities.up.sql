-- Activity Feed System

-- Activity types enum
CREATE TYPE activity_type AS ENUM (
    'post_created',
    'thread_created',
    'profile_post_created',
    'user_followed',
    'reaction_given'
);

-- Main activities table
CREATE TABLE activities (
    id SERIAL PRIMARY KEY,
    activity_type activity_type NOT NULL,
    -- The user who performed the action
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Polymorphic target references (nullable based on activity type)
    target_user_id INT REFERENCES users(id) ON DELETE CASCADE,      -- For follows, profile posts
    target_thread_id INT REFERENCES threads(id) ON DELETE CASCADE,  -- For threads, posts
    target_post_id INT REFERENCES posts(id) ON DELETE CASCADE,      -- For posts, reactions
    target_forum_id INT REFERENCES forums(id) ON DELETE SET NULL,   -- For permission checks

    -- Denormalized data for fast display (avoids expensive joins)
    title TEXT,            -- Thread title, target user name, etc.
    content_preview TEXT,  -- First ~200 chars of content
    reaction_emoji TEXT    -- For reaction activities
);

-- Index for global feed (all activities sorted by time)
CREATE INDEX idx_activities_global ON activities(created_at DESC, id DESC);

-- Index for user profile feed (specific user's activities)
CREATE INDEX idx_activities_user ON activities(user_id, created_at DESC, id DESC);

-- Index for personal feed lookups (activities by followed users)
-- Used with JOIN on user_follows to find activities from people you follow
CREATE INDEX idx_activities_actor_time ON activities(user_id, created_at DESC);

-- Index for cascade deletes when target content is removed
CREATE INDEX idx_activities_target_thread ON activities(target_thread_id) WHERE target_thread_id IS NOT NULL;
CREATE INDEX idx_activities_target_post ON activities(target_post_id) WHERE target_post_id IS NOT NULL;
CREATE INDEX idx_activities_target_user ON activities(target_user_id) WHERE target_user_id IS NOT NULL;
