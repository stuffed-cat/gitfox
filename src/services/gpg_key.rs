//! GPG Key service for system key management and signature verification
//!
//! Handles:
//! - System GPG key generation for WebIDE/API signing
//! - GPG signature verification
//! - Key lookup and management

use crate::error::{AppError, AppResult};
use crate::models::{GpgKey, GpgKeySubkey, ParsedGpgKey, ParsedGpgSubkey};
use chrono::{TimeZone, Utc};
use log::{debug, error, info, warn};
use sqlx::PgPool;
use std::io::Write;
use std::process::{Command, Stdio};

/// GPG Key service for managing GPG keys
pub struct GpgKeyService;

impl GpgKeyService {
    /// Get or create system GPG key for a user
    /// 
    /// This creates a system GPG key that is used for WebIDE/API commits.
    /// The key is not visible in the user's GPG key list.
    pub async fn get_or_create_system_key(
        pool: &PgPool,
        user_id: i64,
        user_email: &str,
        username: &str,
    ) -> AppResult<GpgKey> {
        // Check if system key already exists
        if let Some(key) = sqlx::query_as::<_, GpgKey>(
            "SELECT * FROM gpg_keys WHERE user_id = $1 AND is_system_key = true"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        {
            return Ok(key);
        }

        // Generate new system key
        info!("Generating system GPG key for user {} ({})", user_id, username);
        let (public_key, private_key, parsed) = Self::generate_gpg_key_pair(user_email, username)?;

        // Insert the key
        let key = sqlx::query_as::<_, GpgKey>(
            r#"
            INSERT INTO gpg_keys (
                user_id, primary_key_id, fingerprint, public_key, key_algorithm,
                key_size, emails, can_sign, can_encrypt, can_certify,
                key_created_at, key_expires_at, verified, is_system_key, private_key_encrypted
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&parsed.primary_key_id)
        .bind(&parsed.fingerprint)
        .bind(&public_key)
        .bind(&parsed.algorithm)
        .bind(parsed.key_size)
        .bind(&parsed.emails)
        .bind(parsed.can_sign)
        .bind(parsed.can_encrypt)
        .bind(parsed.can_certify)
        .bind(parsed.created_at)
        .bind(parsed.expires_at)
        .bind(true) // System keys are always verified
        .bind(true) // is_system_key
        .bind(&private_key) // Store private key for signing
        .fetch_one(pool)
        .await?;

        // Insert subkeys if any
        for subkey in &parsed.subkeys {
            sqlx::query(
                r#"
                INSERT INTO gpg_key_subkeys (
                    gpg_key_id, key_id, fingerprint, key_algorithm, key_size,
                    can_sign, can_encrypt, key_created_at, key_expires_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(key.id)
            .bind(&subkey.key_id)
            .bind(&subkey.fingerprint)
            .bind(&subkey.algorithm)
            .bind(subkey.key_size)
            .bind(subkey.can_sign)
            .bind(subkey.can_encrypt)
            .bind(subkey.created_at)
            .bind(subkey.expires_at)
            .execute(pool)
            .await?;
        }

        info!(
            "Created system GPG key {} for user {} (fingerprint: {})",
            key.id, user_id, key.fingerprint
        );

        Ok(key)
    }

    /// Generate a new GPG key pair for system use
    fn generate_gpg_key_pair(
        email: &str,
        name: &str,
    ) -> AppResult<(String, String, ParsedGpgKey)> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            error!("Failed to create temp directory: {}", e);
            AppError::InternalError("Failed to generate GPG key".to_string())
        })?;

        let gpg_home = temp_dir.path();

        // Generate key using batch mode
        // Using Ed25519 for modern security and small key size
        let key_params = format!(
            r#"Key-Type: eddsa
Key-Curve: ed25519
Key-Usage: sign
Subkey-Type: ecdh
Subkey-Curve: cv25519
Subkey-Usage: encrypt
Name-Real: {} (GitFox System Key)
Name-Email: {}
Expire-Date: 0
%no-protection
%commit
"#,
            name, email
        );

        let mut gen_cmd = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--gen-key",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                warn!("Failed to run gpg gen-key: {}", e);
                AppError::InternalError("Failed to generate GPG key (gpg not available)".to_string())
            })?;

        {
            let stdin = gen_cmd.stdin.as_mut().ok_or_else(|| {
                AppError::InternalError("Failed to get gpg stdin".to_string())
            })?;
            stdin.write_all(key_params.as_bytes()).map_err(|e| {
                AppError::InternalError(format!("Failed to write to gpg: {}", e))
            })?;
        }

        let gen_output = gen_cmd.wait_with_output().map_err(|e| {
            AppError::InternalError(format!("Failed to wait for gpg: {}", e))
        })?;

        if !gen_output.status.success() {
            let stderr = String::from_utf8_lossy(&gen_output.stderr);
            error!("GPG key generation failed: {}", stderr);
            return Err(AppError::InternalError(
                "Failed to generate GPG key".to_string(),
            ));
        }

