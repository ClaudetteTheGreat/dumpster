-- Make usernames case-insensitive by adding a unique index on LOWER(name)
-- This prevents users from registering "User" if "user" already exists

-- Create unique index on lowercase name for case-insensitive uniqueness
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_names_lower_name ON user_names (LOWER(name));

-- Also add index for faster case-insensitive lookups during login
CREATE INDEX IF NOT EXISTS idx_user_names_name_lower ON user_names USING btree (LOWER(name));
