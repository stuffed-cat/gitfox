-- Add source_project_id to merge_requests to support cross-repository merge requests
-- This enables both same-repo MRs (branch to branch) and fork MRs (fork to upstream)

ALTER TABLE merge_requests 
ADD COLUMN source_project_id BIGINT REFERENCES projects(id) ON DELETE CASCADE;

-- For existing MRs, set source_project_id to project_id (same repo)
UPDATE merge_requests SET source_project_id = project_id WHERE source_project_id IS NULL;

-- Now make it NOT NULL
ALTER TABLE merge_requests ALTER COLUMN source_project_id SET NOT NULL;

-- Add index for queries
CREATE INDEX idx_merge_requests_source_project ON merge_requests(source_project_id);

-- Add comment
COMMENT ON COLUMN merge_requests.source_project_id IS 'Source project for the merge request. Can be same as project_id (same-repo MR) or different (fork MR)';
