-- Add error_message column to pipelines table for storing CI configuration errors
ALTER TABLE pipelines ADD COLUMN error_message TEXT;

-- Add error_message column to pipeline_jobs table for storing job execution errors
ALTER TABLE pipeline_jobs ADD COLUMN error_message TEXT;
