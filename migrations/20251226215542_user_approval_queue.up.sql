-- User approval queue system
-- New users can be placed in a pending state until approved by moderators

-- Create approval status enum
DO $$ BEGIN
    CREATE TYPE approval_status AS ENUM ('pending', 'approved', 'rejected');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Add approval columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS approval_status approval_status NOT NULL DEFAULT 'approved';
ALTER TABLE users ADD COLUMN IF NOT EXISTS approved_at TIMESTAMP;
ALTER TABLE users ADD COLUMN IF NOT EXISTS approved_by INT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE users ADD COLUMN IF NOT EXISTS rejection_reason TEXT;

-- Index for finding pending users
CREATE INDEX IF NOT EXISTS idx_users_approval_status ON users(approval_status) WHERE approval_status = 'pending';

-- Add approval permissions
INSERT INTO permissions (id, category_id, label, sort) VALUES
    (44, 2, 'moderate.approval.view', 65),    -- View approval queue
    (45, 2, 'moderate.approval.manage', 66)   -- Approve/reject users
ON CONFLICT (id) DO NOTHING;

-- Grant permissions to Moderators group (collection_id 3)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (44, 3, 'yes'),  -- moderate.approval.view
    (45, 3, 'yes')   -- moderate.approval.manage
ON CONFLICT (permission_id, collection_id) DO NOTHING;

-- Grant permissions to Administrators group (collection_id 4)
INSERT INTO permission_values (permission_id, collection_id, value) VALUES
    (44, 4, 'yes'),  -- moderate.approval.view
    (45, 4, 'yes')   -- moderate.approval.manage
ON CONFLICT (permission_id, collection_id) DO NOTHING;

-- Add setting to enable/disable approval requirement
INSERT INTO settings (key, value, value_type, description, category, is_public) VALUES
    ('require_user_approval', 'false', 'bool', 'Require moderator approval for new user registrations', 'security', FALSE)
ON CONFLICT (key) DO NOTHING;