        // Export public key
        let public_output = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--armor",
                "--export",
            ])
            .output()
            .map_err(|e| {
                AppError::InternalError(format!("Failed to export public key: {}", e))
            })?;

        let public_key = String::from_utf8_lossy(&public_output.stdout).to_string();

        // Export private key (for system signing)
        let private_output = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--armor",
                "--export-secret-keys",
            ])
            .output()
            .map_err(|e| {
                AppError::InternalError(format!("Failed to export private key: {}", e))
            })?;

        let private_key = String::from_utf8_lossy(&private_output.stdout).to_string();

        // Parse the generated key to get metadata
        let parsed = Self::parse_gpg_key(&public_key)?;

        Ok((public_key, private_key, parsed))
    }

    /// Sign data using a system GPG key
    pub async fn sign_with_system_key(
        pool: &PgPool,
        user_id: i64,
        data: &str,
    ) -> AppResult<String> {
        // Get the system key with private key
        let key = sqlx::query_as::<_, GpgKey>(
            "SELECT * FROM gpg_keys WHERE user_id = $1 AND is_system_key = true"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("System GPG key not found".to_string()))?;

        let private_key = key.private_key_encrypted.as_ref()
            .ok_or_else(|| AppError::InternalError("System key has no private key".to_string()))?;

        Self::sign_data(private_key, data)
    }

    /// Sign data with a private key
    fn sign_data(private_key: &str, data: &str) -> AppResult<String> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            error!("Failed to create temp directory: {}", e);
            AppError::InternalError("Failed to sign data".to_string())
        })?;

        let gpg_home = temp_dir.path();

        // Import private key
        let mut import_cmd = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--import",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                AppError::InternalError(format!("Failed to run gpg: {}", e))
            })?;

        if let Some(ref mut stdin) = import_cmd.stdin {
            let _ = stdin.write_all(private_key.as_bytes());
        }
        let _ = import_cmd.wait();

        // Sign the data
        let mut sign_cmd = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--armor",
                "--detach-sign",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                AppError::InternalError(format!("Failed to run gpg sign: {}", e))
            })?;

        {
            let stdin = sign_cmd.stdin.as_mut().ok_or_else(|| {
                AppError::InternalError("Failed to get gpg stdin".to_string())
            })?;
            stdin.write_all(data.as_bytes()).map_err(|e| {
                AppError::InternalError(format!("Failed to write to gpg: {}", e))
            })?;
        }

        let sign_output = sign_cmd.wait_with_output().map_err(|e| {
            AppError::InternalError(format!("Failed to wait for gpg: {}", e))
        })?;

        if !sign_output.status.success() {
            let stderr = String::from_utf8_lossy(&sign_output.stderr);
            error!("GPG signing failed: {}", stderr);
            return Err(AppError::InternalError("Failed to sign data".to_string()));
        }

        Ok(String::from_utf8_lossy(&sign_output.stdout).to_string())
    }

    /// Parse a GPG public key to extract metadata
    fn parse_gpg_key(key_data: &str) -> AppResult<ParsedGpgKey> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            error!("Failed to create temp directory: {}", e);
            AppError::InternalError("Failed to parse GPG key".to_string())
        })?;

        let gpg_home = temp_dir.path();

        // Import the key
        let mut import_cmd = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--import",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                AppError::InternalError(format!("Failed to run gpg: {}", e))
            })?;

        if let Some(ref mut stdin) = import_cmd.stdin {
            let _ = stdin.write_all(key_data.as_bytes());
        }
        let _ = import_cmd.wait();

        // List keys with colon-delimited output
        let list_output = Command::new("gpg")
            .args([
                "--homedir",
                gpg_home.to_str().unwrap(),
                "--batch",
                "--with-colons",
                "--with-fingerprint",
                "--list-keys",
            ])
            .output()
            .map_err(|e| {
                AppError::InternalError(format!("Failed to list GPG keys: {}", e))
            })?;

        let output = String::from_utf8_lossy(&list_output.stdout);
        Self::parse_gpg_colon_output(&output)
    }

    /// Parse GPG --with-colons output format
    fn parse_gpg_colon_output(output: &str) -> AppResult<ParsedGpgKey> {
        let mut fingerprint = String::new();
        let mut primary_key_id = String::new();
        let mut algorithm = String::new();
        let mut key_size: Option<i32> = None;
        let mut created_at = None;
        let mut expires_at = None;
        let mut emails = Vec::new();
        let mut can_sign = false;
        let mut can_encrypt = false;
        let mut can_certify = false;
        let mut subkeys = Vec::new();

        let mut current_subkey: Option<ParsedGpgSubkey> = None;
        let mut in_subkey = false;

        for line in output.lines() {
            let fields: Vec<&str> = line.split(':').collect();
            if fields.is_empty() {
                continue;
            }

            match fields[0] {
                "pub" => {
                    if fields.len() > 11 {
                        key_size = fields[2].parse().ok();
                        algorithm = Self::algorithm_from_code(fields[3]);
                        primary_key_id = fields[4].to_string();

                        if let Ok(ts) = fields[5].parse::<i64>() {
                            created_at = Utc.timestamp_opt(ts, 0).single();
                        }

                        if !fields[6].is_empty() {
                            if let Ok(ts) = fields[6].parse::<i64>() {
                                expires_at = Utc.timestamp_opt(ts, 0).single();
                            }
                        }

                        if fields.len() > 11 {
                            let caps = fields[11];
                            can_sign = caps.contains('s') || caps.contains('S');
                            can_encrypt = caps.contains('e') || caps.contains('E');
                            can_certify = caps.contains('c') || caps.contains('C');
                        }
                    }
                    in_subkey = false;
                }
                "fpr" => {
                    if fields.len() > 9 {
                        if in_subkey {
                            if let Some(ref mut sk) = current_subkey {
                                sk.fingerprint = fields[9].to_string();
                            }
                        } else {
                            fingerprint = fields[9].to_string();
                        }
                    }
                }
                "uid" => {
                    if fields.len() > 9 {
                        let uid = fields[9];
                        if let Some(start) = uid.find('<') {
                            if let Some(end) = uid.find('>') {
                                let email = uid[start + 1..end].to_string();
                                if !emails.contains(&email) {
                                    emails.push(email);
                                }
                            }
                        }
                    }
                }
                "sub" => {
                    if let Some(sk) = current_subkey.take() {
                        subkeys.push(sk);
                    }

                    let mut sk = ParsedGpgSubkey {
                        key_id: String::new(),
                        fingerprint: String::new(),
                        algorithm: String::new(),
                        key_size: None,
                        created_at: None,
                        expires_at: None,
                        can_sign: false,
                        can_encrypt: false,
                    };

                    if fields.len() > 11 {
                        sk.key_size = fields[2].parse().ok();
                        sk.algorithm = Self::algorithm_from_code(fields[3]);
                        sk.key_id = fields[4].to_string();

                        if let Ok(ts) = fields[5].parse::<i64>() {
                            sk.created_at = Utc.timestamp_opt(ts, 0).single();
                        }

                        if !fields[6].is_empty() {
                            if let Ok(ts) = fields[6].parse::<i64>() {
                                sk.expires_at = Utc.timestamp_opt(ts, 0).single();
                            }
                        }

                        if fields.len() > 11 {
                            let caps = fields[11];
                            sk.can_sign = caps.contains('s') || caps.contains('S');
                            sk.can_encrypt = caps.contains('e') || caps.contains('E');
                        }
                    }

                    current_subkey = Some(sk);
                    in_subkey = true;
                }
                _ => {}
            }
        }

        if let Some(sk) = current_subkey.take() {
            subkeys.push(sk);
        }

        if fingerprint.is_empty() {
            return Err(AppError::BadRequest(
                "Could not extract fingerprint from GPG key".to_string(),
            ));
        }

        Ok(ParsedGpgKey {
            fingerprint,
            primary_key_id,
            algorithm,
            key_size,
            created_at,
            expires_at,
            emails,
            can_sign,
            can_encrypt,
            can_certify,
            subkeys,
        })
    }

    /// Convert GPG algorithm code to human-readable string
    fn algorithm_from_code(code: &str) -> String {
        match code {
            "1" => "RSA".to_string(),
            "2" => "RSA".to_string(),
            "3" => "RSA".to_string(),
            "16" => "Elgamal".to_string(),
            "17" => "DSA".to_string(),
            "18" => "ECDH".to_string(),
            "19" => "ECDSA".to_string(),
            "22" => "EdDSA".to_string(),
            _ => format!("Unknown({})", code),
        }
    }

    /// Ensure all existing users have system GPG keys
    /// Called during startup or as a background job
    pub async fn ensure_all_users_have_system_keys(pool: &PgPool) -> AppResult<u64> {
        // Find users without system keys
        let users: Vec<(i64, String, String)> = sqlx::query_as(
            r#"
            SELECT u.id, u.email, u.username
            FROM users u
            WHERE NOT EXISTS (
                SELECT 1 FROM gpg_keys g
                WHERE g.user_id = u.id AND g.is_system_key = true
            )
            "#
        )
        .fetch_all(pool)
        .await?;

        let count = users.len() as u64;
        
        if count > 0 {
            info!("Creating system GPG keys for {} users without them", count);
        }

        for (user_id, email, username) in users {
            match Self::get_or_create_system_key(pool, user_id, &email, &username).await {
                Ok(_) => debug!("Created system GPG key for user {}", user_id),
                Err(e) => warn!("Failed to create system GPG key for user {}: {}", user_id, e),
            }
        }

        Ok(count)
    }

    /// Get user's system GPG key public key (for display in UI)
    pub async fn get_system_key_public_info(
        pool: &PgPool,
        user_id: i64,
    ) -> AppResult<Option<(String, String)>> {
        let key: Option<(String, String)> = sqlx::query_as(
            "SELECT fingerprint, primary_key_id FROM gpg_keys WHERE user_id = $1 AND is_system_key = true"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(key)
    }
}
