-- Remove min_posts_to_vote setting
DELETE FROM settings WHERE key = 'min_posts_to_vote';
