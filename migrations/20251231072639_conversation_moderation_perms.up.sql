-- Add permission for moderators to delete any conversation message
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (32, 2, 'moderate.message.delete_any', 70)
ON CONFLICT (id) DO NOTHING;

-- Grant to Moderators group (collection_id = 3) and Administrators group (collection_id = 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (32, 3, 'yes'),
    (32, 4, 'yes')
ON CONFLICT (permission_id, collection_id) DO NOTHING;
