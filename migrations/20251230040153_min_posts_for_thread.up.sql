-- Add setting for minimum posts required to create a thread
INSERT INTO settings (key, value, value_type, description, category, is_public)
VALUES (
    'min_posts_to_create_thread',
    '0',
    'int',
    'Minimum number of approved posts required before a user can create new threads. Set to 0 to disable.',
    'moderation',
    FALSE
);
