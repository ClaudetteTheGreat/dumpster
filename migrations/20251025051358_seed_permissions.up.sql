-- Seed basic groups and permissions for forum operation

-- Insert system groups
INSERT INTO groups (id, label, group_type) VALUES
    (1, 'Guests', 'system_guest'),
    (2, 'Registered Users', 'system_user'),
    (3, 'Moderators', 'normal'),
    (4, 'Administrators', 'normal')
ON CONFLICT (id) DO NOTHING;

-- Insert permission categories
INSERT INTO permission_categories (id, label, sort) VALUES
    (1, 'General', 0),
    (2, 'Moderation', 10),
    (3, 'Administration', 20)
ON CONFLICT (id) DO NOTHING;

-- Insert general permissions
INSERT INTO permissions (id, category_id, label, sort) VALUES
    -- General permissions
    (1, 1, 'forum.view', 0),
    (2, 1, 'thread.view', 10),
    (3, 1, 'thread.create', 20),
    (4, 1, 'post.create', 30),
    (5, 1, 'post.edit_own', 40),
    (6, 1, 'post.delete_own', 50),

    -- Moderation permissions
    (10, 2, 'moderate.thread.lock', 0),
    (11, 2, 'moderate.thread.unlock', 10),
    (12, 2, 'moderate.thread.pin', 20),
    (13, 2, 'moderate.thread.unpin', 30),
    (14, 2, 'moderate.thread.move', 40),
    (15, 2, 'moderate.post.edit_any', 50),
    (16, 2, 'moderate.post.delete_any', 60),
    (17, 2, 'moderate.user.warn', 70),

    -- Administration permissions
    (20, 3, 'admin.user.ban', 0),
    (21, 3, 'admin.user.unban', 10),
    (22, 3, 'admin.forum.manage', 20),
    (23, 3, 'admin.permissions.manage', 30),
    (24, 3, 'admin.system.maintenance', 40)
ON CONFLICT (id) DO NOTHING;

-- Create permission collections for each group
INSERT INTO permission_collections (id, group_id, user_id) VALUES
    (1, 1, NULL),  -- Guests collection
    (2, 2, NULL),  -- Registered Users collection
    (3, 3, NULL),  -- Moderators collection
    (4, 4, NULL)   -- Administrators collection
ON CONFLICT DO NOTHING;

-- Set permissions for Guests (read-only)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (1, 1, 'yes'),  -- forum.view
    (2, 1, 'yes')   -- thread.view
ON CONFLICT DO NOTHING;

-- Set permissions for Registered Users
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (1, 2, 'yes'),  -- forum.view
    (2, 2, 'yes'),  -- thread.view
    (3, 2, 'yes'),  -- thread.create
    (4, 2, 'yes'),  -- post.create
    (5, 2, 'yes'),  -- post.edit_own
    (6, 2, 'yes')   -- post.delete_own
ON CONFLICT DO NOTHING;

-- Set permissions for Moderators (all user permissions + moderation)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    -- General permissions
    (1, 3, 'yes'),
    (2, 3, 'yes'),
    (3, 3, 'yes'),
    (4, 3, 'yes'),
    (5, 3, 'yes'),
    (6, 3, 'yes'),
    -- Moderation permissions
    (10, 3, 'yes'),  -- moderate.thread.lock
    (11, 3, 'yes'),  -- moderate.thread.unlock
    (12, 3, 'yes'),  -- moderate.thread.pin
    (13, 3, 'yes'),  -- moderate.thread.unpin
    (14, 3, 'yes'),  -- moderate.thread.move
    (15, 3, 'yes'),  -- moderate.post.edit_any
    (16, 3, 'yes'),  -- moderate.post.delete_any
    (17, 3, 'yes')   -- moderate.user.warn
ON CONFLICT DO NOTHING;

-- Set permissions for Administrators (all permissions)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    -- General permissions
    (1, 4, 'yes'),
    (2, 4, 'yes'),
    (3, 4, 'yes'),
    (4, 4, 'yes'),
    (5, 4, 'yes'),
    (6, 4, 'yes'),
    -- Moderation permissions
    (10, 4, 'yes'),
    (11, 4, 'yes'),
    (12, 4, 'yes'),
    (13, 4, 'yes'),
    (14, 4, 'yes'),
    (15, 4, 'yes'),
    (16, 4, 'yes'),
    (17, 4, 'yes'),
    -- Administration permissions
    (20, 4, 'yes'),  -- admin.user.ban
    (21, 4, 'yes'),  -- admin.user.unban
    (22, 4, 'yes'),  -- admin.forum.manage
    (23, 4, 'yes'),  -- admin.permissions.manage
    (24, 4, 'yes')   -- admin.system.maintenance
ON CONFLICT DO NOTHING;
