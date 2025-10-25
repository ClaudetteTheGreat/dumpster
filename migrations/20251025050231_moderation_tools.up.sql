-- Add moderation and administration tools

-- User bans table
CREATE TABLE user_bans (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    banned_by INT REFERENCES users(id) ON DELETE SET NULL,
    reason TEXT NOT NULL,
    expires_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    is_permanent BOOLEAN DEFAULT FALSE
);

CREATE INDEX idx_user_bans_user ON user_bans(user_id);
CREATE INDEX idx_user_bans_expires ON user_bans(expires_at);

-- Moderation actions log
CREATE TABLE mod_log (
    id SERIAL PRIMARY KEY,
    moderator_id INT REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(50) NOT NULL,
    target_id INT NOT NULL,
    reason TEXT,
    metadata JSONB,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_mod_log_moderator ON mod_log(moderator_id);
CREATE INDEX idx_mod_log_created ON mod_log(created_at DESC);
CREATE INDEX idx_mod_log_target ON mod_log(target_type, target_id);

-- Thread moderation columns
ALTER TABLE threads ADD COLUMN is_locked BOOLEAN DEFAULT FALSE;
ALTER TABLE threads ADD COLUMN is_pinned BOOLEAN DEFAULT FALSE;
ALTER TABLE threads ADD COLUMN is_announcement BOOLEAN DEFAULT FALSE;

CREATE INDEX idx_threads_pinned ON threads(is_pinned) WHERE is_pinned = TRUE;
CREATE INDEX idx_threads_locked ON threads(is_locked) WHERE is_locked = TRUE;
