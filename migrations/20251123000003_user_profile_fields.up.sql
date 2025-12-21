-- Add profile fields to users table

-- Bio: User's about me section (2000 char limit)
ALTER TABLE users ADD COLUMN bio TEXT;

-- Location: Geographic location (255 char limit)
ALTER TABLE users ADD COLUMN location VARCHAR(255);

-- Website URL: Personal website (2048 char limit for URLs)
ALTER TABLE users ADD COLUMN website_url VARCHAR(2048);

-- Signature: Appended to posts, rendered with BBCode (500 char limit)
ALTER TABLE users ADD COLUMN signature TEXT;

-- Add check constraints for length limits (enforced at DB level as backup)
ALTER TABLE users ADD CONSTRAINT bio_length_check CHECK (char_length(bio) <= 2000);
ALTER TABLE users ADD CONSTRAINT location_length_check CHECK (char_length(location) <= 255);
ALTER TABLE users ADD CONSTRAINT website_url_length_check CHECK (char_length(website_url) <= 2048);
ALTER TABLE users ADD CONSTRAINT signature_length_check CHECK (char_length(signature) <= 500);
