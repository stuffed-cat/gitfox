-- Add project settings fields for feature toggles and status
-- These fields control which features are enabled for a project

-- Add archived flag
ALTER TABLE projects ADD COLUMN IF NOT EXISTS archived BOOLEAN NOT NULL DEFAULT false;

-- Add feature toggle flags
ALTER TABLE projects ADD COLUMN IF NOT EXISTS issues_enabled BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS merge_requests_enabled BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS pipelines_enabled BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS packages_enabled BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS wiki_enabled BOOLEAN NOT NULL DEFAULT true;

-- Add namespace_id for project organization (for group/user namespaces)
-- Check if the column already exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'projects' AND column_name = 'namespace_id'
    ) THEN
        ALTER TABLE projects ADD COLUMN namespace_id BIGINT REFERENCES namespaces(id) ON DELETE CASCADE;
        
        -- Set namespace_id for existing projects based on owner_id
        UPDATE projects p
        SET namespace_id = (
            SELECT n.id FROM namespaces n 
            WHERE n.namespace_type = 'user' AND n.owner_id = p.owner_id
        )
        WHERE p.namespace_id IS NULL;
    END IF;
END $$;

-- Add default_branch if not exists (it was added then removed in earlier migration)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'projects' AND column_name = 'default_branch'
    ) THEN
        ALTER TABLE projects ADD COLUMN default_branch VARCHAR(100) NOT NULL DEFAULT 'main';
    END IF;
END $$;

-- Create index for archived projects
CREATE INDEX IF NOT EXISTS idx_projects_archived ON projects(archived);
CREATE INDEX IF NOT EXISTS idx_projects_namespace ON projects(namespace_id);
