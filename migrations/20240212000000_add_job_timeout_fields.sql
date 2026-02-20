-- Add timeout tracking fields for multi-instance safety

ALTER TABLE jobs ADD COLUMN timeout_at TIMESTAMPTZ;
ALTER TABLE jobs ADD COLUMN watcher_instance VARCHAR(255);

-- Index for timeout checking
CREATE INDEX idx_jobs_timeout_at ON jobs(timeout_at) WHERE status = 'running';
CREATE INDEX idx_jobs_watcher ON jobs(watcher_instance) WHERE status = 'running';

COMMENT ON COLUMN jobs.timeout_at IS 'Timestamp when job should timeout (for efficient queries)';
COMMENT ON COLUMN jobs.watcher_instance IS 'Which devops instance is watching this job timeout (format: hostname:pid)';
