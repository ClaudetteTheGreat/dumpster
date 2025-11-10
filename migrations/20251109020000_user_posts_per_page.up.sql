-- Add posts_per_page preference to users table
-- Allows users to customize how many posts they see per page

ALTER TABLE users
ADD COLUMN posts_per_page INTEGER NOT NULL DEFAULT 25;

-- Valid values are 10, 25, 50, 100
-- Add a check constraint to ensure only valid values
ALTER TABLE users
ADD CONSTRAINT posts_per_page_valid
CHECK (posts_per_page IN (10, 25, 50, 100));
