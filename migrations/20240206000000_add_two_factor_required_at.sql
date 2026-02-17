-- Add two_factor_required_at to users table
ALTER TABLE users
ADD COLUMN two_factor_required_at TIMESTAMPTZ;

-- Add comment
COMMENT ON COLUMN users.two_factor_required_at IS 'Timestamp when user was required to enable 2FA (for grace period tracking)';
