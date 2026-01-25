
-- 1. 先删除所有外键约束和旧表
DROP TABLE IF EXISTS webhook_deliveries CASCADE;
DROP TABLE IF EXISTS webhooks CASCADE;
DROP TABLE IF EXISTS pipeline_configs CASCADE;
DROP TABLE IF EXISTS pipeline_job_logs CASCADE;
DROP TABLE IF EXISTS pipeline_jobs CASCADE;
DROP TABLE IF EXISTS pipelines CASCADE;
DROP TABLE IF EXISTS merge_request_reviews CASCADE;
DROP TABLE IF EXISTS merge_request_comments CASCADE;
DROP TABLE IF EXISTS merge_requests CASCADE;
DROP TABLE IF EXISTS releases CASCADE;
DROP TABLE IF EXISTS tags CASCADE;
DROP TABLE IF EXISTS commits CASCADE;
DROP TABLE IF EXISTS branch_protection_rules CASCADE;
DROP TABLE IF EXISTS branches CASCADE;
DROP TABLE IF EXISTS project_members CASCADE;
DROP TABLE IF EXISTS projects CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- 2. 重建Users表，使用BIGSERIAL
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(100),
    avatar_url TEXT,
    role user_role NOT NULL DEFAULT 'developer',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);

-- 3. 重建Projects表，删除slug，使用BIGSERIAL，添加(owner_id, name)唯一约束
CREATE TABLE projects (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    visibility project_visibility NOT NULL DEFAULT 'private',
    owner_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    default_branch VARCHAR(100) NOT NULL DEFAULT 'main',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(owner_id, name)
);

CREATE INDEX idx_projects_owner ON projects(owner_id);
CREATE INDEX idx_projects_name ON projects(name);

-- 4. Project members
CREATE TABLE project_members (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role member_role NOT NULL DEFAULT 'developer',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, user_id)
);

CREATE INDEX idx_project_members_project ON project_members(project_id);
CREATE INDEX idx_project_members_user ON project_members(user_id);

-- 5. Branches
CREATE TABLE branches (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    commit_sha VARCHAR(40) NOT NULL,
    is_protected BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, name)
);

CREATE INDEX idx_branches_project ON branches(project_id);

-- 6. Branch protection rules
CREATE TABLE branch_protection_rules (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    branch_pattern VARCHAR(255) NOT NULL,
    require_review BOOLEAN NOT NULL DEFAULT false,
    required_reviewers INTEGER NOT NULL DEFAULT 1,
    require_ci_pass BOOLEAN NOT NULL DEFAULT false,
    allow_force_push BOOLEAN NOT NULL DEFAULT false,
    allow_deletion BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_branch_protection_project ON branch_protection_rules(project_id);

-- 7. Commits
CREATE TABLE commits (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    sha VARCHAR(40) NOT NULL,
    message TEXT NOT NULL,
    author_name VARCHAR(255) NOT NULL,
    author_email VARCHAR(255) NOT NULL,
    authored_at TIMESTAMPTZ NOT NULL,
    committer_name VARCHAR(255) NOT NULL,
    committer_email VARCHAR(255) NOT NULL,
    committed_at TIMESTAMPTZ NOT NULL,
    parent_shas TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, sha)
);

CREATE INDEX idx_commits_project ON commits(project_id);
CREATE INDEX idx_commits_sha ON commits(sha);

-- 8. Tags
CREATE TABLE tags (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    commit_sha VARCHAR(40) NOT NULL,
    message TEXT,
    tagger_name VARCHAR(255),
    tagger_email VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, name)
);

CREATE INDEX idx_tags_project ON tags(project_id);

-- 9. Releases
CREATE TABLE releases (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tag_name VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    is_prerelease BOOLEAN NOT NULL DEFAULT false,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, tag_name)
);

CREATE INDEX idx_releases_project ON releases(project_id);

-- 10. Merge requests
CREATE TABLE merge_requests (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    iid BIGINT NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    source_branch VARCHAR(255) NOT NULL,
    target_branch VARCHAR(255) NOT NULL,
    status merge_request_status NOT NULL DEFAULT 'open',
    author_id BIGINT NOT NULL REFERENCES users(id),
    assignee_id BIGINT REFERENCES users(id),
    merged_by BIGINT REFERENCES users(id),
    merged_at TIMESTAMPTZ,
    closed_by BIGINT REFERENCES users(id),
    closed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, iid)
);

CREATE INDEX idx_merge_requests_project ON merge_requests(project_id);
CREATE INDEX idx_merge_requests_author ON merge_requests(author_id);
CREATE INDEX idx_merge_requests_status ON merge_requests(status);

