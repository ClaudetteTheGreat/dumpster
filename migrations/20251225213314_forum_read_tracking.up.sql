-- Forum read tracking: stores when a user last marked a forum as read
-- Any threads with last_post_at > read_at are considered unread
CREATE TABLE forum_read (
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    forum_id INT NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    read_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, forum_id)
);

-- Thread read tracking: stores when a user last viewed a specific thread
-- Any posts with created_at > read_at are considered unread
CREATE TABLE thread_read (
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    thread_id INT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    read_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, thread_id)
);

-- Index for efficient lookups by user
CREATE INDEX idx_forum_read_user ON forum_read(user_id);
CREATE INDEX idx_thread_read_user ON thread_read(user_id);
