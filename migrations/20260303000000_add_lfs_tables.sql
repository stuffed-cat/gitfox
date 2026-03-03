-- Add Git LFS (Large File Storage) tables

-- LFS objects table - stores metadata for all LFS objects
CREATE TABLE lfs_objects (
    id BIGSERIAL PRIMARY KEY,
    oid VARCHAR(64) NOT NULL UNIQUE,  -- SHA-256 hash of the file
    size BIGINT NOT NULL,             -- File size in bytes
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for faster OID lookups
CREATE INDEX idx_lfs_objects_oid ON lfs_objects(oid);

-- Project LFS objects - many-to-many relationship between projects and LFS objects
-- An LFS object can be shared across multiple projects (deduplication)
CREATE TABLE project_lfs_objects (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    lfs_object_id BIGINT NOT NULL REFERENCES lfs_objects(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, lfs_object_id)
);

CREATE INDEX idx_project_lfs_objects_project ON project_lfs_objects(project_id);
CREATE INDEX idx_project_lfs_objects_lfs_object ON project_lfs_objects(lfs_object_id);

-- LFS locks table - for file locking feature
CREATE TABLE lfs_locks (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    path VARCHAR(1024) NOT NULL,      -- File path being locked
    ref_name VARCHAR(255),            -- Optional branch reference
    locked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, path)
);

CREATE INDEX idx_lfs_locks_project ON lfs_locks(project_id);
CREATE INDEX idx_lfs_locks_user ON lfs_locks(user_id);

-- LFS batch operations tracking (optional, for rate limiting / metrics)
CREATE TABLE lfs_batch_operations (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    operation VARCHAR(10) NOT NULL,   -- 'download' or 'upload'
    object_count INTEGER NOT NULL,
    total_size BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_lfs_batch_operations_project ON lfs_batch_operations(project_id);
CREATE INDEX idx_lfs_batch_operations_created ON lfs_batch_operations(created_at);

-- Add LFS enabled flag to projects (optional, for per-project LFS toggle)
ALTER TABLE projects ADD COLUMN IF NOT EXISTS lfs_enabled BOOLEAN NOT NULL DEFAULT true;

-- Add LFS storage used tracking to projects
ALTER TABLE projects ADD COLUMN IF NOT EXISTS lfs_storage_used BIGINT NOT NULL DEFAULT 0;

-- Add comment
COMMENT ON TABLE lfs_objects IS 'Git LFS object metadata - stores SHA-256 OID and size';
COMMENT ON TABLE project_lfs_objects IS 'Links projects to LFS objects - enables deduplication across projects';
COMMENT ON TABLE lfs_locks IS 'Git LFS file locking - prevents concurrent edits on binary files';
COMMENT ON TABLE lfs_batch_operations IS 'LFS batch operation tracking for metrics and rate limiting';
