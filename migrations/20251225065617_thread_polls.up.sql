-- Thread polls feature
-- Allows users to create polls when creating threads

-- Polls table (one per thread, optional)
CREATE TABLE polls (
    id SERIAL PRIMARY KEY,
    thread_id INT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    question VARCHAR(500) NOT NULL,
    max_choices INT NOT NULL DEFAULT 1,  -- Number of options a user can select
    allow_change_vote BOOLEAN NOT NULL DEFAULT TRUE,
    show_results_before_vote BOOLEAN NOT NULL DEFAULT FALSE,
    closes_at TIMESTAMP NULL,  -- Optional poll expiration
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(thread_id)  -- One poll per thread
);

-- Poll options (2-10 options per poll)
CREATE TABLE poll_options (
    id SERIAL PRIMARY KEY,
    poll_id INT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    option_text VARCHAR(200) NOT NULL,
    display_order INT NOT NULL DEFAULT 0,
    vote_count INT NOT NULL DEFAULT 0  -- Cached vote count for performance
);

CREATE INDEX idx_poll_options_poll ON poll_options(poll_id, display_order);

-- Poll votes (one vote per user per poll, or multiple if max_choices > 1)
CREATE TABLE poll_votes (
    id SERIAL PRIMARY KEY,
    poll_id INT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    option_id INT NOT NULL REFERENCES poll_options(id) ON DELETE CASCADE,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(poll_id, option_id, user_id)  -- Prevent duplicate votes on same option
);

CREATE INDEX idx_poll_votes_poll ON poll_votes(poll_id);
CREATE INDEX idx_poll_votes_user ON poll_votes(user_id);
CREATE INDEX idx_poll_votes_option ON poll_votes(option_id);

-- Function to update vote count when a vote is cast
CREATE OR REPLACE FUNCTION update_poll_option_vote_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE poll_options SET vote_count = vote_count + 1 WHERE id = NEW.option_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE poll_options SET vote_count = vote_count - 1 WHERE id = OLD.option_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Trigger for vote count updates
CREATE TRIGGER trigger_poll_vote_count
AFTER INSERT OR DELETE ON poll_votes
FOR EACH ROW EXECUTE FUNCTION update_poll_option_vote_count();
