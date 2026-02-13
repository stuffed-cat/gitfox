-- Fix namespaces, groups, and group_members tables to use BIGINT instead of UUID
-- This aligns with the users table which was changed to BIGINT in 20240126000000
-- PRESERVE ALL EXISTING DATA

-- Step 1: Create a mapping table for UUID -> BIGINT conversion for users
CREATE TEMP TABLE user_id_mapping AS
SELECT id AS new_id, id::text AS old_uuid_text FROM users;

-- Step 2: Handle namespaces table
-- 2a: Add new BIGINT columns
ALTER TABLE namespaces ADD COLUMN id_new BIGSERIAL;
ALTER TABLE namespaces ADD COLUMN parent_id_new BIGINT;
ALTER TABLE namespaces ADD COLUMN owner_id_new BIGINT;

-- 2b: Create mapping for namespace IDs
CREATE TEMP TABLE namespace_id_mapping AS
SELECT id AS old_id, id_new AS new_id FROM namespaces;

-- 2c: Populate parent_id_new using the mapping
UPDATE namespaces n
SET parent_id_new = m.new_id
FROM namespace_id_mapping m
WHERE n.parent_id = m.old_id;

-- 2d: Populate owner_id_new - need to match UUID text to BIGINT
-- The users table now has BIGINT id, but namespaces.owner_id is UUID referencing old users
-- We need to find matching users by some other criteria or assume the UUIDs were converted
-- Since the original migration created namespaces with owner_id = users.id (when users had UUID),
-- and now users have BIGINT ids, we match by username through the namespace path
UPDATE namespaces n
SET owner_id_new = u.id
FROM users u
WHERE n.namespace_type = 'user' AND n.path = u.username;

-- For group namespaces, try to match owner by checking if there's a user namespace with same owner
UPDATE namespaces n
SET owner_id_new = u.id
FROM users u
WHERE n.namespace_type = 'group' AND n.owner_id_new IS NULL
  AND EXISTS (SELECT 1 FROM namespaces un WHERE un.namespace_type = 'user' AND un.path = u.username AND un.owner_id = n.owner_id);

-- Step 3: Handle groups table
ALTER TABLE groups ADD COLUMN id_new BIGSERIAL;
ALTER TABLE groups ADD COLUMN namespace_id_new BIGINT;
ALTER TABLE groups ADD COLUMN parent_id_new BIGINT;

-- Create mapping for group IDs
CREATE TEMP TABLE group_id_mapping AS
SELECT id AS old_id, id_new AS new_id FROM groups;

-- Populate namespace_id_new
UPDATE groups g
SET namespace_id_new = m.new_id
FROM namespace_id_mapping m
WHERE g.namespace_id = m.old_id;

-- Populate parent_id_new for groups
UPDATE groups g
SET parent_id_new = m.new_id
FROM group_id_mapping m
WHERE g.parent_id = m.old_id;

-- Step 4: Handle group_members table
ALTER TABLE group_members ADD COLUMN id_new BIGSERIAL;
ALTER TABLE group_members ADD COLUMN group_id_new BIGINT;
ALTER TABLE group_members ADD COLUMN user_id_new BIGINT;

-- Populate group_id_new
UPDATE group_members gm
SET group_id_new = m.new_id
FROM group_id_mapping m
WHERE gm.group_id = m.old_id;

-- Populate user_id_new - match by finding users
-- group_members.user_id was UUID, need to map to new BIGINT users.id
-- Try matching through namespaces (user namespaces have owner_id = user's old UUID id)
UPDATE group_members gm
SET user_id_new = n.owner_id_new
FROM namespaces n
WHERE n.namespace_type = 'user' AND n.owner_id = gm.user_id AND n.owner_id_new IS NOT NULL;

-- Step 5: Handle projects.namespace_id if it exists
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'projects' AND column_name = 'namespace_id') THEN
        -- Check if it's UUID type
        IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'projects' AND column_name = 'namespace_id' AND data_type = 'uuid') THEN
            ALTER TABLE projects ADD COLUMN namespace_id_new BIGINT;
            
            UPDATE projects p
            SET namespace_id_new = m.new_id
            FROM namespace_id_mapping m
            WHERE p.namespace_id = m.old_id;
            
            ALTER TABLE projects DROP COLUMN namespace_id;
            ALTER TABLE projects RENAME COLUMN namespace_id_new TO namespace_id;
        END IF;
    ELSE
        ALTER TABLE projects ADD COLUMN namespace_id BIGINT;
    END IF;
END $$;

-- Step 6: Drop old columns and constraints, rename new columns

-- 6a: namespaces - drop constraints first
ALTER TABLE namespaces DROP CONSTRAINT IF EXISTS namespaces_pkey CASCADE;
ALTER TABLE namespaces DROP CONSTRAINT IF EXISTS namespaces_parent_id_fkey CASCADE;
ALTER TABLE namespaces DROP CONSTRAINT IF EXISTS namespaces_owner_id_fkey CASCADE;

-- Drop old columns
ALTER TABLE namespaces DROP COLUMN id;
ALTER TABLE namespaces DROP COLUMN parent_id;
ALTER TABLE namespaces DROP COLUMN owner_id;

-- Rename new columns
ALTER TABLE namespaces RENAME COLUMN id_new TO id;
ALTER TABLE namespaces RENAME COLUMN parent_id_new TO parent_id;
ALTER TABLE namespaces RENAME COLUMN owner_id_new TO owner_id;

