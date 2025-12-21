-- Revert profile fields

ALTER TABLE users DROP CONSTRAINT IF EXISTS signature_length_check;
ALTER TABLE users DROP CONSTRAINT IF EXISTS website_url_length_check;
ALTER TABLE users DROP CONSTRAINT IF EXISTS location_length_check;
ALTER TABLE users DROP CONSTRAINT IF EXISTS bio_length_check;

ALTER TABLE users DROP COLUMN IF EXISTS signature;
ALTER TABLE users DROP COLUMN IF EXISTS website_url;
ALTER TABLE users DROP COLUMN IF EXISTS location;
ALTER TABLE users DROP COLUMN IF EXISTS bio;
