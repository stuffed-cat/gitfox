-- Add email verification setting to system_configs
-- SMTP server settings should be configured via environment variables
INSERT INTO system_configs (key, value) VALUES
    ('require_email_confirmation', 'false')
ON CONFLICT (key) DO NOTHING;

-- Add email confirmation fields to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_confirmed BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_confirmation_token TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_confirmation_sent_at TIMESTAMPTZ;

-- Add password reset fields to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_reset_token TEXT;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_reset_sent_at TIMESTAMPTZ;

-- Create index for token lookups
CREATE INDEX IF NOT EXISTS idx_users_email_confirmation_token ON users(email_confirmation_token) WHERE email_confirmation_token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_password_reset_token ON users(password_reset_token) WHERE password_reset_token IS NOT NULL;
z