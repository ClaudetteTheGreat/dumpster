-- Add site branding settings
INSERT INTO settings (key, value, value_type, description, category, is_public) VALUES
('site_title', 'ruforo', 'string', 'Site title/brand displayed in navigation header', 'general', TRUE),
('footer_message', 'Live Free or Die', 'string', 'Message displayed in site footer', 'general', TRUE)
ON CONFLICT (key) DO NOTHING;
