-- Revert thread move and merge changes

DROP INDEX IF EXISTS idx_threads_merged_into;
ALTER TABLE threads DROP COLUMN IF EXISTS merged_into_id;

DELETE FROM permission_values WHERE permission_id IN (34, 35);
DELETE FROM permissions WHERE id IN (34, 35);
