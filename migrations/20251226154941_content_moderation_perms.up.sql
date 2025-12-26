-- Add content moderation permissions for deletion types

-- Add new permissions
INSERT INTO permissions (id, category_id, label, sort) VALUES
    -- Moderation permissions for content management
    (25, 2, 'moderate.post.restore', 65),           -- Restore soft-deleted posts
    (26, 2, 'moderate.post.delete_permanent', 66),  -- Permanently delete (spam removal)
    (27, 2, 'moderate.thread.delete_any', 67),      -- Delete any thread
    (28, 2, 'moderate.thread.restore', 68),         -- Restore soft-deleted threads
    (29, 2, 'moderate.thread.delete_permanent', 69), -- Permanently delete threads

    -- Admin permissions for legal holds
    (30, 3, 'admin.content.legal_hold', 50),        -- Place legal holds on content
    (31, 3, 'admin.content.remove_legal_hold', 51)  -- Remove legal holds
ON CONFLICT (id) DO NOTHING;

-- Grant moderation permissions to Moderators group
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (25, 3, 'yes'),  -- moderate.post.restore
    (26, 3, 'yes'),  -- moderate.post.delete_permanent
    (27, 3, 'yes'),  -- moderate.thread.delete_any
    (28, 3, 'yes'),  -- moderate.thread.restore
    (29, 3, 'yes')   -- moderate.thread.delete_permanent
ON CONFLICT DO NOTHING;

-- Grant all permissions to Administrators group
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (25, 4, 'yes'),  -- moderate.post.restore
    (26, 4, 'yes'),  -- moderate.post.delete_permanent
    (27, 4, 'yes'),  -- moderate.thread.delete_any
    (28, 4, 'yes'),  -- moderate.thread.restore
    (29, 4, 'yes'),  -- moderate.thread.delete_permanent
    (30, 4, 'yes'),  -- admin.content.legal_hold
    (31, 4, 'yes')   -- admin.content.remove_legal_hold
ON CONFLICT DO NOTHING;
