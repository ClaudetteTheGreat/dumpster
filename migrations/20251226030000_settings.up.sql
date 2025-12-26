-- Site settings table
CREATE TABLE settings (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT NOT NULL,
    value_type VARCHAR(20) NOT NULL, -- 'string', 'int', 'bool', 'json'
    description TEXT,
    category VARCHAR(50) NOT NULL DEFAULT 'general',
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_by INT REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_settings_category ON settings(category);

-- Setting history (audit trail)
CREATE TABLE setting_history (
    id SERIAL PRIMARY KEY,
    setting_key VARCHAR(100) NOT NULL,
    old_value TEXT,
    new_value TEXT NOT NULL,
    changed_by INT REFERENCES users(id) ON DELETE SET NULL,
    changed_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_setting_history_key ON setting_history(setting_key);
CREATE INDEX idx_setting_history_changed ON setting_history(changed_at DESC);

-- Feature flags
CREATE TABLE feature_flags (
    key VARCHAR(100) PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    description TEXT,
    rollout_percentage INT NOT NULL DEFAULT 100,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Default settings
INSERT INTO settings (key, value, value_type, description, category, is_public) VALUES
-- General settings
('site_name', 'Ruforo', 'string', 'Name of the forum', 'general', TRUE),
('site_description', 'A forum built in Rust', 'string', 'Site description for meta tags', 'general', TRUE),
('site_url', 'http://localhost:8080', 'string', 'Base URL of the site', 'general', FALSE),
('timezone', 'UTC', 'string', 'Default timezone', 'general', TRUE),

-- Display settings
('posts_per_page', '25', 'int', 'Default posts per page', 'display', TRUE),
('threads_per_page', '20', 'int', 'Threads per page in forum list', 'display', TRUE),

-- Storage settings
('max_upload_size_mb', '10', 'int', 'Maximum file upload size in MB', 'storage', TRUE),
('max_avatar_size_kb', '500', 'int', 'Maximum avatar file size in KB', 'storage', TRUE),

-- Security settings
('session_timeout_minutes', '1440', 'int', 'Session timeout in minutes (default 24h)', 'security', FALSE),
('max_login_attempts', '5', 'int', 'Maximum login attempts before lockout', 'security', FALSE),
('lockout_duration_minutes', '15', 'int', 'Account lockout duration in minutes', 'security', FALSE),
('registration_enabled', 'true', 'bool', 'Allow new user registrations', 'security', TRUE),

-- Feature toggles
('maintenance_mode', 'false', 'bool', 'Put site in maintenance mode', 'features', FALSE),
('chat_enabled', 'true', 'bool', 'Enable real-time chat feature', 'features', TRUE),
('reactions_enabled', 'true', 'bool', 'Enable post reactions', 'features', TRUE),
('polls_enabled', 'true', 'bool', 'Enable thread polls', 'features', TRUE),
('signatures_enabled', 'true', 'bool', 'Show user signatures in posts', 'features', TRUE);

-- Default feature flags
INSERT INTO feature_flags (key, enabled, description) VALUES
('dark_mode', TRUE, 'Enable dark mode theme option'),
('keyboard_shortcuts', TRUE, 'Enable keyboard navigation shortcuts'),
('post_preview', TRUE, 'Enable post preview before submission'),
('draft_autosave', TRUE, 'Auto-save post drafts');
