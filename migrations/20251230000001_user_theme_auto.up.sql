-- Add theme_auto column to users table
ALTER TABLE users ADD COLUMN theme_auto BOOLEAN NOT NULL DEFAULT FALSE;

-- Migrate 'auto' users to use theme_auto flag
UPDATE users SET theme_auto = TRUE WHERE theme = 'auto';
UPDATE users SET theme = 'light' WHERE theme = 'auto';

-- Drop the old check constraint that only allowed light/dark/auto
ALTER TABLE users DROP CONSTRAINT IF EXISTS theme_check;

-- Make theme nullable (guest/default case) and allow any slug
ALTER TABLE users ALTER COLUMN theme DROP NOT NULL;
ALTER TABLE users ALTER COLUMN theme SET DEFAULT 'light';
