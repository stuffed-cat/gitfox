//! SSH host key management

use std::path::Path;

use anyhow::{Context, Result};
use log::info;
use russh_keys::key::KeyPair;
use russh_keys::PublicKeyBase64;
use tokio::fs;

/// Manages SSH host keys for the server
pub struct HostKeyManager {
    key_path: String,
}

impl HostKeyManager {
    /// Create a new host key manager
    pub fn new(key_path: &str) -> Result<Self> {
        Ok(Self {
            key_path: key_path.to_string(),
        })
    }

    /// Get or generate host keys
    pub async fn get_keys(&self) -> Result<Vec<KeyPair>> {
        let ed25519_path = format!("{}_ed25519", self.key_path);
        
        // Try to load existing key
        if Path::new(&ed25519_path).exists() {
            info!("Loading SSH host key from {}", ed25519_path);
            let key_data = fs::read(&ed25519_path)
                .await
                .context("Failed to read host key")?;
            
            let key = russh_keys::decode_secret_key(&String::from_utf8_lossy(&key_data), None)
                .context("Failed to decode host key")?;
            
            return Ok(vec![key]);
        }

        // Generate new key
        info!("Generating new SSH host key at {}", ed25519_path);
        let key = KeyPair::generate_ed25519()
            .context("Failed to generate Ed25519 key")?;

        // Ensure directory exists
        if let Some(parent) = Path::new(&ed25519_path).parent() {
            fs::create_dir_all(parent).await?;
        }

        // Save the key - encode to a Vec<u8> buffer
        let mut key_data = Vec::new();
        russh_keys::encode_pkcs8_pem(&key, &mut key_data)
            .context("Failed to encode host key")?;
        
        fs::write(&ed25519_path, &key_data)
            .await
            .context("Failed to write host key")?;

        // Set permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&ed25519_path).await?;
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&ed25519_path, perms).await?;
        }

        info!("SSH host key generated successfully");

        Ok(vec![key])
    }

    /// Get the public key fingerprint for display
    pub fn get_fingerprint(key: &KeyPair) -> String {
        use base64::Engine;
        
        match key.clone_public_key() {
            Ok(public_key) => {
                let blob = public_key.public_key_bytes();
                let hash = ring::digest::digest(&ring::digest::SHA256, &blob);
                let fingerprint = base64::engine::general_purpose::STANDARD.encode(hash.as_ref());
                format!("SHA256:{}", fingerprint.trim_end_matches('='))
            }
            Err(_) => "unknown".to_string(),
        }
    }
}
