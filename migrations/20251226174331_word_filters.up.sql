-- Word filter system for content moderation
-- Supports word replacement (exchange), blocking, and flagging

-- Action types:
-- 'replace' - Replace the matched text with replacement (word exchange)
-- 'block' - Reject the content entirely
-- 'flag' - Allow but flag for moderator review

CREATE TABLE word_filters (
    id SERIAL PRIMARY KEY,
    pattern VARCHAR(255) NOT NULL,
    replacement VARCHAR(255),  -- Used when action = 'replace'
    is_regex BOOLEAN DEFAULT FALSE,
    is_case_sensitive BOOLEAN DEFAULT FALSE,
    is_whole_word BOOLEAN DEFAULT TRUE,  -- Match whole words only
    action VARCHAR(20) DEFAULT 'replace' CHECK (action IN ('replace', 'block', 'flag')),
    is_enabled BOOLEAN DEFAULT TRUE,
    created_by INT REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    notes TEXT  -- Admin notes about why this filter exists
);

-- Index for efficient lookups
CREATE INDEX idx_word_filters_enabled ON word_filters(is_enabled) WHERE is_enabled = TRUE;
CREATE INDEX idx_word_filters_action ON word_filters(action);

-- Add word filter permissions
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (36, 3, 'admin.word_filters.view', 80),    -- View word filters
    (37, 3, 'admin.word_filters.manage', 81)   -- Create/edit/delete word filters
ON CONFLICT (id) DO NOTHING;

-- Grant permissions to Administrators group (collection_id 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (36, 4, 'yes'),  -- admin.word_filters.view
    (37, 4, 'yes')   -- admin.word_filters.manage
ON CONFLICT DO NOTHING;
