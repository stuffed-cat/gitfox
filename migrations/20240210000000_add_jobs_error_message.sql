-- Add error_message column to jobs table for storing job execution errors
ALTER TABLE jobs ADD COLUMN error_message TEXT;
