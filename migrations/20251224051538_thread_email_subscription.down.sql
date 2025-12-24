-- Remove email notification option from watched threads
DROP INDEX IF EXISTS idx_watched_threads_email;
ALTER TABLE watched_threads DROP COLUMN IF EXISTS email_on_reply;
