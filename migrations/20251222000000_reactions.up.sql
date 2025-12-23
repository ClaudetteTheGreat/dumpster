-- Reaction types (like, thanks, funny, etc.)
CREATE TABLE reaction_types (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    emoji VARCHAR(10) NOT NULL,
    display_order INT NOT NULL DEFAULT 0,
    is_positive BOOLEAN NOT NULL DEFAULT TRUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- Insert default reaction types
INSERT INTO reaction_types (name, emoji, display_order, is_positive) VALUES
    ('like', '👍', 1, TRUE),
    ('thanks', '🙏', 2, TRUE),
    ('funny', '😂', 3, TRUE),
    ('informative', '💡', 4, TRUE),
    ('agree', '✅', 5, TRUE),
    ('disagree', '❌', 6, FALSE);

-- User reactions on content (posts, chat messages, etc.)
CREATE TABLE ugc_reactions (
    id SERIAL PRIMARY KEY,
    ugc_id INT NOT NULL REFERENCES ugc(id) ON DELETE CASCADE,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reaction_type_id INT NOT NULL REFERENCES reaction_types(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(ugc_id, user_id, reaction_type_id)
);

-- Indexes for efficient queries
CREATE INDEX idx_ugc_reactions_ugc ON ugc_reactions(ugc_id);
CREATE INDEX idx_ugc_reactions_user ON ugc_reactions(user_id);
CREATE INDEX idx_ugc_reactions_type ON ugc_reactions(reaction_type_id);

-- Denormalized reaction counts on ugc for performance
ALTER TABLE ugc ADD COLUMN reaction_count INT NOT NULL DEFAULT 0;

-- Function to update reaction count
CREATE OR REPLACE FUNCTION update_ugc_reaction_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE ugc SET reaction_count = reaction_count + 1 WHERE id = NEW.ugc_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE ugc SET reaction_count = reaction_count - 1 WHERE id = OLD.ugc_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger to keep reaction count in sync
CREATE TRIGGER trigger_ugc_reaction_count
AFTER INSERT OR DELETE ON ugc_reactions
FOR EACH ROW EXECUTE FUNCTION update_ugc_reaction_count();
