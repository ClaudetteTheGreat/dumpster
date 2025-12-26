-- Mass moderation permissions
-- Allow moderators to perform bulk actions on users and content

INSERT INTO permissions (id, category_id, label, sort) VALUES
    (46, 2, 'moderate.mass.users', 70),     -- Mass actions on users (ban, warn, etc.)
    (47, 2, 'moderate.mass.content', 71)    -- Mass actions on content (delete, move, etc.)
ON CONFLICT (id) DO NOTHING;

-- Grant permissions to Moderators group (collection_id 3)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (46, 3, 'yes'),  -- moderate.mass.users
    (47, 3, 'yes')   -- moderate.mass.content
ON CONFLICT (permission_id, collection_id) DO NOTHING;

-- Grant permissions to Administrators group (collection_id 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (46, 4, 'yes'),  -- moderate.mass.users
    (47, 4, 'yes')   -- moderate.mass.content
ON CONFLICT (permission_id, collection_id) DO NOTHING;
