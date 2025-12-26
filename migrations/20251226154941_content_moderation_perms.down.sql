-- Remove content moderation permissions

DELETE FROM permission_values WHERE permission_id IN (25, 26, 27, 28, 29, 30, 31);
DELETE FROM permissions WHERE id IN (25, 26, 27, 28, 29, 30, 31);
