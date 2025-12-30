-- Restore the unique constraint on (group_id, user_id)
-- WARNING: This may fail if forum-specific collections have been created with duplicate (group_id, user_id) pairs
ALTER TABLE permission_collections ADD CONSTRAINT permission_collections_group_id_user_id_key UNIQUE (group_id, user_id);
