-- Package Registry 表
-- 软件包镜像仓库功能，支持 Docker、npm 等包管理器

-- 包类型枚举
CREATE TYPE package_type AS ENUM (
    'npm',          -- npm JavaScript 包
    'docker',       -- Docker 容器镜像
    'generic'       -- 通用包（预留）
);

-- 包状态枚举
CREATE TYPE package_status AS ENUM (
    'default',      -- 正常状态
    'hidden',       -- 隐藏（不在列表显示）
    'deleted'       -- 已删除（软删除）
);

-- 包表
CREATE TABLE packages (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    package_type package_type NOT NULL,
    version VARCHAR(255) NOT NULL,
    status package_status NOT NULL DEFAULT 'default',
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- 同一项目、包名、类型、版本组合必须唯一
    UNIQUE(project_id, package_type, name, version)
);

-- 包文件表（一个包版本可能包含多个文件）
CREATE TABLE package_files (
    id BIGSERIAL PRIMARY KEY,
    package_id BIGINT NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
    file_name VARCHAR(500) NOT NULL,
    file_path VARCHAR(1000) NOT NULL,  -- 存储路径（相对于 storage 根目录）
    file_size BIGINT NOT NULL,
    file_sha256 VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(package_id, file_name)
);

-- Docker 镜像特有的表
-- Docker manifests 表
CREATE TABLE docker_manifests (
    id BIGSERIAL PRIMARY KEY,
    package_id BIGINT NOT NULL REFERENCES packages(id) ON DELETE CASCADE,
    digest VARCHAR(128) NOT NULL,      -- sha256:xxx
    media_type VARCHAR(255) NOT NULL,  -- application/vnd.docker.distribution.manifest.v2+json 等
    schema_version INTEGER NOT NULL DEFAULT 2,
    config_digest VARCHAR(128),        -- 配置层的 digest
    total_size BIGINT NOT NULL DEFAULT 0,
    manifest_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(package_id, digest)
);

-- Docker blobs 表（层和配置）
CREATE TABLE docker_blobs (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    digest VARCHAR(128) NOT NULL,      -- sha256:xxx
    media_type VARCHAR(255),           -- 可选，某些层没有明确类型
    size BIGINT NOT NULL,
    file_path VARCHAR(1000) NOT NULL,  -- 存储路径
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- 同一项目内 digest 唯一（支持跨镜像复用层）
    UNIQUE(project_id, digest)
);

-- Docker blob 与 manifest 关联表（多对多关系）
CREATE TABLE docker_manifest_blobs (
    id BIGSERIAL PRIMARY KEY,
    manifest_id BIGINT NOT NULL REFERENCES docker_manifests(id) ON DELETE CASCADE,
    blob_id BIGINT NOT NULL REFERENCES docker_blobs(id) ON DELETE CASCADE,
    layer_order INTEGER,  -- 层顺序，NULL 表示是配置层
    
    UNIQUE(manifest_id, blob_id)
);

-- Docker 上传会话表（用于分块上传）
CREATE TABLE docker_upload_sessions (
    id BIGSERIAL PRIMARY KEY,
    uuid VARCHAR(36) NOT NULL UNIQUE,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    digest VARCHAR(128),               -- 预期的 digest（可选）
    uploaded_bytes BIGINT NOT NULL DEFAULT 0,
    temp_path VARCHAR(1000),           -- 临时文件路径
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '1 day'
);

-- npm 特定表
-- npm 包元数据
CREATE TABLE npm_package_metadata (
    id BIGSERIAL PRIMARY KEY,
    package_id BIGINT NOT NULL REFERENCES packages(id) ON DELETE CASCADE UNIQUE,
    dist_tag VARCHAR(50) NOT NULL DEFAULT 'latest',  -- dist-tag (latest, next, beta 等)
    tarball_sha512 VARCHAR(128),       -- npm 使用 sha512
    npm_readme TEXT,
    npm_keywords TEXT[],
    npm_license VARCHAR(255),
    npm_repository JSONB,              -- npm repository 配置
    npm_dependencies JSONB,            -- dependencies
    npm_dev_dependencies JSONB,        -- devDependencies
    npm_peer_dependencies JSONB,       -- peerDependencies
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- npm dist-tags 表（用于跟踪 latest、next 等标签）
CREATE TABLE npm_dist_tags (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    package_name VARCHAR(255) NOT NULL,
    tag VARCHAR(50) NOT NULL,
    version VARCHAR(255) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(project_id, package_name, tag)
);

-- 创建索引
CREATE INDEX idx_packages_project ON packages(project_id);
CREATE INDEX idx_packages_type ON packages(package_type);
CREATE INDEX idx_packages_name ON packages(name);
CREATE INDEX idx_packages_project_type_name ON packages(project_id, package_type, name);
CREATE INDEX idx_package_files_package ON package_files(package_id);
CREATE INDEX idx_docker_manifests_package ON docker_manifests(package_id);
CREATE INDEX idx_docker_manifests_digest ON docker_manifests(digest);
CREATE INDEX idx_docker_blobs_project ON docker_blobs(project_id);
CREATE INDEX idx_docker_blobs_digest ON docker_blobs(digest);
CREATE INDEX idx_docker_upload_sessions_uuid ON docker_upload_sessions(uuid);
CREATE INDEX idx_docker_upload_sessions_expires ON docker_upload_sessions(expires_at);
CREATE INDEX idx_npm_dist_tags_project_package ON npm_dist_tags(project_id, package_name);

-- 添加 package registry 系统配置
INSERT INTO system_configs (key, value) VALUES 
    ('registry_enabled', 'true'),
    ('registry_domain', '""'),
    ('registry_docker_enabled', 'true'),
    ('registry_npm_enabled', 'true'),
    ('registry_storage_path', '"./packages"'),
    ('registry_max_package_size', '536870912')
ON CONFLICT (key) DO NOTHING;

-- 创建清理过期上传会话的函数
CREATE OR REPLACE FUNCTION cleanup_expired_upload_sessions()
RETURNS void AS $$
BEGIN
    DELETE FROM docker_upload_sessions WHERE expires_at < NOW();
END;
$$ LANGUAGE plpgsql;
