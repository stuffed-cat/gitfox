-- Add trusted and skip_authorization flags to OAuth applications
-- Only admins can create trusted applications
-- Only trusted applications can use skip_authorization

-- Add trusted column if not exists
DO $$ 
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'oauth_applications' AND column_name = 'trusted'
    ) THEN
        ALTER TABLE oauth_applications 
        ADD COLUMN trusted BOOLEAN NOT NULL DEFAULT FALSE;
    END IF;
END $$;

-- Add skip_authorization column if not exists
DO $$ 
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'oauth_applications' AND column_name = 'skip_authorization'
    ) THEN
        ALTER TABLE oauth_applications 
        ADD COLUMN skip_authorization BOOLEAN NOT NULL DEFAULT FALSE;
    END IF;
END $$;

-- Add constraint: only trusted apps can skip authorization (if not exists)
DO $$ 
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint 
        WHERE conname = 'check_skip_authorization_requires_trusted'
    ) THEN
        ALTER TABLE oauth_applications 
        ADD CONSTRAINT check_skip_authorization_requires_trusted 
        CHECK (NOT skip_authorization OR trusted);
    END IF;
END $$;

-- Add index for trusted apps query (if not exists)
DO $$ 
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_indexes 
        WHERE indexname = 'idx_oauth_applications_trusted'
    ) THEN
        CREATE INDEX idx_oauth_applications_trusted 
        ON oauth_applications(trusted) WHERE trusted = TRUE;
    END IF;
END $$;

