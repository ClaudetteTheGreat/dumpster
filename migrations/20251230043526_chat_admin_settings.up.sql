-- Global chat settings
INSERT INTO settings (key, value, value_type, description, category, is_public) VALUES
('chat_max_message_length', '1024', 'int', 'Maximum message length in bytes (0 for unlimited)', 'chat', FALSE),
('chat_history_limit', '40', 'int', 'Number of messages to load when joining a room', 'chat', FALSE),
('chat_rate_limit_seconds', '0', 'int', 'Minimum seconds between chat messages per user (0 to disable)', 'chat', FALSE);

-- Per-room access control settings
ALTER TABLE chat_rooms ADD COLUMN min_posts_required INT NOT NULL DEFAULT 0;
ALTER TABLE chat_rooms ADD COLUMN min_account_age_hours INT NOT NULL DEFAULT 0;
ALTER TABLE chat_rooms ADD COLUMN is_staff_only BOOLEAN NOT NULL DEFAULT FALSE;

-- Create a default room if none exists
INSERT INTO chat_rooms (title, description, display_order)
SELECT 'General', 'General chat room', 0
WHERE NOT EXISTS (SELECT 1 FROM chat_rooms LIMIT 1);
