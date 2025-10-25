-- Rollback full-text search support

-- Drop triggers
DROP TRIGGER IF EXISTS threads_title_tsv_update ON threads;
DROP TRIGGER IF EXISTS ugc_revisions_content_tsv_update ON ugc_revisions;

-- Drop functions
DROP FUNCTION IF EXISTS threads_title_tsv_trigger();
DROP FUNCTION IF EXISTS ugc_revisions_content_tsv_trigger();

-- Drop indexes
DROP INDEX IF EXISTS threads_title_tsv_idx;
DROP INDEX IF EXISTS ugc_revisions_content_tsv_idx;

-- Drop columns
ALTER TABLE threads DROP COLUMN IF EXISTS title_tsv;
ALTER TABLE ugc_revisions DROP COLUMN IF EXISTS content_tsv;
