-- Add full-text search support for posts and threads

-- Add tsvector columns for full-text search
ALTER TABLE threads ADD COLUMN title_tsv tsvector;
ALTER TABLE ugc_revisions ADD COLUMN content_tsv tsvector;

-- Create GIN indexes for fast full-text search
CREATE INDEX threads_title_tsv_idx ON threads USING GIN(title_tsv);
CREATE INDEX ugc_revisions_content_tsv_idx ON ugc_revisions USING GIN(content_tsv);

-- Create functions to update tsvector columns
CREATE OR REPLACE FUNCTION threads_title_tsv_trigger() RETURNS trigger AS $$
BEGIN
    NEW.title_tsv := to_tsvector('english', COALESCE(NEW.title, ''));
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION ugc_revisions_content_tsv_trigger() RETURNS trigger AS $$
BEGIN
    NEW.content_tsv := to_tsvector('english', COALESCE(NEW.content, ''));
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

-- Create triggers to automatically update tsvector columns
CREATE TRIGGER threads_title_tsv_update
    BEFORE INSERT OR UPDATE OF title
    ON threads
    FOR EACH ROW
    EXECUTE FUNCTION threads_title_tsv_trigger();

CREATE TRIGGER ugc_revisions_content_tsv_update
    BEFORE INSERT OR UPDATE OF content
    ON ugc_revisions
    FOR EACH ROW
    EXECUTE FUNCTION ugc_revisions_content_tsv_trigger();

-- Populate existing data
UPDATE threads SET title_tsv = to_tsvector('english', COALESCE(title, ''));
UPDATE ugc_revisions SET content_tsv = to_tsvector('english', COALESCE(content, ''));
