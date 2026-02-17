-- 添加 runner 作用域支持
-- scope: 'system' (系统级-管理员), 'user' (用户级), 'namespace' (组群级), 'project' (项目级)
ALTER TABLE runners ADD COLUMN scope VARCHAR(20) NOT NULL DEFAULT 'system';
ALTER TABLE runners ADD COLUMN user_id BIGINT REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE runners ADD COLUMN namespace_id BIGINT REFERENCES namespaces(id) ON DELETE CASCADE;
ALTER TABLE runners ADD COLUMN project_id BIGINT REFERENCES projects(id) ON DELETE CASCADE;
ALTER TABLE runners ADD COLUMN description TEXT;
ALTER TABLE runners ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE runners ADD COLUMN run_untagged BOOLEAN NOT NULL DEFAULT TRUE; -- 是否可以运行没有标签的作业
ALTER TABLE runners ADD COLUMN locked BOOLEAN NOT NULL DEFAULT FALSE; -- 是否锁定到当前作用域
ALTER TABLE runners ADD COLUMN maximum_timeout INTEGER; -- 最大超时时间（秒）

-- 添加约束：确保 scope 和对应的 ID 一致
ALTER TABLE runners ADD CONSTRAINT check_runner_scope 
    CHECK (
        (scope = 'system' AND user_id IS NULL AND namespace_id IS NULL AND project_id IS NULL) OR
        (scope = 'user' AND user_id IS NOT NULL AND namespace_id IS NULL AND project_id IS NULL) OR
        (scope = 'namespace' AND namespace_id IS NOT NULL AND project_id IS NULL) OR
        (scope = 'project' AND project_id IS NOT NULL)
    );

-- 添加索引
CREATE INDEX idx_runners_scope ON runners(scope);
CREATE INDEX idx_runners_user_id ON runners(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_runners_namespace_id ON runners(namespace_id) WHERE namespace_id IS NOT NULL;
CREATE INDEX idx_runners_project_id ON runners(project_id) WHERE project_id IS NOT NULL;
CREATE INDEX idx_runners_is_active ON runners(is_active);
CREATE INDEX idx_runners_status_active ON runners(status, is_active);

-- 为现有 runners 设置默认值（如果有的话）
UPDATE runners SET scope = 'system' WHERE scope IS NULL;
