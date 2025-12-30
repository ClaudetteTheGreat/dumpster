-- Add thread_template column to forums table
-- This text appears as a faded placeholder in the new thread content box
ALTER TABLE forums ADD COLUMN thread_template TEXT;

COMMENT ON COLUMN forums.thread_template IS 'Template/placeholder text shown in new thread content box';
