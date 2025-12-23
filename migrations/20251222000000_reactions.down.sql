-- Drop trigger and function
DROP TRIGGER IF EXISTS trigger_ugc_reaction_count ON ugc_reactions;
DROP FUNCTION IF EXISTS update_ugc_reaction_count();

-- Remove reaction_count column from ugc
ALTER TABLE ugc DROP COLUMN IF EXISTS reaction_count;

-- Drop tables
DROP TABLE IF EXISTS ugc_reactions;
DROP TABLE IF EXISTS reaction_types;
