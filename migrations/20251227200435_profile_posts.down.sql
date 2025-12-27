-- Revert profile posts

DROP TABLE IF EXISTS profile_posts;

ALTER TABLE users DROP COLUMN IF EXISTS allow_profile_posts;
