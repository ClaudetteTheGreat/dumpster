-- Remove admin.user.manage permission
DELETE FROM permission_values WHERE permission_id = 38;
DELETE FROM permissions WHERE id = 38;
