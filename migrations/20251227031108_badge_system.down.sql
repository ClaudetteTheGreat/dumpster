-- Rollback Badge System Migration

-- Remove permission values
DELETE FROM permission_values
WHERE permission_id IN (SELECT id FROM permissions WHERE label = 'admin.badges.manage');

-- Remove permissions
DELETE FROM permissions WHERE label = 'admin.badges.manage';

-- Drop tables
DROP TABLE IF EXISTS user_badges;
DROP TABLE IF EXISTS badges;

-- Drop enum type
DROP TYPE IF EXISTS badge_condition_type;