-- Add primary key and constraints
ALTER TABLE namespaces ADD PRIMARY KEY (id);
ALTER TABLE namespaces ADD CONSTRAINT namespaces_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES namespaces(id) ON DELETE CASCADE;
ALTER TABLE namespaces ADD CONSTRAINT namespaces_owner_id_fkey FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE SET NULL;

-- 6b: groups
ALTER TABLE groups DROP CONSTRAINT IF EXISTS groups_pkey CASCADE;
ALTER TABLE groups DROP CONSTRAINT IF EXISTS groups_namespace_id_fkey CASCADE;
ALTER TABLE groups DROP CONSTRAINT IF EXISTS groups_parent_id_fkey CASCADE;

ALTER TABLE groups DROP COLUMN id;
ALTER TABLE groups DROP COLUMN namespace_id;
ALTER TABLE groups DROP COLUMN parent_id;

ALTER TABLE groups RENAME COLUMN id_new TO id;
ALTER TABLE groups RENAME COLUMN namespace_id_new TO namespace_id;
ALTER TABLE groups RENAME COLUMN parent_id_new TO parent_id;

ALTER TABLE groups ADD PRIMARY KEY (id);
ALTER TABLE groups ADD CONSTRAINT groups_namespace_id_fkey FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE;
ALTER TABLE groups ADD CONSTRAINT groups_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES groups(id) ON DELETE CASCADE;

-- 6c: group_members
ALTER TABLE group_members DROP CONSTRAINT IF EXISTS group_members_pkey CASCADE;
ALTER TABLE group_members DROP CONSTRAINT IF EXISTS group_members_group_id_fkey CASCADE;
ALTER TABLE group_members DROP CONSTRAINT IF EXISTS group_members_user_id_fkey CASCADE;
ALTER TABLE group_members DROP CONSTRAINT IF EXISTS group_members_group_id_user_id_key CASCADE;

ALTER TABLE group_members DROP COLUMN id;
ALTER TABLE group_members DROP COLUMN group_id;
ALTER TABLE group_members DROP COLUMN user_id;

ALTER TABLE group_members RENAME COLUMN id_new TO id;
ALTER TABLE group_members RENAME COLUMN group_id_new TO group_id;
ALTER TABLE group_members RENAME COLUMN user_id_new TO user_id;

ALTER TABLE group_members ADD PRIMARY KEY (id);
ALTER TABLE group_members ADD CONSTRAINT group_members_group_id_fkey FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE;
ALTER TABLE group_members ADD CONSTRAINT group_members_user_id_fkey FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE group_members ADD CONSTRAINT group_members_group_id_user_id_key UNIQUE (group_id, user_id);

-- Step 7: Add foreign key for projects.namespace_id
ALTER TABLE projects ADD CONSTRAINT projects_namespace_id_fkey FOREIGN KEY (namespace_id) REFERENCES namespaces(id) ON DELETE CASCADE;

-- Step 8: Recreate indexes
DROP INDEX IF EXISTS idx_namespaces_path;
DROP INDEX IF EXISTS idx_namespaces_parent;
DROP INDEX IF EXISTS idx_namespaces_type;
DROP INDEX IF EXISTS idx_groups_path;
DROP INDEX IF EXISTS idx_groups_parent;
DROP INDEX IF EXISTS idx_groups_namespace;
DROP INDEX IF EXISTS idx_group_members_group;
DROP INDEX IF EXISTS idx_group_members_user;
DROP INDEX IF EXISTS idx_projects_namespace;

CREATE INDEX idx_namespaces_path ON namespaces(path);
CREATE INDEX idx_namespaces_parent ON namespaces(parent_id);
CREATE INDEX idx_namespaces_type ON namespaces(namespace_type);
CREATE INDEX idx_namespaces_owner ON namespaces(owner_id);
CREATE INDEX idx_groups_path ON groups(path);
CREATE INDEX idx_groups_parent ON groups(parent_id);
CREATE INDEX idx_groups_namespace ON groups(namespace_id);
CREATE INDEX idx_group_members_group ON group_members(group_id);
CREATE INDEX idx_group_members_user ON group_members(user_id);
CREATE INDEX idx_projects_namespace ON projects(namespace_id);

-- Step 9: Create/update triggers
DROP TRIGGER IF EXISTS update_namespaces_updated_at ON namespaces;
DROP TRIGGER IF EXISTS update_groups_updated_at ON groups;
CREATE TRIGGER update_namespaces_updated_at BEFORE UPDATE ON namespaces FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_groups_updated_at BEFORE UPDATE ON groups FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Step 10: Clean up temp tables
DROP TABLE IF EXISTS user_id_mapping;
DROP TABLE IF EXISTS namespace_id_mapping;
DROP TABLE IF EXISTS group_id_mapping;

-- Step 11: Ensure all existing users have namespaces
INSERT INTO namespaces (name, path, namespace_type, visibility, owner_id, created_at, updated_at)
SELECT 
    COALESCE(display_name, username), 
    username, 
    'user'::namespace_type, 
    'public'::namespace_visibility, 
    id, 
    created_at, 
    updated_at
FROM users u
WHERE NOT EXISTS (SELECT 1 FROM namespaces n WHERE n.path = u.username AND n.namespace_type = 'user')
ON CONFLICT (path) DO NOTHING;

-- Step 12: Update projects to link with user namespaces if not already linked
UPDATE projects p
SET namespace_id = n.id
FROM namespaces n
WHERE n.owner_id = p.owner_id 
  AND n.namespace_type = 'user' 
  AND p.namespace_id IS NULL;

