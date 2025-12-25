-- Remove thread tags feature
DROP TRIGGER IF EXISTS trigger_tag_use_count ON thread_tags;
DROP FUNCTION IF EXISTS update_tag_use_count();
DROP TABLE IF EXISTS thread_tags;
DROP TABLE IF EXISTS tags;
