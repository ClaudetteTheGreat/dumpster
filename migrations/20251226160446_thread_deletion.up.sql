-- Add deletion support to threads table
-- Uses the same deletion_type enum from ugc_deletions migration

-- Add deletion fields to threads table
ALTER TABLE threads ADD COLUMN deleted_at TIMESTAMP;
ALTER TABLE threads ADD COLUMN deleted_by INT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE threads ADD COLUMN deletion_type deletion_type;
ALTER TABLE threads ADD COLUMN deletion_reason TEXT;
ALTER TABLE threads ADD COLUMN legal_hold_at TIMESTAMP;
ALTER TABLE threads ADD COLUMN legal_hold_by INT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE threads ADD COLUMN legal_hold_reason TEXT;

-- Index for querying non-deleted threads efficiently
CREATE INDEX idx_threads_deleted ON threads(deleted_at) WHERE deleted_at IS NULL;

-- Index for querying threads by deletion type
CREATE INDEX idx_threads_deletion_type ON threads(deletion_type) WHERE deletion_type IS NOT NULL;
