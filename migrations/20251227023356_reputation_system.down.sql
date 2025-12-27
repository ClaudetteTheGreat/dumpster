-- Drop trigger
DROP TRIGGER IF EXISTS trigger_update_user_reputation ON ugc_reactions;

-- Drop function
DROP FUNCTION IF EXISTS update_user_reputation();

-- Remove setting
DELETE FROM settings WHERE key = 'min_posts_to_vote';

-- Drop index
DROP INDEX IF EXISTS idx_users_reputation;

-- Remove reputation_score from users
ALTER TABLE users DROP COLUMN IF EXISTS reputation_score;

-- Remove reputation_value from reaction_types
ALTER TABLE reaction_types DROP COLUMN IF EXISTS reputation_value;
