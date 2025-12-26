-- Add deletion type to ugc_deletions table
-- Three types:
--   'normal' - Soft delete, visible to moderators, can be restored
--   'permanent' - Hard reference kept for audit, content purged
--   'legal_hold' - Cannot be modified/restored except by admin

-- Create enum type for deletion types
DO $$ BEGIN
    CREATE TYPE deletion_type AS ENUM ('normal', 'permanent', 'legal_hold');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Add deletion_type column with default 'normal'
ALTER TABLE ugc_deletions ADD COLUMN IF NOT EXISTS deletion_type deletion_type NOT NULL DEFAULT 'normal';

-- Add deleted_by_id to track who deleted (moderator/admin)
-- This is different from user_id which tracks the original content author
ALTER TABLE ugc_deletions ADD COLUMN IF NOT EXISTS deleted_by_id INT REFERENCES users(id) ON DELETE SET NULL;

-- Add index for querying by deletion type
CREATE INDEX IF NOT EXISTS idx_ugc_deletions_type ON ugc_deletions(deletion_type);

-- Add legal_hold_at timestamp for legal hold records
ALTER TABLE ugc_deletions ADD COLUMN IF NOT EXISTS legal_hold_at TIMESTAMP;

-- Add legal_hold_by to track who placed the legal hold
ALTER TABLE ugc_deletions ADD COLUMN IF NOT EXISTS legal_hold_by INT REFERENCES users(id) ON DELETE SET NULL;

-- Add legal_hold_reason for legal hold documentation
ALTER TABLE ugc_deletions ADD COLUMN IF NOT EXISTS legal_hold_reason TEXT;
