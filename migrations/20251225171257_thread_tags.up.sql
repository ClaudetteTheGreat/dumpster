-- Thread tags feature
-- Tags can be global (forum_id NULL) or forum-specific

CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    slug VARCHAR(50) NOT NULL,
    color VARCHAR(7) DEFAULT '#6c757d', -- Hex color for display
    forum_id INT NULL REFERENCES forums(id) ON DELETE CASCADE,
    use_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(slug, forum_id)
);

-- Junction table for thread-tag relationships
CREATE TABLE thread_tags (
    id SERIAL PRIMARY KEY,
    thread_id INT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    tag_id INT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE(thread_id, tag_id)
);

-- Indexes for efficient lookups
CREATE INDEX idx_tags_forum_id ON tags(forum_id);
CREATE INDEX idx_tags_slug ON tags(slug);
CREATE INDEX idx_thread_tags_thread_id ON thread_tags(thread_id);
CREATE INDEX idx_thread_tags_tag_id ON thread_tags(tag_id);

-- Trigger to update tag use_count when thread_tags change
CREATE OR REPLACE FUNCTION update_tag_use_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE tags SET use_count = use_count + 1 WHERE id = NEW.tag_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE tags SET use_count = use_count - 1 WHERE id = OLD.tag_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_tag_use_count
AFTER INSERT OR DELETE ON thread_tags
FOR EACH ROW EXECUTE FUNCTION update_tag_use_count();
