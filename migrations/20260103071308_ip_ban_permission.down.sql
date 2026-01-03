-- Remove IP ban permission

DELETE FROM permission_values WHERE permission_id = 48;
DELETE FROM permissions WHERE id = 48;
