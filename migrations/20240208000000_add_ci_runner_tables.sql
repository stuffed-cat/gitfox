-- 添加 runners 表
CREATE TABLE runners (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    token VARCHAR(255) NOT NULL UNIQUE,
    tags TEXT[] DEFAULT '{}',
    executor VARCHAR(50) NOT NULL DEFAULT 'shell',
    status VARCHAR(50) NOT NULL DEFAULT 'offline', -- online, offline, paused
    last_contact TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 添加 jobs 表
CREATE TABLE jobs (
    id BIGSERIAL PRIMARY KEY,
    pipeline_id BIGINT NOT NULL,
    project_id BIGINT NOT NULL,
    runner_id BIGINT,
    name VARCHAR(255) NOT NULL,
    stage VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- pending, running, success, failed, canceled, skipped
    config JSONB NOT NULL, -- 完整的 job 配置
    artifacts_path TEXT,
    coverage DECIMAL(5,2),
    allow_failure BOOLEAN DEFAULT FALSE,
    when_condition VARCHAR(50) DEFAULT 'on_success',
    retry_count INTEGER DEFAULT 0,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (pipeline_id) REFERENCES pipelines(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (runner_id) REFERENCES runners(id) ON DELETE SET NULL
);

-- 添加 job_logs 表
CREATE TABLE job_logs (
    id BIGSERIAL PRIMARY KEY,
    job_id BIGINT NOT NULL,
    output TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (job_id) REFERENCES jobs(id) ON DELETE CASCADE
);

-- 添加索引
CREATE INDEX idx_runners_token ON runners(token);
CREATE INDEX idx_runners_status ON runners(status);
CREATE INDEX idx_jobs_pipeline_id ON jobs(pipeline_id);
CREATE INDEX idx_jobs_runner_id ON jobs(runner_id);
CREATE INDEX idx_jobs_status ON jobs(status);
CREATE INDEX idx_job_logs_job_id ON job_logs(job_id);
CREATE INDEX idx_job_logs_created_at ON job_logs(created_at);
