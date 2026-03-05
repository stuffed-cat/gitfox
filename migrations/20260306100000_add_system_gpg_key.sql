-- Add system GPG key support
-- System keys are auto-generated for users and used for WebIDE/API commits
-- They are not visible in the user's GPG key list

-- Add is_system_key column to gpg_keys table
ALTER TABLE gpg_keys ADD COLUMN is_system_key BOOLEAN NOT NULL DEFAULT FALSE;

-- Add private_key column for system keys (encrypted, only for system keys)
-- This column stores the private key for signing commits via WebIDE/API
ALTER TABLE gpg_keys ADD COLUMN private_key_encrypted TEXT;

-- Index for quickly finding system keys
CREATE INDEX idx_gpg_keys_system ON gpg_keys(user_id, is_system_key) WHERE is_system_key = true;

-- Add comment for documentation
COMMENT ON COLUMN gpg_keys.is_system_key IS 'True if this is a system-generated key for WebIDE/API signing (hidden from user list)';
COMMENT ON COLUMN gpg_keys.private_key_encrypted IS 'Encrypted private key (only for system keys, used for WebIDE/API signing)';
