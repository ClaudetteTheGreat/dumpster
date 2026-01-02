-- Remove chat image domain whitelist setting
DELETE FROM settings WHERE key = 'chat_image_domain_whitelist';
