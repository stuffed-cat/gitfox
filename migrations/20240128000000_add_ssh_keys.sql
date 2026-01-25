6-- SSH Keys table for storing user SSH public keys
CREATE TABLE ssh_keys (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    key_type VARCHAR(50) NOT NULL,  -- ssh-rsa, ssh-ed25519, ecdsa-sha2-nistp256, etc.
    public_key TEXT NOT NULL,
    fingerprint VARCHAR(255) NOT NULL UNIQUE,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ssh_keys_user ON ssh_keys(user_id);
CREATE INDEX idx_ssh_keys_fingerprint ON ssh_keys(fingerprint);

-- Access logs for SSH operations
CREATE TABLE ssh_access_logs (
    id BIGSERIAL PRIMARY KEY,
    ssh_key_id BIGINT REFERENCES ssh_keys(id) ON DELETE SET NULL,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id BIGINT REFERENCES projects(id) ON DELETE SET NULL,
    action VARCHAR(50) NOT NULL,  -- git-upload-pack, git-receive-pack
    repository_path VARCHAR(255) NOT NULL,
    client_ip VARCHAR(45),
    success BOOLEAN NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ssh_access_logs_user ON ssh_access_logs(user_id);
CREATE INDEX idx_ssh_access_logs_key ON ssh_access_logs(ssh_key_id);
CREATE INDEX idx_ssh_access_logs_created ON ssh_access_logs(created_at);

-- Add repository path to projects for SSH access
-- This helps map namespace/project to the actual project
ALTER TABLE projects ADD COLUMN IF NOT EXISTS repository_path VARCHAR(255);

-- Create a unique index on repository_path
CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_repository_path ON projects(repository_path) WHERE repository_path IS NOT NULL;
