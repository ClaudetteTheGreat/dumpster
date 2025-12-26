-- User warning system
-- Warnings accumulate points; when threshold is reached, user can be auto-banned

CREATE TABLE user_warnings (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    issued_by INT REFERENCES users(id) ON DELETE SET NULL,
    reason TEXT NOT NULL,
    points INT NOT NULL DEFAULT 1,
    expires_at TIMESTAMP,  -- NULL = permanent warning
    acknowledged_at TIMESTAMP,  -- When user acknowledged the warning
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for looking up warnings by user
CREATE INDEX idx_user_warnings_user_id ON user_warnings(user_id);
-- Index for finding non-permanent warnings (need to check expiry at query time)
CREATE INDEX idx_user_warnings_expires ON user_warnings(expires_at) WHERE expires_at IS NOT NULL;
-- Index for recent warnings
CREATE INDEX idx_user_warnings_created ON user_warnings(created_at DESC);

-- Add warning-related columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS warning_points INT NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_warning_at TIMESTAMP;

-- Add warning permissions
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (41, 2, 'moderate.warnings.view', 62),    -- View user warnings
    (42, 2, 'moderate.warnings.issue', 63),   -- Issue warnings to users
    (43, 2, 'moderate.warnings.delete', 64)   -- Delete/revoke warnings
ON CONFLICT (id) DO NOTHING;

-- Grant permissions to Moderators group (collection_id 3)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (41, 3, 'yes'),  -- moderate.warnings.view
    (42, 3, 'yes'),  -- moderate.warnings.issue
    (43, 3, 'yes')   -- moderate.warnings.delete
ON CONFLICT (permission_id, collection_id) DO NOTHING;

-- Grant permissions to Administrators group (collection_id 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (41, 4, 'yes'),  -- moderate.warnings.view
    (42, 4, 'yes'),  -- moderate.warnings.issue
    (43, 4, 'yes')   -- moderate.warnings.delete
ON CONFLICT (permission_id, collection_id) DO NOTHING;

-- Add warning threshold setting
INSERT INTO settings (key, value, value_type, description, category, is_public) VALUES
    ('warning_threshold', '10', 'int', 'Warning points threshold for auto-ban', 'moderation', FALSE),
    ('warning_ban_duration_days', '7', 'int', 'Days to ban when warning threshold reached (0 = permanent)', 'moderation', FALSE),
    ('warning_expiry_days', '90', 'int', 'Days until warnings expire (0 = never)', 'moderation', FALSE)
ON CONFLICT (key) DO NOTHING;
