-- Add email notification option to watched threads
ALTER TABLE watched_threads ADD COLUMN email_on_reply BOOLEAN NOT NULL DEFAULT false;

-- Index for finding users to email when a thread gets a new post
CREATE INDEX idx_watched_threads_email ON watched_threads (thread_id) WHERE email_on_reply = true;
