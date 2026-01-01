-- Remove user preference column
ALTER TABLE users DROP COLUMN IF EXISTS default_chat_room;

-- Remove admin setting
DELETE FROM settings WHERE key = 'chat_default_room';
