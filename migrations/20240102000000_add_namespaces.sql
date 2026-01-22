-- Add namespace and groups support for GitLab-style paths

-- Create namespace_type enum
DO $$ BEGIN
    CREATE TYPE namespace_type AS ENUM ('user', 'group');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create namespace_visibility enum (reuse from project if possible)
DO $$ BEGIN
    CREATE TYPE namespace_visibility AS ENUM ('public', 'private', 'internal');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create access_level type for group membership
DO $$ BEGIN
    CREATE TYPE access_level AS ENUM ('10', '20', '30', '40', '50');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create namespaces table
CREATE TABLE IF NOT EXISTS namespaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    path VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    avatar_url TEXT,
    namespace_type namespace_type NOT NULL DEFAULT 'user',
    parent_id UUID REFERENCES namespaces(id) ON DELETE CASCADE,
    visibility namespace_visibility NOT NULL DEFAULT 'private',
    owner_id UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create groups table
CREATE TABLE IF NOT EXISTS groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    namespace_id UUID NOT NULL REFERENCES namespaces(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    path VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    avatar_url TEXT,
    visibility namespace_visibility NOT NULL DEFAULT 'private',
    parent_id UUID REFERENCES groups(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create group_members table
CREATE TABLE IF NOT EXISTS group_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access_level INTEGER NOT NULL DEFAULT 10, -- 10=Guest, 20=Reporter, 30=Developer, 40=Maintainer, 50=Owner
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    UNIQUE(group_id, user_id)
);

-- Add namespace_id to projects table
ALTER TABLE projects 
ADD COLUMN IF NOT EXISTS namespace_id UUID REFERENCES namespaces(id) ON DELETE CASCADE;

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_namespaces_path ON namespaces(path);
CREATE INDEX IF NOT EXISTS idx_namespaces_parent ON namespaces(parent_id);
CREATE INDEX IF NOT EXISTS idx_namespaces_type ON namespaces(namespace_type);
CREATE INDEX IF NOT EXISTS idx_groups_path ON groups(path);
CREATE INDEX IF NOT EXISTS idx_groups_parent ON groups(parent_id);
CREATE INDEX IF NOT EXISTS idx_group_members_group ON group_members(group_id);
CREATE INDEX IF NOT EXISTS idx_group_members_user ON group_members(user_id);
CREATE INDEX IF NOT EXISTS idx_projects_namespace ON projects(namespace_id);

-- Create namespaces for existing users
INSERT INTO namespaces (id, name, path, namespace_type, visibility, owner_id, created_at, updated_at)
SELECT gen_random_uuid(), display_name, username, 'user'::namespace_type, 'public'::namespace_visibility, id, created_at, updated_at
FROM users
WHERE NOT EXISTS (
    SELECT 1 FROM namespaces WHERE path = users.username
);

-- Update projects to use owner's namespace
UPDATE projects p
SET namespace_id = n.id
FROM namespaces n
JOIN users u ON n.owner_id = u.id
WHERE p.owner_id = u.id AND p.namespace_id IS NULL;

-- Create trigger to auto-create namespace for new users
CREATE OR REPLACE FUNCTION create_user_namespace()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO namespaces (name, path, namespace_type, visibility, owner_id)
    VALUES (NEW.display_name, NEW.username, 'user', 'public', NEW.id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS user_namespace_trigger ON users;
CREATE TRIGGER user_namespace_trigger
AFTER INSERT ON users
FOR EACH ROW
EXECUTE FUNCTION create_user_namespace();
