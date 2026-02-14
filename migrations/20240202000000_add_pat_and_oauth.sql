-- ============================================================================
-- Personal Access Tokens (PAT)
-- Long-lived tokens for API/Git access (like GitHub PAT or GitLab PAT)
-- ============================================================================
CREATE TABLE personal_access_tokens (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    -- Token hash (we never store the raw token, only SHA256 hash)
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    -- Token prefix for identification (first 8 chars, e.g., "gfpat_ab")
    token_prefix VARCHAR(20) NOT NULL,
    -- Scopes: read_api, write_api, read_repository, write_repository, read_user, sudo, etc.
    scopes TEXT[] NOT NULL DEFAULT '{}',
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Each user can only have one token with the same name
    CONSTRAINT unique_user_pat_name UNIQUE(user_id, name)
);

CREATE INDEX idx_pat_user ON personal_access_tokens(user_id);
CREATE INDEX idx_pat_token_hash ON personal_access_tokens(token_hash);
CREATE INDEX idx_pat_token_prefix ON personal_access_tokens(token_prefix);
CREATE INDEX idx_pat_active ON personal_access_tokens(user_id) WHERE revoked_at IS NULL;

-- ============================================================================
-- OAuth Applications (GitFox as OAuth Provider)
-- Third-party apps can register here to authenticate users via GitFox
-- ============================================================================
CREATE TABLE oauth_applications (
    id BIGSERIAL PRIMARY KEY,
    owner_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    -- Unique client_id (public identifier)
    uid VARCHAR(64) NOT NULL UNIQUE,
    -- Hashed client_secret (never store raw)
    secret_hash VARCHAR(255) NOT NULL,
    -- Redirect URIs as JSON array (can have multiple)
    redirect_uris JSONB NOT NULL DEFAULT '[]',
    -- Scopes this app can request
    scopes JSONB NOT NULL DEFAULT '["read_user"]',
    -- App metadata
    description TEXT,
    homepage_url TEXT,
    logo_url TEXT,
    -- Confidential clients (servers) vs public clients (SPAs, mobile)
    confidential BOOLEAN NOT NULL DEFAULT true,
    -- Trusted apps skip user consent screen
    trusted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_oauth_apps_owner ON oauth_applications(owner_id);
CREATE INDEX idx_oauth_apps_uid ON oauth_applications(uid);

-- ============================================================================
-- OAuth Authorization Codes
-- Short-lived codes exchanged for access tokens (Authorization Code flow)
-- ============================================================================
CREATE TABLE oauth_authorization_codes (
    id BIGSERIAL PRIMARY KEY,
    application_id BIGINT NOT NULL REFERENCES oauth_applications(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The authorization code (hashed)
    code_hash VARCHAR(255) NOT NULL UNIQUE,
    -- Redirect URI used in this request (must match on token exchange)
    redirect_uri TEXT NOT NULL,
    -- Granted scopes
    scopes JSONB NOT NULL DEFAULT '[]',
    -- PKCE support (Proof Key for Code Exchange)
    code_challenge VARCHAR(128),
    code_challenge_method VARCHAR(10), -- 'plain' or 'S256'
    -- Expiration (typically 10 minutes)
    expires_at TIMESTAMPTZ NOT NULL,
    -- Used flag (codes are single-use)
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_oauth_codes_app ON oauth_authorization_codes(application_id);
CREATE INDEX idx_oauth_codes_hash ON oauth_authorization_codes(code_hash);

-- ============================================================================
-- OAuth Access Tokens (issued by GitFox as Provider)
-- Tokens issued to applications on behalf of users
-- ============================================================================
CREATE TABLE oauth_access_tokens (
    id BIGSERIAL PRIMARY KEY,
    application_id BIGINT NOT NULL REFERENCES oauth_applications(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Hashed access token
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    -- Hashed refresh token (optional)
    refresh_token_hash VARCHAR(255) UNIQUE,
    -- Granted scopes
    scopes JSONB NOT NULL DEFAULT '[]',
    -- Expiration
    expires_at TIMESTAMPTZ NOT NULL,
    -- Revocation
    revoked_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_oauth_tokens_app ON oauth_access_tokens(application_id);
CREATE INDEX idx_oauth_tokens_user ON oauth_access_tokens(user_id);
CREATE INDEX idx_oauth_tokens_hash ON oauth_access_tokens(token_hash);
CREATE INDEX idx_oauth_tokens_refresh ON oauth_access_tokens(refresh_token_hash) WHERE refresh_token_hash IS NOT NULL;

-- ============================================================================
-- External OAuth Providers (GitFox as OAuth Client)
-- Dynamically configured external identity providers (GitHub, Google, etc.)
-- Admin can add/remove providers at runtime
-- ============================================================================
CREATE TABLE oauth_providers (
    id BIGSERIAL PRIMARY KEY,
    -- Unique provider slug (e.g., "github", "google", "my-company-sso")
    name VARCHAR(100) NOT NULL UNIQUE,
    -- Display name shown to users (e.g., "GitHub", "Google", "Company SSO")
    display_name VARCHAR(255) NOT NULL,
    -- Provider type for built-in handling: "github", "gitlab", "google", "oidc", "oauth2"
    provider_type VARCHAR(50) NOT NULL DEFAULT 'oauth2',
    -- OAuth endpoints
    authorization_url TEXT NOT NULL,
    token_url TEXT NOT NULL,
    userinfo_url TEXT,
    -- OIDC specific (OpenID Connect)
    issuer_url TEXT,
    jwks_uri TEXT,
    -- Client credentials
    client_id VARCHAR(255) NOT NULL,
    -- Encrypted client secret (use AES-256-GCM with OAUTH_SECRET_KEY)
    client_secret_encrypted TEXT NOT NULL,
    -- Default scopes to request (JSON array)
    scopes JSONB NOT NULL DEFAULT '["openid", "email", "profile"]',
    -- Field mappings: how to extract user info from provider response
    -- e.g., {"id": "sub", "email": "email", "username": "preferred_username", "name": "name", "avatar": "picture"}
    field_mappings JSONB NOT NULL DEFAULT '{"id": "id", "email": "email", "username": "login", "name": "name", "avatar": "avatar_url"}',
    -- Whether this provider is enabled
    enabled BOOLEAN NOT NULL DEFAULT true,
    -- Whether to auto-create users on first login via this provider
    allow_signup BOOLEAN NOT NULL DEFAULT false,
    -- Whether to auto-link existing users by email match
    auto_link_by_email BOOLEAN NOT NULL DEFAULT true,
    -- Provider icon (URL or icon name like "github", "google")
    icon VARCHAR(255),
    -- Sort order for display on login page
    sort_order INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_oauth_provider_name ON oauth_providers(name);
CREATE INDEX idx_oauth_provider_enabled ON oauth_providers(enabled) WHERE enabled = true;

-- ============================================================================
-- OAuth Identities (links external provider accounts to local users)
-- ============================================================================
CREATE TABLE oauth_identities (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_id BIGINT NOT NULL REFERENCES oauth_providers(id) ON DELETE CASCADE,
    -- Provider-specific user ID (e.g., GitHub numeric user ID)
    external_uid VARCHAR(255) NOT NULL,
    -- External username/login (for display)
    external_username VARCHAR(255),
    -- External email (may differ from local user's email)
    external_email VARCHAR(255),
    -- External avatar URL
    external_avatar_url TEXT,
    -- Encrypted access token for provider API calls (optional)
    access_token_encrypted TEXT,
    -- Encrypted refresh token (optional)
    refresh_token_encrypted TEXT,
    token_expires_at TIMESTAMPTZ,
    -- Raw profile data from provider (for debugging)
    raw_info JSONB,
    last_sign_in_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Each provider can only be linked once per user
    CONSTRAINT unique_user_provider UNIQUE(user_id, provider_id),
    -- Each external UID can only be linked to one local user per provider
    CONSTRAINT unique_provider_external_uid UNIQUE(provider_id, external_uid)
);

CREATE INDEX idx_oauth_identity_user ON oauth_identities(user_id);
CREATE INDEX idx_oauth_identity_provider ON oauth_identities(provider_id);
CREATE INDEX idx_oauth_identity_lookup ON oauth_identities(provider_id, external_uid);

-- ============================================================================
-- Comments for documentation
-- ============================================================================
COMMENT ON TABLE personal_access_tokens IS 'Personal Access Tokens for API and Git authentication.
Token format: gfpat_{random_32_chars}
Available scopes:
- api: Full API access (read + write)
- read_api: Read-only API access
- read_user: Read user profile info
- read_repository: Read repository content
- write_repository: Write repository content (push)
- create_runner: Register CI/CD runners
- sudo: Admin impersonation (admin only)';

COMMENT ON TABLE oauth_providers IS 'External OAuth/OIDC identity providers (GitFox as OAuth Client).
provider_type determines how to handle the OAuth flow:
- github: GitHub OAuth (uses GitHub-specific endpoints and field mappings)
- gitlab: GitLab OAuth (uses GitLab-specific endpoints)
- google: Google OAuth (uses Google-specific endpoints)
- oidc: Generic OpenID Connect (auto-discover from issuer_url)
- oauth2: Generic OAuth2 (manual endpoint configuration)';

COMMENT ON TABLE oauth_applications IS 'OAuth applications registered with GitFox (GitFox as OAuth Provider).
These apps can authenticate users via GitFox using OAuth 2.0 Authorization Code flow.
Supports PKCE for public clients (SPAs, mobile apps).';

-- ============================================================================
-- Add PAT & OAuth related system configs (dynamic settings)
-- ============================================================================
INSERT INTO system_configs (key, value) VALUES
    -- PAT settings
    ('pat_enabled', 'true'),
    ('pat_default_expiry_days', '90'),
    ('pat_max_expiry_days', '365'),
    -- OAuth Provider settings (GitFox as OAuth Provider)
    ('oauth_applications_enabled', 'true'),
    ('oauth_access_token_expiry_hours', '2'),
    ('oauth_refresh_token_expiry_days', '14'),
    -- OAuth Client settings (external providers)
    ('oauth_external_providers_enabled', 'true'),
    ('oauth_auto_link_by_email', 'true'),
    ('oauth_allow_signup_from_external', 'false'),
    ('oauth_block_auto_created_users', 'false')
ON CONFLICT (key) DO NOTHING;

-- ============================================================================
-- 预置常用 OAuth 提供商配置（URL 和字段映射）
-- 凭证通过 .env 配置，这里只存储端点和字段映射
-- client_id/client_secret_encrypted 为空字符串表示从 .env 读取
-- ============================================================================
INSERT INTO oauth_providers (name, display_name, provider_type, authorization_url, token_url, userinfo_url, client_id, client_secret_encrypted, scopes, field_mappings, icon, enabled, sort_order, created_at, updated_at) VALUES
-- GitHub
('github', 'GitHub', 'github',
 'https://github.com/login/oauth/authorize',
 'https://github.com/login/oauth/access_token',
 'https://api.github.com/user',
 '', '',
 '["user:email"]',
 '{"id": "id", "username": "login", "email": "email", "name": "name", "avatar": "avatar_url"}',
 'github', true, 1, NOW(), NOW()),

-- GitLab (默认 gitlab.com，可通过管理界面修改为自托管地址)
('gitlab', 'GitLab', 'gitlab',
 'https://gitlab.com/oauth/authorize',
 'https://gitlab.com/oauth/token',
 'https://gitlab.com/api/v4/user',
 '', '',
 '["read_user"]',
 '{"id": "id", "username": "username", "email": "email", "name": "name", "avatar": "avatar_url"}',
 'gitlab', true, 2, NOW(), NOW()),

-- Google
('google', 'Google', 'google',
 'https://accounts.google.com/o/oauth2/v2/auth',
 'https://oauth2.googleapis.com/token',
 'https://www.googleapis.com/oauth2/v2/userinfo',
 '', '',
 '["openid", "email", "profile"]',
 '{"id": "id", "username": "name", "email": "email", "name": "name", "avatar": "picture"}',
 'google', true, 3, NOW(), NOW()),

-- Azure AD (Microsoft)
('azure_ad', 'Microsoft', 'azure_ad',
 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize',
 'https://login.microsoftonline.com/common/oauth2/v2.0/token',
 'https://graph.microsoft.com/v1.0/me',
 '', '',
 '["openid", "email", "profile"]',
 '{"id": "id", "username": "userPrincipalName", "email": "mail", "name": "displayName", "avatar": null}',
 'microsoft', true, 4, NOW(), NOW()),

-- Bitbucket
('bitbucket', 'Bitbucket', 'bitbucket',
 'https://bitbucket.org/site/oauth2/authorize',
 'https://bitbucket.org/site/oauth2/access_token',
 'https://api.bitbucket.org/2.0/user',
 '', '',
 '["account"]',
 '{"id": "uuid", "username": "username", "email": "email", "name": "display_name", "avatar": "links.avatar.href"}',
 'bitbucket', true, 5, NOW(), NOW())
ON CONFLICT (name) DO NOTHING;
