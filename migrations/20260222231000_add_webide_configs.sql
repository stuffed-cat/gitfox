-- Add WebIDE / IDE integration system configuration options
-- GitLab-style VS Code extension marketplace and Gitpod integration

INSERT INTO system_configs (key, value, updated_at) VALUES
    -- VS Code Extension Marketplace settings (for WebIDE)
    ('vscode_extensions_enabled', 'false', NOW()),                                      -- Enable extension marketplace for all users
    ('vscode_extensions_use_open_vsx', 'true', NOW()),                                  -- Use Open VSX Registry
    ('vscode_extensions_service_url', '"https://open-vsx.org/vscode/gallery"', NOW()),  -- Gallery service URL
    ('vscode_extensions_item_url', '"https://open-vsx.org/vscode/item"', NOW()),        -- Item detail URL
    ('vscode_extensions_resource_url', '"https://open-vsx.org/vscode/unpkg/{publisher}/{name}/{version}/{path}"', NOW()),  -- Resource URL template
    
    -- Gitpod integration (cloud development environment)
    ('gitpod_enabled', 'false', NOW()),                    -- Enable Gitpod integration
    ('gitpod_url', '"https://gitpod.io/"', NOW())          -- Gitpod instance URL (self-hosted or gitpod.io)
ON CONFLICT (key) DO NOTHING;