-- 11. Merge request comments
CREATE TABLE merge_request_comments (
    id BIGSERIAL PRIMARY KEY,
    merge_request_id BIGINT NOT NULL REFERENCES merge_requests(id) ON DELETE CASCADE,
    author_id BIGINT NOT NULL REFERENCES users(id),
    content TEXT NOT NULL,
    line_number INTEGER,
    file_path TEXT,
    parent_id BIGINT REFERENCES merge_request_comments(id),
    is_resolved BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_mr_comments_mr ON merge_request_comments(merge_request_id);

-- 12. Merge request reviews
CREATE TABLE merge_request_reviews (
    id BIGSERIAL PRIMARY KEY,
    merge_request_id BIGINT NOT NULL REFERENCES merge_requests(id) ON DELETE CASCADE,
    reviewer_id BIGINT NOT NULL REFERENCES users(id),
    status review_status NOT NULL DEFAULT 'pending',
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(merge_request_id, reviewer_id)
);

CREATE INDEX idx_mr_reviews_mr ON merge_request_reviews(merge_request_id);

-- 13. Pipelines
CREATE TABLE pipelines (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    ref_name VARCHAR(255) NOT NULL,
    commit_sha VARCHAR(40) NOT NULL,
    status pipeline_status NOT NULL DEFAULT 'pending',
    trigger_type pipeline_trigger_type NOT NULL,
    triggered_by BIGINT REFERENCES users(id),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    duration_seconds INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pipelines_project ON pipelines(project_id);
CREATE INDEX idx_pipelines_status ON pipelines(status);

-- 14. Pipeline jobs
CREATE TABLE pipeline_jobs (
    id BIGSERIAL PRIMARY KEY,
    pipeline_id BIGINT NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    stage VARCHAR(100) NOT NULL,
    status pipeline_status NOT NULL DEFAULT 'pending',
    runner_id BIGINT,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    duration_seconds INTEGER,
    allow_failure BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pipeline_jobs_pipeline ON pipeline_jobs(pipeline_id);

-- 15. Pipeline job logs
CREATE TABLE pipeline_job_logs (
    id BIGSERIAL PRIMARY KEY,
    job_id BIGINT NOT NULL REFERENCES pipeline_jobs(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pipeline_job_logs_job ON pipeline_job_logs(job_id);

-- 16. Pipeline configs
CREATE TABLE pipeline_configs (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id)
);

-- 17. Webhooks
CREATE TABLE webhooks (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    secret TEXT,
    events TEXT[] NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhooks_project ON webhooks(project_id);

-- 18. Webhook deliveries
CREATE TABLE webhook_deliveries (
    id BIGSERIAL PRIMARY KEY,
    webhook_id BIGINT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    response_status INTEGER,
    response_body TEXT,
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_deliveries_webhook ON webhook_deliveries(webhook_id);
CREATE INDEX idx_webhook_deliveries_created ON webhook_deliveries(created_at);

-- 重建triggers
DROP TRIGGER IF EXISTS update_users_updated_at ON users;
DROP TRIGGER IF EXISTS update_projects_updated_at ON projects;
DROP TRIGGER IF EXISTS update_branches_updated_at ON branches;
DROP TRIGGER IF EXISTS update_releases_updated_at ON releases;
DROP TRIGGER IF EXISTS update_merge_requests_updated_at ON merge_requests;
DROP TRIGGER IF EXISTS update_mr_comments_updated_at ON merge_request_comments;
DROP TRIGGER IF EXISTS update_mr_reviews_updated_at ON merge_request_reviews;
DROP TRIGGER IF EXISTS update_pipelines_updated_at ON pipelines;
DROP TRIGGER IF EXISTS update_pipeline_jobs_updated_at ON pipeline_jobs;
DROP TRIGGER IF EXISTS update_pipeline_configs_updated_at ON pipeline_configs;
DROP TRIGGER IF EXISTS update_webhooks_updated_at ON webhooks;

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_projects_updated_at BEFORE UPDATE ON projects FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_branches_updated_at BEFORE UPDATE ON branches FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_releases_updated_at BEFORE UPDATE ON releases FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_merge_requests_updated_at BEFORE UPDATE ON merge_requests FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_mr_comments_updated_at BEFORE UPDATE ON merge_request_comments FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_mr_reviews_updated_at BEFORE UPDATE ON merge_request_reviews FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_pipelines_updated_at BEFORE UPDATE ON pipelines FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_pipeline_jobs_updated_at BEFORE UPDATE ON pipeline_jobs FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_pipeline_configs_updated_at BEFORE UPDATE ON pipeline_configs FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_webhooks_updated_at BEFORE UPDATE ON webhooks FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
