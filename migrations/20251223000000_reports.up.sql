-- Reports table for user-submitted content reports
CREATE TABLE reports (
    id SERIAL PRIMARY KEY,

    -- Reporter info
    reporter_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Reported content (polymorphic - can be post, thread, user, etc.)
    content_type VARCHAR(50) NOT NULL,  -- 'post', 'thread', 'user', 'message'
    content_id INT NOT NULL,

    -- Report details
    reason VARCHAR(100) NOT NULL,  -- 'spam', 'harassment', 'illegal', 'other'
    details TEXT,  -- Optional additional details from reporter

    -- Status tracking
    status VARCHAR(20) NOT NULL DEFAULT 'open',  -- 'open', 'reviewed', 'resolved', 'dismissed'

    -- Moderation
    moderator_id INT REFERENCES users(id) ON DELETE SET NULL,
    moderator_notes TEXT,
    resolved_at TIMESTAMP,

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_reports_status ON reports(status);
CREATE INDEX idx_reports_content ON reports(content_type, content_id);
CREATE INDEX idx_reports_reporter ON reports(reporter_id);
CREATE INDEX idx_reports_created ON reports(created_at DESC);

-- Prevent duplicate reports from same user on same content
CREATE UNIQUE INDEX idx_reports_unique_pending ON reports(reporter_id, content_type, content_id)
    WHERE status IN ('open', 'reviewed');

-- Report reasons reference table (optional, for UI dropdown)
CREATE TABLE report_reasons (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    label VARCHAR(100) NOT NULL,
    description TEXT,
    display_order INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- Insert default report reasons
INSERT INTO report_reasons (name, label, description, display_order) VALUES
    ('spam', 'Spam', 'Unsolicited advertising or repeated content', 1),
    ('harassment', 'Harassment', 'Targeted harassment or bullying', 2),
    ('hate_speech', 'Hate Speech', 'Content promoting hatred against protected groups', 3),
    ('illegal', 'Illegal Content', 'Content that violates laws', 4),
    ('misinformation', 'Misinformation', 'False or misleading information', 5),
    ('off_topic', 'Off Topic', 'Content not relevant to the discussion', 6),
    ('other', 'Other', 'Other reason (please specify in details)', 7);
