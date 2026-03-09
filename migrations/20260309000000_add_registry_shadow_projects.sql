-- 添加 registry 影子项目支持
-- 影子项目用于存储包注册表数据（npm/cargo/docker），对普通用户不可见

-- 添加 is_registry_shadow 字段到 projects 表
ALTER TABLE projects 
ADD COLUMN IF NOT EXISTS is_registry_shadow BOOLEAN DEFAULT FALSE;

-- 修复项目唯一性约束：应该是 (namespace_id, name) 而不是 (owner_id, name)
-- 语义：保证完整路径 namespace/project 唯一，项目名称本身可以重复
-- 例如：alice/myapp 和 bob/myapp 可以同时存在（不同 namespace）
-- 但：alice/myapp 不能重复创建（相同 namespace_id + name）
DO $$ 
BEGIN
    -- 删除旧约束（如果存在）
    IF EXISTS (
        SELECT 1 FROM pg_constraint 
        WHERE conname = 'projects_owner_id_name_key' 
        AND conrelid = 'projects'::regclass
    ) THEN
        ALTER TABLE projects DROP CONSTRAINT projects_owner_id_name_key;
        RAISE NOTICE 'Dropped old UNIQUE(owner_id, name) constraint';
    END IF;

    -- 添加新约束（如果不存在）
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint 
        WHERE conname = 'projects_namespace_id_name_key' 
        AND conrelid = 'projects'::regclass
    ) THEN
        ALTER TABLE projects ADD CONSTRAINT projects_namespace_id_name_key 
            UNIQUE(namespace_id, name);
        RAISE NOTICE 'Added new UNIQUE(namespace_id, name) constraint';
    END IF;
END $$;

-- 创建索引以加快查询
CREATE INDEX IF NOT EXISTS idx_projects_registry_shadow ON projects(namespace_id, is_registry_shadow) 
WHERE is_registry_shadow = TRUE;

-- 为每个 namespace 创建 registry 影子项目
-- 注意：这不会与用户自己创建的 registry 项目冲突，因为 is_registry_shadow 字段区分了它们
INSERT INTO projects (namespace_id, name, description, visibility, owner_id, is_registry_shadow, created_at, updated_at)
SELECT 
    ns.id,
    'registry',
    'Package registry storage (auto-generated)',
    'private',
    -- owner_id 选择逻辑：
    -- 1. 优先使用 namespace 的 owner_id
    -- 2. 如果为 NULL（group 类型），从 group_members 中选第一个成员
    -- 3. 如果还是没有，选系统第一个 admin 用户
    COALESCE(
        ns.owner_id,
        (SELECT user_id FROM group_members gm 
         JOIN groups g ON g.id = gm.group_id 
         WHERE g.namespace_id = ns.id 
         LIMIT 1),
        (SELECT id FROM users WHERE role = 'admin' ORDER BY id LIMIT 1)
    ),
    TRUE,
    NOW(),
    NOW()
FROM namespaces ns
WHERE NOT EXISTS (
    SELECT 1 FROM projects p 
    WHERE p.namespace_id = ns.id AND p.is_registry_shadow = TRUE
)
-- 确保有有效的 owner_id（如果上面的 COALESCE 结果为 NULL，跳过该 namespace）
AND (
    ns.owner_id IS NOT NULL 
    OR EXISTS (SELECT 1 FROM group_members gm JOIN groups g ON g.id = gm.group_id WHERE g.namespace_id = ns.id)
    OR EXISTS (SELECT 1 FROM users WHERE role = 'admin')
);

-- 迁移现有包数据到影子项目
-- 将每个 namespace 下非影子项目中的包迁移到影子项目
DO $$
DECLARE
    ns_record RECORD;
    shadow_project_id BIGINT;
BEGIN
    FOR ns_record IN 
        SELECT DISTINCT ns.id AS namespace_id
        FROM namespaces ns
        JOIN projects p ON p.namespace_id = ns.id
        JOIN packages pkg ON pkg.project_id = p.id
        WHERE p.is_registry_shadow = FALSE
    LOOP
        -- 获取该 namespace 的影子项目 ID
        SELECT id INTO shadow_project_id
        FROM projects
        WHERE namespace_id = ns_record.namespace_id
          AND is_registry_shadow = TRUE
        LIMIT 1;

        IF shadow_project_id IS NOT NULL THEN
            -- 迁移包到影子项目
            UPDATE packages
            SET project_id = shadow_project_id
            WHERE project_id IN (
                SELECT p.id 
                FROM projects p 
                WHERE p.namespace_id = ns_record.namespace_id
                  AND p.is_registry_shadow = FALSE
            );
            
            RAISE NOTICE 'Migrated packages for namespace % to shadow project %', 
                ns_record.namespace_id, shadow_project_id;
        END IF;
    END LOOP;
END $$;

-- 添加注释
COMMENT ON COLUMN projects.is_registry_shadow IS 'Registry shadow project, hidden from normal project lists, used for package storage';
