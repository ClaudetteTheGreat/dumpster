-- Add thumbnail settings
INSERT INTO settings (key, value, value_type, description, category, is_public)
VALUES
    ('enforce_thumbnails', 'false', 'bool', 'Force all inserted images to use thumbnail format instead of full-size', 'display', false),
    ('thumbnail_max_size', '150', 'int', 'Maximum width/height in pixels for thumbnail images', 'display', true)
ON CONFLICT (key) DO NOTHING;
