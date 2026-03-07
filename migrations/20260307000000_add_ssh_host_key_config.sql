-- SSH Host Key configuration for cluster deployment
-- Store SSH host key in database for cluster-wide consistency
-- gitfox-shell nodes can fetch the key via gRPC from main app

-- ssh_host_key_private: PEM encoded Ed25519 private key
-- ssh_host_key_public: OpenSSH format public key (for display/verification)
-- ssh_host_key_fingerprint: SHA256 fingerprint (for verification)
INSERT INTO system_configs (key, value, updated_at) VALUES
    ('ssh_host_key_private', '""', NOW()),
    ('ssh_host_key_public', '""', NOW()),
    ('ssh_host_key_fingerprint', '""', NOW())
ON CONFLICT (key) DO NOTHING;
