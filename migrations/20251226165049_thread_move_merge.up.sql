-- Add thread move and merge permissions

-- Add permissions for moving and merging threads
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (34, 2, 'moderate.thread.move', 72),    -- Move threads between forums
    (35, 2, 'moderate.thread.merge', 73)    -- Merge threads together
ON CONFLICT (id) DO NOTHING;

-- Grant move/merge permissions to Moderators group (collection_id 3)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (34, 3, 'yes'),  -- moderate.thread.move
    (35, 3, 'yes')   -- moderate.thread.merge
ON CONFLICT (permission_id, collection_id) DO NOTHING;

-- Grant move/merge permissions to Administrators group (collection_id 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (34, 4, 'yes'),  -- moderate.thread.move
    (35, 4, 'yes')   -- moderate.thread.merge
ON CONFLICT (permission_id, collection_id) DO NOTHING;

-- Add merged_into_id column to track merged threads
ALTER TABLE threads ADD COLUMN IF NOT EXISTS merged_into_id INT REFERENCES threads(id) ON DELETE SET NULL;

-- Index for finding merged threads
CREATE INDEX IF NOT EXISTS idx_threads_merged_into ON threads(merged_into_id) WHERE merged_into_id IS NOT NULL;
