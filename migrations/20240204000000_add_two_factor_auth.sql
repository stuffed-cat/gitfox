-- Add two-factor authentication support

-- Add 2FA enabled flag to users
ALTER TABLE users ADD COLUMN two_factor_enabled BOOLEAN NOT NULL DEFAULT false;

-- TOTP (Time-based One-Time Password) secrets for authenticator apps
CREATE TABLE user_totp (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    secret VARCHAR(255) NOT NULL, -- Base32 encoded TOTP secret
    enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    verified_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ
);

CREATE INDEX idx_user_totp_user_id ON user_totp(user_id);

-- WebAuthn credentials (Passkeys/FIDO2)
CREATE TABLE user_webauthn_credentials (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id BYTEA NOT NULL UNIQUE, -- WebAuthn credential ID
    public_key BYTEA NOT NULL, -- COSE public key
    counter BIGINT NOT NULL DEFAULT 0, -- Signature counter for replay protection
    name VARCHAR(100) NOT NULL, -- User-friendly name (e.g., "YubiKey", "TouchID")
    aaguid BYTEA, -- Authenticator Attestation GUID
    transports TEXT[], -- Supported transports: "usb", "nfc", "ble", "internal"
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX idx_user_webauthn_user_id ON user_webauthn_credentials(user_id);
CREATE INDEX idx_user_webauthn_credential_id ON user_webauthn_credentials(credential_id);

-- Recovery codes for account recovery
CREATE TABLE user_recovery_codes (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash VARCHAR(255) NOT NULL, -- Hashed recovery code
    used BOOLEAN NOT NULL DEFAULT false,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_recovery_codes_user_id ON user_recovery_codes(user_id);

-- Table to track 2FA verification attempts (for rate limiting)
CREATE TABLE two_factor_attempts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ip_address INET NOT NULL,
    success BOOLEAN NOT NULL,
    method VARCHAR(50) NOT NULL, -- 'totp', 'webauthn', 'recovery'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_two_factor_attempts_user_id ON two_factor_attempts(user_id);
CREATE INDEX idx_two_factor_attempts_created_at ON two_factor_attempts(created_at);
