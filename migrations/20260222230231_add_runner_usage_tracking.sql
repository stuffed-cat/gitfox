-- Add runner usage tracking table
CREATE TABLE user_runner_usage (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    job_id BIGINT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    minutes_used INTEGER NOT NULL DEFAULT 0, -- 使用的分钟数
    month VARCHAR(7) NOT NULL, -- 格式: YYYY-MM，用于按月统计
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, job_id) -- 每个 job 只记录一次
);

-- Create indexes for efficient queries
CREATE INDEX idx_user_runner_usage_user_month ON user_runner_usage(user_id, month);
CREATE INDEX idx_user_runner_usage_user_id ON user_runner_usage(user_id);
CREATE INDEX idx_user_runner_usage_month ON user_runner_usage(month);

-- Add default CI/CD configurations
INSERT INTO system_configs (key, value, updated_at) VALUES
    ('pro_runner_quota_minutes', '0', NOW()),  -- 0 = 不限制
    ('regular_runner_quota_minutes', '2000', NOW())  -- 非 PRO 用户每月 2000 分钟
ON CONFLICT (key) DO NOTHING;
