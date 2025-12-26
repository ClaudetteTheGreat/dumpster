-- Revert word filter changes

DROP INDEX IF EXISTS idx_word_filters_action;
DROP INDEX IF EXISTS idx_word_filters_enabled;
DROP TABLE IF EXISTS word_filters;

DELETE FROM permission_values WHERE permission_id IN (36, 37);
DELETE FROM permissions WHERE id IN (36, 37);
