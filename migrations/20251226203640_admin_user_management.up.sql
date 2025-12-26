-- Add admin.user.manage permission
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (38, 3, 'admin.user.manage', 55)
ON CONFLICT (id) DO NOTHING;

-- Grant admin.user.manage permission to Administrators group (collection_id = 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (38, 4, 'yes')
ON CONFLICT (permission_id, collection_id) DO NOTHING;
