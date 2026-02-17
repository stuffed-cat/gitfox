-- Add user status fields for presence and availability management
ALTER TABLE users
    ADD COLUMN status_emoji VARCHAR(20),
    ADD COLUMN status_message VARCHAR(255),
    ADD COLUMN busy BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN status_set_at TIMESTAMPTZ,
    ADD COLUMN status_clear_at TIMESTAMPTZ;

COMMENT ON COLUMN users.status_emoji IS 'Emoji representing user status';
COMMENT ON COLUMN users.status_message IS 'Custom status message';
COMMENT ON COLUMN users.busy IS 'Whether user is marked as busy';
COMMENT ON COLUMN users.status_set_at IS 'When the status was set';
COMMENT ON COLUMN users.status_clear_at IS 'When the status should auto-clear';
