-- Add thread prefixes support

ALTER TABLE threads ADD COLUMN prefix VARCHAR(50);

-- Create index for searching by prefix
CREATE INDEX idx_threads_prefix ON threads(prefix) WHERE prefix IS NOT NULL;

-- Add comment explaining the column
COMMENT ON COLUMN threads.prefix IS 'Thread prefix like [SOLVED], [QUESTION], [DISCUSSION], etc. Used for categorization and filtering.';
