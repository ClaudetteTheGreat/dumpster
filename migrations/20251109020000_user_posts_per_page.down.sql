-- Remove posts_per_page preference from users table

ALTER TABLE users
DROP CONSTRAINT posts_per_page_valid;

ALTER TABLE users
DROP COLUMN posts_per_page;
