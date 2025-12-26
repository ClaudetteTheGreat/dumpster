-- Add report management permissions

-- Add permissions for viewing and managing reports
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (32, 2, 'moderate.reports.view', 70),     -- View report queue
    (33, 2, 'moderate.reports.manage', 71)    -- Update report status
ON CONFLICT (id) DO NOTHING;

-- Grant report permissions to Moderators group (collection_id 3)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (32, 3, 'yes'),  -- moderate.reports.view
    (33, 3, 'yes')   -- moderate.reports.manage
ON CONFLICT DO NOTHING;

-- Grant report permissions to Administrators group (collection_id 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (32, 4, 'yes'),  -- moderate.reports.view
    (33, 4, 'yes')   -- moderate.reports.manage
ON CONFLICT DO NOTHING;
