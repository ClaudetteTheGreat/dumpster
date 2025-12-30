-- Drop the unique constraint on (group_id, user_id) to allow forum-specific permission collections
-- Each forum can have its own permission collection for a group, linked via forum_permissions table
ALTER TABLE permission_collections DROP CONSTRAINT IF EXISTS permission_collections_group_id_user_id_key;
