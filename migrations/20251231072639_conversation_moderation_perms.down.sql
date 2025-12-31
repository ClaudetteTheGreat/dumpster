-- Remove conversation moderation permission values
DELETE FROM permission_values WHERE permission_id = 32;

-- Remove the permission
DELETE FROM permissions WHERE id = 32;
