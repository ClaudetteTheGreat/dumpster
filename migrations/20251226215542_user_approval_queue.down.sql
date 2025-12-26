-- Remove approval setting
DELETE FROM settings WHERE key = 'require_user_approval';

-- Remove approval permissions
DELETE FROM permission_values WHERE permission_id IN (44, 45);
DELETE FROM permissions WHERE id IN (44, 45);

-- Remove approval columns from users table
ALTER TABLE users DROP COLUMN IF EXISTS rejection_reason;
ALTER TABLE users DROP COLUMN IF EXISTS approved_by;
ALTER TABLE users DROP COLUMN IF EXISTS approved_at;
ALTER TABLE users DROP COLUMN IF EXISTS approval_status;

-- Drop approval status enum
DROP TYPE IF EXISTS approval_status;
