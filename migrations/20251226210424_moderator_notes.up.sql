-- Create moderator notes table
CREATE TABLE moderator_notes (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    author_id INT REFERENCES users(id) ON DELETE SET NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Index for looking up notes by user
CREATE INDEX idx_moderator_notes_user_id ON moderator_notes(user_id);
CREATE INDEX idx_moderator_notes_created_at ON moderator_notes(created_at DESC);

-- Add permission for viewing/managing moderator notes
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (39, 2, 'moderate.notes.view', 60),
    (40, 2, 'moderate.notes.manage', 61)
ON CONFLICT (id) DO NOTHING;

-- Grant permissions to Moderators and Administrators
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (39, 3, 'yes'),  -- Moderators can view notes
    (40, 3, 'yes'),  -- Moderators can manage notes
    (39, 4, 'yes'),  -- Administrators can view notes
    (40, 4, 'yes')   -- Administrators can manage notes
ON CONFLICT (permission_id, collection_id) DO NOTHING;
