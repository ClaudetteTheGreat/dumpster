-- Restore theme column constraint
ALTER TABLE users ALTER COLUMN theme SET NOT NULL;
ALTER TABLE users ALTER COLUMN theme SET DEFAULT 'light';

-- Restore 'auto' values from theme_auto flag
UPDATE users SET theme = 'auto' WHERE theme_auto = TRUE;

-- Re-add the old check constraint
ALTER TABLE users ADD CONSTRAINT theme_check
    CHECK (theme IN ('light', 'dark', 'auto'));

-- Drop theme_auto column
ALTER TABLE users DROP COLUMN IF EXISTS theme_auto;
