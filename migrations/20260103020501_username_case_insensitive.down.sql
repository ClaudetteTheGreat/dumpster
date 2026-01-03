-- Revert case-insensitive username indexes
DROP INDEX IF EXISTS idx_user_names_lower_name;
DROP INDEX IF EXISTS idx_user_names_name_lower;
