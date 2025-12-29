-- Restore forum_id column to tags
ALTER TABLE tags ADD COLUMN forum_id INT REFERENCES forums(id);

-- Migrate data back (pick first forum if multiple)
UPDATE tags t SET forum_id = (
    SELECT forum_id FROM tag_forums tf WHERE tf.tag_id = t.id LIMIT 1
);

-- Set forum_id to NULL for global tags
UPDATE tags SET forum_id = NULL WHERE is_global = TRUE;

-- Drop is_global column
ALTER TABLE tags DROP COLUMN is_global;

-- Drop junction table
DROP TABLE tag_forums;
