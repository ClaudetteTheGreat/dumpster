-- Remove report permissions

DELETE FROM permission_values WHERE permission_id IN (32, 33);
DELETE FROM permissions WHERE id IN (32, 33);
