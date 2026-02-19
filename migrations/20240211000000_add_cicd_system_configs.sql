-- Add CI/CD related system configurations

INSERT INTO system_configs (key, value) VALUES
    -- Job timeout settings
    ('ci_default_job_timeout', '3600'),           -- Default job timeout in seconds (1 hour)
    ('ci_max_job_timeout', '86400'),              -- Maximum allowed job timeout (24 hours)
    ('ci_timeout_check_interval', '30'),          -- How often to check for timed-out jobs (seconds)
    
    -- Runner settings
    ('ci_runner_registration_enabled', 'true'),   -- Allow new runners to register
    ('ci_concurrent_jobs_limit', '100'),          -- System-wide concurrent jobs limit
    
    -- Pipeline settings
    ('ci_max_pipeline_size', '50'),               -- Maximum jobs per pipeline
    ('ci_pipeline_retention_days', '90'),         -- How long to keep pipeline data
    ('ci_artifacts_retention_days', '30'),        -- Default artifacts retention
    
    -- Log settings
    ('ci_max_log_size_mb', '100'),                -- Maximum log file size per job
    ('ci_log_streaming_enabled', 'true')          -- Enable WebSocket log streaming
ON CONFLICT (key) DO NOTHING;
