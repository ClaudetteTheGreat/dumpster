-- Add min_posts_to_vote setting
INSERT INTO settings (key, value, value_type, description, category, is_public) VALUES
('min_posts_to_vote', '5', 'int', 'Minimum posts required to give reactions', 'reactions', FALSE)
ON CONFLICT (key) DO NOTHING;
