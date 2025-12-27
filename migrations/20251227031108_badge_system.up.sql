-- Badge System Migration
-- Adds badges and user_badges tables for an achievement/badge system

-- Create badge condition type enum
DO $$ BEGIN
    CREATE TYPE badge_condition_type AS ENUM (
        'manual',       -- Manually awarded by staff
        'post_count',   -- Awarded when user reaches X posts
        'thread_count', -- Awarded when user creates X threads
        'time_member',  -- Awarded after X days as member
        'reputation'    -- Awarded when user reaches X reputation
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- Badges table
CREATE TABLE IF NOT EXISTS badges (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    icon VARCHAR(100) NOT NULL DEFAULT '🏆',  -- Emoji or icon class
    color VARCHAR(20) DEFAULT '#6366f1',       -- Badge color (hex)
    condition_type badge_condition_type NOT NULL DEFAULT 'manual',
    condition_value INT,                        -- Value for automatic conditions
    display_order INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User badges junction table
CREATE TABLE IF NOT EXISTS user_badges (
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    badge_id INT NOT NULL REFERENCES badges(id) ON DELETE CASCADE,
    awarded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    awarded_by INT REFERENCES users(id) ON DELETE SET NULL,  -- NULL for automatic awards
    PRIMARY KEY (user_id, badge_id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_badges_slug ON badges(slug);
CREATE INDEX IF NOT EXISTS idx_badges_condition ON badges(condition_type, condition_value) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_badges_display_order ON badges(display_order) WHERE is_active = TRUE;
CREATE INDEX IF NOT EXISTS idx_user_badges_user ON user_badges(user_id);
CREATE INDEX IF NOT EXISTS idx_user_badges_badge ON user_badges(badge_id);
CREATE INDEX IF NOT EXISTS idx_user_badges_awarded ON user_badges(awarded_at DESC);

-- Insert default badges
INSERT INTO badges (name, slug, description, icon, color, condition_type, condition_value, display_order) VALUES
    ('Newcomer', 'newcomer', 'Welcome to the community!', '👋', '#22c55e', 'manual', NULL, 1),
    ('First Post', 'first-post', 'Made their first post', '✍️', '#3b82f6', 'post_count', 1, 2),
    ('Contributor', 'contributor', 'Reached 100 posts', '💬', '#8b5cf6', 'post_count', 100, 3),
    ('Prolific', 'prolific', 'Reached 1,000 posts', '📚', '#f59e0b', 'post_count', 1000, 4),
    ('Thread Starter', 'thread-starter', 'Created 10 threads', '🧵', '#06b6d4', 'thread_count', 10, 5),
    ('One Month', 'one-month', 'Member for 30 days', '📅', '#64748b', 'time_member', 30, 6),
    ('One Year', 'one-year', 'Member for one year', '🎂', '#ec4899', 'time_member', 365, 7),
    ('Veteran', 'veteran', 'Member for 3 years', '🎖️', '#eab308', 'time_member', 1095, 8),
    ('Well Liked', 'well-liked', 'Reached 100 reputation', '❤️', '#ef4444', 'reputation', 100, 9),
    ('Beloved', 'beloved', 'Reached 1,000 reputation', '💖', '#f472b6', 'reputation', 1000, 10)
ON CONFLICT (slug) DO NOTHING;

-- Add badge management permission
INSERT INTO permissions (label, category_id, sort)
SELECT 'admin.badges.manage', id, 50
FROM permission_categories WHERE label = 'Administration'
ON CONFLICT DO NOTHING;

-- Grant badge management to administrators
INSERT INTO permission_values (collection_id, permission_id, value)
SELECT pc.id, p.id, 'yes'
FROM permission_collections pc
JOIN groups g ON pc.group_id = g.id
CROSS JOIN permissions p
WHERE g.label = 'Administrators' AND p.label = 'admin.badges.manage'
ON CONFLICT (collection_id, permission_id) DO NOTHING;
