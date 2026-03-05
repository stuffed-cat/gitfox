-- Add tables for project settings features
-- 1. CI/CD Variables - Store encrypted variables for CI/CD pipelines
-- 2. Pipeline Triggers - Allow external systems to trigger pipelines
-- 3. Deploy Keys - SSH keys for read-only or read-write repository access
-- 4. Project Access Tokens - Scoped access tokens for projects

-- =========================================
-- 1. CI/CD Variables
-- =========================================
CREATE TABLE IF NOT EXISTS ci_variables (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    value_encrypted TEXT NOT NULL,           -- AES encrypted value
    protected BOOLEAN NOT NULL DEFAULT false, -- Only available in protected branches
    masked BOOLEAN NOT NULL DEFAULT false,    -- Masked in logs
    file BOOLEAN NOT NULL DEFAULT false,      -- Expose as file instead of env var
    environment_scope VARCHAR(255) NOT NULL DEFAULT '*',  -- Environment scope
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, key, environment_scope)
);

CREATE INDEX idx_ci_variables_project ON ci_variables(project_id);

-- =========================================
-- 2. Pipeline Triggers
-- =========================================
CREATE TABLE IF NOT EXISTS pipeline_triggers (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    description VARCHAR(255),
    token_hash VARCHAR(64) NOT NULL UNIQUE,  -- SHA256 hash of trigger token
    token_preview VARCHAR(8) NOT NULL,       -- First 8 chars for display
    created_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pipeline_triggers_project ON pipeline_triggers(project_id);
CREATE INDEX idx_pipeline_triggers_token ON pipeline_triggers(token_hash);

-- =========================================
-- 3. Deploy Keys
-- =========================================
CREATE TABLE IF NOT EXISTS deploy_keys (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    key_type VARCHAR(50) NOT NULL,           -- ssh-rsa, ssh-ed25519, etc.
    public_key TEXT NOT NULL,
    fingerprint VARCHAR(100) NOT NULL,
    can_push BOOLEAN NOT NULL DEFAULT false, -- Read-only by default
    created_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, fingerprint)
);

CREATE INDEX idx_deploy_keys_project ON deploy_keys(project_id);
CREATE INDEX idx_deploy_keys_fingerprint ON deploy_keys(fingerprint);

-- =========================================
-- 4. Project Access Tokens
-- =========================================
CREATE TABLE IF NOT EXISTS project_access_tokens (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    token_hash VARCHAR(64) NOT NULL UNIQUE,  -- SHA256 hash
    token_preview VARCHAR(20) NOT NULL,      -- gitfox-pat_xxx... for display
    scopes TEXT[] NOT NULL DEFAULT '{}',     -- Array of scope strings
    role VARCHAR(50) NOT NULL DEFAULT 'developer', -- maintainer, developer, reporter
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_project_access_tokens_project ON project_access_tokens(project_id);
CREATE INDEX idx_project_access_tokens_hash ON project_access_tokens(token_hash);
