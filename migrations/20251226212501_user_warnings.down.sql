-- Remove warning settings
DELETE FROM settings WHERE key IN ('warning_threshold', 'warning_ban_duration_days', 'warning_expiry_days');

-- Remove warning permissions
DELETE FROM permission_values WHERE permission_id IN (41, 42, 43);
DELETE FROM permissions WHERE id IN (41, 42, 43);

-- Remove warning columns from users table
ALTER TABLE users DROP COLUMN IF EXISTS warning_points;
ALTER TABLE users DROP COLUMN IF EXISTS last_warning_at;

-- Drop user warnings table
DROP TABLE IF EXISTS user_warnings;
