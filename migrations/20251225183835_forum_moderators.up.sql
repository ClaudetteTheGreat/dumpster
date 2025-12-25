-- Forum moderators junction table
CREATE TABLE forum_moderators (
    id SERIAL PRIMARY KEY,
    forum_id INT NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(forum_id, user_id)
);

CREATE INDEX idx_forum_moderators_forum_id ON forum_moderators(forum_id);
CREATE INDEX idx_forum_moderators_user_id ON forum_moderators(user_id);
