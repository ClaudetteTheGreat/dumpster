-- Remove thumbnail settings
DELETE FROM settings WHERE key IN ('enforce_thumbnails', 'thumbnail_max_size');
