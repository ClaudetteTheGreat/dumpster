-- Notifications table
CREATE TABLE notifications (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    url VARCHAR(500),
    source_user_id INT REFERENCES users(id) ON DELETE SET NULL,
    source_content_type VARCHAR(50),
    source_content_id INT,
    is_read BOOLEAN DEFAULT FALSE,
    is_emailed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT NOW(),
    read_at TIMESTAMP
);

CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_unread ON notifications(user_id, is_read);
CREATE INDEX idx_notifications_created ON notifications(created_at DESC);

-- Notification preferences
CREATE TABLE notification_preferences (
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    notification_type VARCHAR(50) NOT NULL,
    in_app BOOLEAN DEFAULT TRUE,
    email BOOLEAN DEFAULT TRUE,
    frequency VARCHAR(20) DEFAULT 'immediate',
    PRIMARY KEY (user_id, notification_type)
);

-- Watched threads for notifications
CREATE TABLE watched_threads (
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    thread_id INT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    notify_on_reply BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (user_id, thread_id)
);

CREATE INDEX idx_watched_threads_thread ON watched_threads(thread_id);

-- Insert default notification preferences for existing users
INSERT INTO notification_preferences (user_id, notification_type, in_app, email, frequency)
SELECT id, 'reply', TRUE, TRUE, 'immediate' FROM users;

INSERT INTO notification_preferences (user_id, notification_type, in_app, email, frequency)
SELECT id, 'mention', TRUE, TRUE, 'immediate' FROM users;

INSERT INTO notification_preferences (user_id, notification_type, in_app, email, frequency)
SELECT id, 'pm', TRUE, TRUE, 'immediate' FROM users;

INSERT INTO notification_preferences (user_id, notification_type, in_app, email, frequency)
SELECT id, 'quote', TRUE, FALSE, 'immediate' FROM users;

INSERT INTO notification_preferences (user_id, notification_type, in_app, email, frequency)
SELECT id, 'thread_watch', TRUE, FALSE, 'immediate' FROM users;
