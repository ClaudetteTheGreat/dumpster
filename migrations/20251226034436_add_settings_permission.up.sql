-- Add admin.settings permission
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (25, 3, 'admin.settings', 50)
ON CONFLICT (id) DO NOTHING;

-- Grant admin.settings permission to Administrators group (group_id = 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (25, 4, 'yes')
ON CONFLICT (permission_id, collection_id) DO NOTHING;
