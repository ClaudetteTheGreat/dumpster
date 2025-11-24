-- Revert theme preference column

DROP INDEX IF EXISTS idx_users_theme;
ALTER TABLE users DROP CONSTRAINT IF EXISTS theme_check;
ALTER TABLE users DROP COLUMN IF EXISTS theme;
