-- Remove global chat settings
DELETE FROM settings WHERE key IN (
    'chat_max_message_length',
    'chat_history_limit',
    'chat_min_posts_to_send',
    'chat_rate_limit_seconds'
);

-- Remove per-room access control columns
ALTER TABLE chat_rooms DROP COLUMN IF EXISTS min_posts_required;
ALTER TABLE chat_rooms DROP COLUMN IF EXISTS min_account_age_hours;
ALTER TABLE chat_rooms DROP COLUMN IF EXISTS is_staff_only;
