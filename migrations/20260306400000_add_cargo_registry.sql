-- Cargo Registry 支持
-- 实现完整的 Cargo crates 注册表功能
-- 注意：枚举值 'cargo' 已在 20260306390000_add_cargo_package_type.sql 中添加

-- Cargo crate 元数据表
-- 存储每个 crate 版本的详细元数据
CREATE TABLE cargo_crate_metadata (
    id BIGSERIAL PRIMARY KEY,
    package_id BIGINT NOT NULL REFERENCES packages(id) ON DELETE CASCADE UNIQUE,
    
    -- 基本信息
    description TEXT,
    documentation VARCHAR(1024),  -- 文档 URL
    homepage VARCHAR(1024),       -- 主页 URL
    repository VARCHAR(1024),     -- 仓库 URL
    readme TEXT,                   -- README 内容
    readme_file VARCHAR(255),      -- README 文件名
    license VARCHAR(255),          -- 许可证标识符 (如 "MIT OR Apache-2.0")
    license_file VARCHAR(255),     -- 许可证文件名
    
    -- 关键词和分类
    keywords TEXT[],               -- 关键词数组
    categories TEXT[],             -- 分类数组
    
    -- 版本状态
    yanked BOOLEAN NOT NULL DEFAULT FALSE,  -- 是否被撤回
    
    -- Rust 版本要求
    rust_version VARCHAR(50),      -- 最低 Rust 版本要求
    
    -- 作者信息
    authors TEXT[],                -- 作者列表
    
    -- 链接（用于 build scripts）
    links VARCHAR(255),            -- 链接到的原生库
    
    -- 特性（features）
    features JSONB DEFAULT '{}'::jsonb,  -- 可选特性映射
    default_features TEXT[],       -- 默认启用的特性
    
    -- Cargo.toml 原始数据
    cargo_toml_content TEXT,       -- 完整的 Cargo.toml 内容
    
    -- 校验和
    cksum VARCHAR(64) NOT NULL,    -- SHA256 校验和
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Cargo crate 依赖表
-- 每个 crate 版本可能有多个依赖
CREATE TABLE cargo_dependencies (
    id BIGSERIAL PRIMARY KEY,
    package_id BIGINT NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
    
    -- 依赖的 crate 名称
    name VARCHAR(255) NOT NULL,
    -- 依赖包在 Cargo.toml 中的名称（可能与实际包名不同）
    explicit_name_in_toml VARCHAR(255),
    -- 版本要求（semver 字符串）
    version_req VARCHAR(255) NOT NULL,
    
    -- 依赖类型
    kind VARCHAR(20) NOT NULL DEFAULT 'normal',  -- normal, dev, build
    -- 可选依赖
    optional BOOLEAN NOT NULL DEFAULT FALSE,
    -- 默认启用的特性
    default_features BOOLEAN NOT NULL DEFAULT TRUE,
    -- 启用的特性列表
    features TEXT[],
    -- 目标平台（用于条件依赖）
    target VARCHAR(255),           -- 如 "cfg(unix)" 或 "x86_64-unknown-linux-gnu"
    -- 依赖的注册表（NULL 表示默认 crates.io 或当前注册表）
    registry VARCHAR(512),
    
    -- 为 path 和 git 依赖添加支持
    -- 这些通常只在开发时使用，发布时会被替换
    path VARCHAR(512),
    git VARCHAR(512),
    branch VARCHAR(255),
    tag VARCHAR(255),
    rev VARCHAR(64),
    
    UNIQUE(package_id, name, kind, target)
);

-- Cargo crate 所有者表
-- 记录 crate 的所有者（用户或团队）
CREATE TABLE cargo_crate_owners (
    id BIGSERIAL PRIMARY KEY,
    -- 按 namespace + crate_name 唯一标识一个 crate（跨版本共享所有者）
    namespace_id BIGINT NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    crate_name VARCHAR(255) NOT NULL,
    
    -- 所有者可以是用户或团队
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    team_id BIGINT REFERENCES namespaces(id) ON DELETE CASCADE,  -- 团队（group namespace）
    
    -- 类型
    owner_kind VARCHAR(20) NOT NULL DEFAULT 'user',  -- user 或 team
    
    -- 邀请状态
    invited_by BIGINT REFERENCES users(id),
    invited_at TIMESTAMPTZ,
    accepted_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- 确保 user_id 和 team_id 只有一个非空
    CONSTRAINT cargo_owner_type_check CHECK (
        (owner_kind = 'user' AND user_id IS NOT NULL AND team_id IS NULL) OR
        (owner_kind = 'team' AND team_id IS NOT NULL AND user_id IS NULL)
    ),
    
    -- 唯一约束：同一 crate 下的所有者不能重复
    UNIQUE(namespace_id, crate_name, user_id),
    UNIQUE(namespace_id, crate_name, team_id)
);

-- Cargo 下载统计表
-- 记录下载次数用于统计
CREATE TABLE cargo_download_stats (
    id BIGSERIAL PRIMARY KEY,
    package_id BIGINT NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
    
    -- 按日期聚合
    download_date DATE NOT NULL DEFAULT CURRENT_DATE,
    download_count BIGINT NOT NULL DEFAULT 0,
    
    UNIQUE(package_id, download_date)
);

-- Cargo API tokens 表
-- 用于 cargo login 生成的专用 tokens
CREATE TABLE cargo_tokens (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    name VARCHAR(255) NOT NULL,  -- Token 名称（用户定义）
    token_hash VARCHAR(64) NOT NULL UNIQUE,  -- SHA-256 hash
    
    -- 权限范围
    scopes TEXT[] NOT NULL DEFAULT ARRAY['publish-update', 'yank', 'change-owners'],
    
    -- 限制到特定 crate（NULL 表示所有 crate）
    crate_names TEXT[],
    
    -- 有效期限
    expires_at TIMESTAMPTZ,
    
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- 索引触发原因
    revoked_at TIMESTAMPTZ,
    revoked_reason VARCHAR(255)
);

-- Cargo 索引状态表
-- 跟踪每个项目的 Cargo 索引状态
CREATE TABLE cargo_index_status (
    id BIGSERIAL PRIMARY KEY,
    namespace_id BIGINT NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE UNIQUE,
    
    -- 索引仓库路径（相对于 storage 根目录）
    index_repo_path VARCHAR(512) NOT NULL,
    
    -- 最后更新时间
    last_indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- 索引状态
    status VARCHAR(50) NOT NULL DEFAULT 'active',  -- active, rebuilding, error
    error_message TEXT,
    
    -- 统计
    total_crates BIGINT NOT NULL DEFAULT 0,
    total_versions BIGINT NOT NULL DEFAULT 0,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Cargo 审计日志表
-- 记录重要操作（发布、撤回、所有者变更等）
CREATE TABLE cargo_audit_log (
    id BIGSERIAL PRIMARY KEY,
    namespace_id BIGINT NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    crate_name VARCHAR(255) NOT NULL,
    version VARCHAR(255),  -- 某些操作不涉及特定版本
    
    -- 操作类型
    action VARCHAR(50) NOT NULL,  -- publish, yank, unyank, add_owner, remove_owner
    
    -- 执行者
    user_id BIGINT NOT NULL REFERENCES users(id),
    
    -- 详情（JSON 格式存储额外信息）
    details JSONB DEFAULT '{}'::jsonb,
    
    -- IP 地址（用于安全审计）
    ip_address INET,
    user_agent TEXT,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 创建索引
CREATE INDEX idx_cargo_metadata_package ON cargo_crate_metadata(package_id);
CREATE INDEX idx_cargo_dependencies_package ON cargo_dependencies(package_id);
CREATE INDEX idx_cargo_dependencies_name ON cargo_dependencies(name);
CREATE INDEX idx_cargo_owners_namespace_crate ON cargo_crate_owners(namespace_id, crate_name);
CREATE INDEX idx_cargo_owners_user ON cargo_crate_owners(user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_cargo_owners_team ON cargo_crate_owners(team_id) WHERE team_id IS NOT NULL;
CREATE INDEX idx_cargo_download_stats_package ON cargo_download_stats(package_id);
CREATE INDEX idx_cargo_download_stats_date ON cargo_download_stats(download_date);
CREATE INDEX idx_cargo_tokens_user ON cargo_tokens(user_id);
CREATE INDEX idx_cargo_tokens_hash ON cargo_tokens(token_hash);
CREATE INDEX idx_cargo_audit_namespace_crate ON cargo_audit_log(namespace_id, crate_name);
CREATE INDEX idx_cargo_audit_user ON cargo_audit_log(user_id);
CREATE INDEX idx_cargo_audit_created ON cargo_audit_log(created_at);

-- 触发器：更新 packages 表的 updated_at
CREATE OR REPLACE FUNCTION update_package_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE packages SET updated_at = NOW() WHERE id = NEW.package_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER cargo_metadata_update_package
    AFTER INSERT OR UPDATE ON cargo_crate_metadata
    FOR EACH ROW
    EXECUTE FUNCTION update_package_timestamp();

-- 触发器：更新索引状态统计
CREATE OR REPLACE FUNCTION update_cargo_index_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.package_type = 'cargo' THEN
        -- 获取 namespace_id
        UPDATE cargo_index_status cis
        SET 
            total_versions = total_versions + 1,
            updated_at = NOW()
        FROM projects p
        WHERE p.id = NEW.project_id 
          AND cis.namespace_id = p.namespace_id;
        
        -- 如果是新 crate（同名只算一次），更新 total_crates
        IF NOT EXISTS (
            SELECT 1 FROM packages p2 
            JOIN projects proj ON p2.project_id = proj.id
            WHERE p2.name = NEW.name 
              AND p2.package_type = 'cargo'
              AND p2.id != NEW.id
              AND proj.namespace_id = (SELECT namespace_id FROM projects WHERE id = NEW.project_id)
        ) THEN
            UPDATE cargo_index_status cis
            SET total_crates = total_crates + 1
            FROM projects p
            WHERE p.id = NEW.project_id 
              AND cis.namespace_id = p.namespace_id;
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER packages_cargo_stats
    AFTER INSERT ON packages
    FOR EACH ROW
    WHEN (NEW.package_type = 'cargo')
    EXECUTE FUNCTION update_cargo_index_stats();

-- 添加系统配置
INSERT INTO system_configs (key, value) VALUES 
    ('registry_cargo_enabled', 'true'),
    ('registry_cargo_max_crate_size', '10485760'),  -- 10MB 默认上限
    ('registry_cargo_allow_sparse_index', 'true'),  -- 启用 sparse protocol
    ('registry_cargo_require_verified_email', 'false')  -- 发布是否需要已验证邮箱
ON CONFLICT (key) DO NOTHING;

-- 创建清理已撤回 crate API token 的函数
CREATE OR REPLACE FUNCTION cleanup_revoked_cargo_tokens()
RETURNS void AS $$
BEGIN
    -- 删除 30 天前被撤销的 tokens
    DELETE FROM cargo_tokens 
    WHERE revoked_at IS NOT NULL 
      AND revoked_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- 创建用于生成 Cargo 索引条目的函数
-- 这个函数生成符合 Cargo Registry Index 格式的 JSON
CREATE OR REPLACE FUNCTION generate_cargo_index_entry(p_package_id BIGINT)
RETURNS TEXT AS $$
DECLARE
    pkg RECORD;
    meta RECORD;
    deps JSONB;
    features JSONB;
    result JSONB;
BEGIN
    -- 获取包信息
    SELECT * INTO pkg FROM packages WHERE id = p_package_id;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;
    
    -- 获取 cargo 元数据
    SELECT * INTO meta FROM cargo_crate_metadata WHERE package_id = p_package_id;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;
    
    -- 构建依赖数组
    SELECT COALESCE(
        jsonb_agg(
            jsonb_build_object(
                'name', d.name,
                'req', d.version_req,
                'features', COALESCE(d.features, ARRAY[]::text[]),
                'optional', d.optional,
                'default_features', d.default_features,
                'target', d.target,
                'kind', d.kind,
                'registry', d.registry,
                'package', d.explicit_name_in_toml
            )
        ),
        '[]'::jsonb
    ) INTO deps
    FROM cargo_dependencies d
    WHERE d.package_id = p_package_id;
    
    -- 获取特性
    features := COALESCE(meta.features, '{}'::jsonb);
    
    -- 构建索引条目
    result := jsonb_build_object(
        'name', pkg.name,
        'vers', pkg.version,
        'deps', deps,
        'cksum', meta.cksum,
        'features', features,
        'yanked', meta.yanked,
        'links', meta.links
    );
    
    -- 添加可选字段
    IF meta.rust_version IS NOT NULL THEN
        result := result || jsonb_build_object('rust_version', meta.rust_version);
    END IF;
    
    -- 添加 features2（用于隐式可选依赖特性）
    -- Cargo 1.60+ 支持
    IF features ? 'features2' THEN
        result := result || jsonb_build_object('features2', features->'features2');
    END IF;
    
    RETURN result::text;
END;
$$ LANGUAGE plpgsql;

COMMENT ON TABLE cargo_crate_metadata IS 'Cargo crate 版本元数据，每个 crate 版本对应一条记录';
COMMENT ON TABLE cargo_dependencies IS 'Cargo crate 依赖关系，支持 normal/dev/build 三种依赖类型';
COMMENT ON TABLE cargo_crate_owners IS 'Cargo crate 所有者，可以是用户或团队';
COMMENT ON TABLE cargo_download_stats IS 'Cargo crate 下载统计，按日期聚合';
COMMENT ON TABLE cargo_tokens IS 'Cargo API tokens，用于 cargo publish/yank 等操作';
COMMENT ON TABLE cargo_index_status IS 'Cargo 索引状态，跟踪每个 namespace 的索引情况';
COMMENT ON TABLE cargo_audit_log IS 'Cargo 操作审计日志';
COMMENT ON FUNCTION generate_cargo_index_entry IS '生成符合 Cargo Registry Index 格式的 JSON 条目';
