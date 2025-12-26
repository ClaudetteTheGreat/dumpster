-- Revert thread deletion changes

DROP INDEX IF EXISTS idx_threads_deletion_type;
DROP INDEX IF EXISTS idx_threads_deleted;
ALTER TABLE threads DROP COLUMN IF EXISTS legal_hold_reason;
ALTER TABLE threads DROP COLUMN IF EXISTS legal_hold_by;
ALTER TABLE threads DROP COLUMN IF EXISTS legal_hold_at;
ALTER TABLE threads DROP COLUMN IF EXISTS deletion_reason;
ALTER TABLE threads DROP COLUMN IF EXISTS deletion_type;
ALTER TABLE threads DROP COLUMN IF EXISTS deleted_by;
ALTER TABLE threads DROP COLUMN IF EXISTS deleted_at;
