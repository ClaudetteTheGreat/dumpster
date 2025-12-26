-- Revert deletion types changes

ALTER TABLE ugc_deletions DROP COLUMN IF EXISTS legal_hold_reason;
ALTER TABLE ugc_deletions DROP COLUMN IF EXISTS legal_hold_by;
ALTER TABLE ugc_deletions DROP COLUMN IF EXISTS legal_hold_at;
DROP INDEX IF EXISTS idx_ugc_deletions_type;
ALTER TABLE ugc_deletions DROP COLUMN IF EXISTS deleted_by_id;
ALTER TABLE ugc_deletions DROP COLUMN IF EXISTS deletion_type;
DROP TYPE IF EXISTS deletion_type;
