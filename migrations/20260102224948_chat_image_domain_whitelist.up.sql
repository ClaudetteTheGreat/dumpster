-- Chat image domain whitelist setting
-- Comma-separated list of domains allowed to show image thumbnails in chat
-- Use "*" to allow all domains (default), or specify domains like "example.com,cdn.example.com"
INSERT INTO settings (key, value, value_type, description, category, is_public)
VALUES (
    'chat_image_domain_whitelist',
    '*',
    'string',
    'Comma-separated list of domains allowed to show image thumbnails in chat. Use * to allow all domains.',
    'chat',
    false
)
ON CONFLICT (key) DO NOTHING;
