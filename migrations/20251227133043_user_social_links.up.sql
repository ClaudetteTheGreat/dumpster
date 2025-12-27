-- Social media links for user profiles
-- Allows users to add links to their social media accounts

-- Enum for supported platforms (extendable)
CREATE TYPE social_platform AS ENUM (
    'twitter',
    'discord',
    'github',
    'youtube',
    'twitch',
    'steam',
    'telegram',
    'reddit',
    'instagram',
    'facebook',
    'linkedin',
    'tiktok',
    'website',
    'other'
);

-- User social links table
CREATE TABLE IF NOT EXISTS user_social_links (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    platform social_platform NOT NULL,
    username VARCHAR(255) NOT NULL,  -- Username/handle on the platform
    url VARCHAR(500),                 -- Full URL (optional, can be auto-generated)
    display_order INT NOT NULL DEFAULT 0,
    is_visible BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Each user can only have one link per platform
    UNIQUE(user_id, platform)
);

-- Index for fast lookup by user
CREATE INDEX idx_user_social_links_user_id ON user_social_links(user_id);

-- Index for ordering
CREATE INDEX idx_user_social_links_display_order ON user_social_links(user_id, display_order);
