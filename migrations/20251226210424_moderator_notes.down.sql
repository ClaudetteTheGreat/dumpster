-- Remove moderator notes permissions
DELETE FROM permission_values WHERE permission_id IN (39, 40);
DELETE FROM permissions WHERE id IN (39, 40);

-- Drop moderator notes table
DROP TABLE IF EXISTS moderator_notes;
