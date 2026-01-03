-- Add IP ban permission for administrators

-- Insert the permission
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (48, 3, 'admin.ip.ban', 55)
ON CONFLICT (id) DO NOTHING;

-- Grant to Administrators (collection_id 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (48, 4, 'yes')
ON CONFLICT DO NOTHING;
