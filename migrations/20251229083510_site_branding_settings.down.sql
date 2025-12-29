-- Remove site branding settings
DELETE FROM settings WHERE key IN ('site_title', 'footer_message');
