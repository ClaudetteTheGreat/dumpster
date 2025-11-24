-- Add theme preference column to users table for dark mode support

ALTER TABLE users ADD COLUMN theme VARCHAR(20) DEFAULT 'light' NOT NULL;

-- Add check constraint to ensure only valid theme values
ALTER TABLE users ADD CONSTRAINT theme_check CHECK (theme IN ('light', 'dark', 'auto'));

-- Add index for potential future queries filtering by theme
CREATE INDEX idx_users_theme ON users(theme);

-- Update existing users to have light theme as default
UPDATE users SET theme = 'light' WHERE theme IS NULL;
