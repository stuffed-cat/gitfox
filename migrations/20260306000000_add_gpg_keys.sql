-- GPG Keys table for storing user GPG public keys
-- Used for commit signature verification
CREATE TABLE gpg_keys (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Primary key ID (64-bit key ID in hex format)
    primary_key_id VARCHAR(40) NOT NULL,
    -- Full fingerprint of the primary key
    fingerprint VARCHAR(64) NOT NULL UNIQUE,
    -- ASCII-armored public key
    public_key TEXT NOT NULL,
    -- Key algorithm (RSA, DSA, ECDSA, EdDSA, etc.)
    key_algorithm VARCHAR(20) NOT NULL,
    -- Key size in bits (e.g., 4096 for RSA)
    key_size INTEGER,
    -- Associated email addresses (stored as JSON array)
    emails TEXT[] NOT NULL DEFAULT '{}',
    -- Whether this key can sign
    can_sign BOOLEAN NOT NULL DEFAULT TRUE,
    -- Whether this key can encrypt
    can_encrypt BOOLEAN NOT NULL DEFAULT FALSE,
    -- Whether this key can certify other keys
    can_certify BOOLEAN NOT NULL DEFAULT FALSE,
    -- Key creation date from the GPG key itself
    key_created_at TIMESTAMPTZ,
    -- Key expiration date from the GPG key itself (NULL if no expiration)
    key_expires_at TIMESTAMPTZ,
    -- Whether the key is verified (all emails belong to the user)
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    -- Whether the key is revoked
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    -- Last time this key was used to verify a signature
    last_used_at TIMESTAMPTZ,
    -- Database timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient lookups
CREATE INDEX idx_gpg_keys_user ON gpg_keys(user_id);
CREATE INDEX idx_gpg_keys_primary_key_id ON gpg_keys(primary_key_id);
CREATE INDEX idx_gpg_keys_fingerprint ON gpg_keys(fingerprint);
-- GIN index for searching by email
CREATE INDEX idx_gpg_keys_emails ON gpg_keys USING GIN(emails);

-- GPG Key subkeys table
-- Some GPG keys have separate subkeys for signing
CREATE TABLE gpg_key_subkeys (
    id BIGSERIAL PRIMARY KEY,
    gpg_key_id BIGINT NOT NULL REFERENCES gpg_keys(id) ON DELETE CASCADE,
    -- Subkey ID (64-bit key ID in hex format)
    key_id VARCHAR(40) NOT NULL,
    -- Subkey fingerprint
    fingerprint VARCHAR(64) NOT NULL,
    -- Subkey algorithm
    key_algorithm VARCHAR(20) NOT NULL,
    -- Subkey size in bits
    key_size INTEGER,
    -- Whether this subkey can sign
    can_sign BOOLEAN NOT NULL DEFAULT FALSE,
    -- Whether this subkey can encrypt
    can_encrypt BOOLEAN NOT NULL DEFAULT FALSE,
    -- Subkey creation date
    key_created_at TIMESTAMPTZ,
    -- Subkey expiration date
    key_expires_at TIMESTAMPTZ,
    -- Whether the subkey is revoked
    revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gpg_key_subkeys_gpg_key ON gpg_key_subkeys(gpg_key_id);
CREATE INDEX idx_gpg_key_subkeys_key_id ON gpg_key_subkeys(key_id);
CREATE INDEX idx_gpg_key_subkeys_fingerprint ON gpg_key_subkeys(fingerprint);

-- GPG signature cache table
-- Cache commit signature verification results for performance
CREATE TABLE gpg_signatures (
    id BIGSERIAL PRIMARY KEY,
    -- Project ID (for scoping cached results)
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    -- Commit SHA
    commit_sha VARCHAR(64) NOT NULL,
    -- GPG key that created the signature (NULL if key not in our system)
    gpg_key_id BIGINT REFERENCES gpg_keys(id) ON DELETE SET NULL,
    -- GPG key subkey ID if signed by a subkey
    gpg_key_subkey_id BIGINT REFERENCES gpg_key_subkeys(id) ON DELETE SET NULL,
    -- Signer key ID from the signature (for display even if key not in system)
    signer_key_id VARCHAR(40) NOT NULL,
    -- Verification status: 'verified', 'unverified', 'bad_email', 'unknown_key', 'bad_signature', 'expired_key', 'revoked_key'
    verification_status VARCHAR(20) NOT NULL,
    -- Additional verification message
    verification_message TEXT,
    -- Signer's email from the commit
    signer_email VARCHAR(255),
    -- Signer's name from the commit
    signer_name VARCHAR(255),
    -- User ID of the signer (if key is verified and belongs to a user)
    signer_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    -- Raw signature (for debugging)
    raw_signature TEXT,
    -- Signature creation time
    signed_at TIMESTAMPTZ,
    -- Cache entry creation time
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_gpg_signatures_project_commit ON gpg_signatures(project_id, commit_sha);
CREATE INDEX idx_gpg_signatures_gpg_key ON gpg_signatures(gpg_key_id);
CREATE INDEX idx_gpg_signatures_signer ON gpg_signatures(signer_user_id);
CREATE INDEX idx_gpg_signatures_status ON gpg_signatures(verification_status);

-- Trigger to update updated_at on gpg_keys
CREATE OR REPLACE FUNCTION update_gpg_keys_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER gpg_keys_updated_at_trigger
    BEFORE UPDATE ON gpg_keys
    FOR EACH ROW
    EXECUTE FUNCTION update_gpg_keys_updated_at();

-- Add comment for documentation
COMMENT ON TABLE gpg_keys IS 'User GPG public keys for commit signature verification';
COMMENT ON TABLE gpg_key_subkeys IS 'GPG key subkeys (some keys have separate signing subkeys)';
COMMENT ON TABLE gpg_signatures IS 'Cached commit signature verification results';
COMMENT ON COLUMN gpg_keys.primary_key_id IS 'Last 16 hex characters of the fingerprint, used as key ID';
COMMENT ON COLUMN gpg_signatures.verification_status IS 'verified=valid and email matches, unverified=valid but email mismatch, unknown_key=key not in system, bad_signature=cryptographically invalid';
