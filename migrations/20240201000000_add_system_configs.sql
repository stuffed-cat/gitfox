-- System configuration table (key-value store for runtime settings)
-- These settings are cached in Redis for fast access.

CREATE TABLE IF NOT EXISTS system_configs (
    key        TEXT PRIMARY KEY,
    value      JSONB NOT NULL DEFAULT '""',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default settings
INSERT INTO system_configs (key, value) VALUES
    ('site_name', '"GitFox"'),
    ('site_description', '"GitFox DevSecOps Platform"'),
    ('signup_enabled', 'true'),
    ('require_email_confirmation', 'false'),
    ('default_project_visibility', '"private"'),
    ('max_attachment_size_mb', '10'),
    ('gravatar_enabled', 'false'),
    ('home_page_url', '""'),
    ('after_sign_in_path', '"/"'),
    ('terms_of_service', '""'),
    ('user_default_role', '"developer"')
ON CONFLICT (key) DO NOTHING;
