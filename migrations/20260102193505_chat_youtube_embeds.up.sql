-- Add setting to enable/disable YouTube embeds in chat
INSERT INTO settings (key, value, value_type, description, category, is_public)
VALUES ('chat_embed_youtube', 'true', 'bool', 'Allow YouTube video embeds in chat messages', 'chat', false)
ON CONFLICT (key) DO NOTHING;
