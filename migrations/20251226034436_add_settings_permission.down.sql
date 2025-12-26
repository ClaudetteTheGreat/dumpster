-- Remove admin.settings permission value
DELETE FROM permission_values WHERE permission_id = 25;

-- Remove admin.settings permission
DELETE FROM permissions WHERE id = 25;
