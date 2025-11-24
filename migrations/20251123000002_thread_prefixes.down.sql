-- Remove thread prefix support

DROP INDEX IF EXISTS idx_threads_prefix;
ALTER TABLE threads DROP COLUMN IF EXISTS prefix;
