-- Add custom_title column to users table
-- Allows users to set a custom title displayed under their username

ALTER TABLE users ADD COLUMN custom_title VARCHAR(100);

-- Add check constraint for length limit
ALTER TABLE users ADD CONSTRAINT custom_title_length_check CHECK (char_length(custom_title) <= 100);
