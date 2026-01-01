-- Add admin setting for default chat room
INSERT INTO settings (key, value, value_type, description, category, is_public)
VALUES ('chat_default_room', '0', 'int', 'Default chat room ID to auto-join (0 = none, user must select)', 'chat', false)
ON CONFLICT (key) DO NOTHING;

-- Add user preference column for default chat room override
ALTER TABLE users ADD COLUMN IF NOT EXISTS default_chat_room INTEGER DEFAULT NULL;

-- Add comment for documentation
COMMENT ON COLUMN users.default_chat_room IS 'User override for default chat room to auto-join (NULL = use site default)';
