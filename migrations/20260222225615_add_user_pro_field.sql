-- Add is_pro field to users table
ALTER TABLE users ADD COLUMN is_pro BOOLEAN NOT NULL DEFAULT false;

-- Create index for pro users (for efficient queries)
CREATE INDEX idx_users_is_pro ON users(is_pro) WHERE is_pro = true;
