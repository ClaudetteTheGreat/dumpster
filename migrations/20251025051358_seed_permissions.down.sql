-- Rollback permission seeds

-- Delete in reverse order of dependencies
DELETE FROM permission_values WHERE collection_id IN (1, 2, 3, 4);
DELETE FROM permission_collections WHERE id IN (1, 2, 3, 4);
DELETE FROM permissions WHERE id IN (1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 14, 15, 16, 17, 20, 21, 22, 23, 24);
DELETE FROM permission_categories WHERE id IN (1, 2, 3);
DELETE FROM groups WHERE id IN (1, 2, 3, 4);
