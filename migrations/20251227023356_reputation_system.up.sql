-- Add reputation_value to reaction_types
ALTER TABLE reaction_types ADD COLUMN reputation_value INTEGER NOT NULL DEFAULT 0;

-- Update existing reactions with sensible defaults
UPDATE reaction_types SET reputation_value = 1 WHERE name IN ('like', 'thanks', 'informative', 'agree');
UPDATE reaction_types SET reputation_value = -1 WHERE name IN ('disagree');
UPDATE reaction_types SET reputation_value = 0 WHERE name = 'funny';

-- Add reputation_score to users
ALTER TABLE users ADD COLUMN reputation_score INTEGER NOT NULL DEFAULT 0;

-- Create index for sorting by reputation
CREATE INDEX idx_users_reputation ON users(reputation_score DESC);

-- Add minimum posts required to vote setting
INSERT INTO settings (key, value, value_type, description, category, is_public, updated_at)
VALUES ('min_posts_to_vote', '5', 'int', 'Minimum posts required to give reactions', 'reactions', false, NOW())
ON CONFLICT (key) DO NOTHING;

-- Trigger function to update user reputation when reactions change
CREATE OR REPLACE FUNCTION update_user_reputation()
RETURNS TRIGGER AS $$
DECLARE
    post_author_id INTEGER;
    rep_value INTEGER;
BEGIN
    -- Get the post author from ugc -> posts
    SELECT p.user_id INTO post_author_id
    FROM posts p
    WHERE p.ugc_id = COALESCE(NEW.ugc_id, OLD.ugc_id);

    IF post_author_id IS NULL THEN
        RETURN COALESCE(NEW, OLD);
    END IF;

    -- Get reputation value for this reaction type
    SELECT reputation_value INTO rep_value
    FROM reaction_types
    WHERE id = COALESCE(NEW.reaction_type_id, OLD.reaction_type_id);

    IF rep_value IS NULL THEN
        rep_value := 0;
    END IF;

    IF TG_OP = 'INSERT' THEN
        UPDATE users SET reputation_score = reputation_score + rep_value
        WHERE id = post_author_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE users SET reputation_score = reputation_score - rep_value
        WHERE id = post_author_id;
    END IF;

    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Create trigger for reputation updates
CREATE TRIGGER trigger_update_user_reputation
AFTER INSERT OR DELETE ON ugc_reactions
FOR EACH ROW EXECUTE FUNCTION update_user_reputation();

-- Calculate initial reputation scores from existing reactions
UPDATE users u
SET reputation_score = COALESCE((
    SELECT SUM(rt.reputation_value)
    FROM ugc_reactions ur
    JOIN reaction_types rt ON rt.id = ur.reaction_type_id
    JOIN posts p ON p.ugc_id = ur.ugc_id
    WHERE p.user_id = u.id
), 0);
