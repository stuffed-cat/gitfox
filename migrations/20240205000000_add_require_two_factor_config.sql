-- Add require_two_factor system config
INSERT INTO system_configs (key, value) 
VALUES (
    'require_two_factor',
    'false'
)
ON CONFLICT (key) DO NOTHING;

-- Add require_two_factor_admin system config
INSERT INTO system_configs (key, value) 
VALUES (
    'require_two_factor_admin',
    'true'
)
ON CONFLICT (key) DO NOTHING;

-- Add two_factor_grace_period_days system config
INSERT INTO system_configs (key, value) 
VALUES (
    'two_factor_grace_period_days',
    '7'
)
ON CONFLICT (key) DO NOTHING;
