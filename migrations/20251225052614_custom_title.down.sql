-- Remove custom_title column from users table
ALTER TABLE users DROP CONSTRAINT IF EXISTS custom_title_length_check;
ALTER TABLE users DROP COLUMN IF EXISTS custom_title;
