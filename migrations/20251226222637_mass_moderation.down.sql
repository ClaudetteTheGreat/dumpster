-- Remove mass moderation permissions
DELETE FROM permission_values WHERE permission_id IN (46, 47);
DELETE FROM permissions WHERE id IN (46, 47);
