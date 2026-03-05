//! GPG Key handlers for user-facing API

use actix_web::{web, HttpResponse};
use chrono::{DateTime, TimeZone, Utc};
use log::{debug, error, info, warn};
use sqlx::PgPool;
use std::io::Write;
use std::process::{Command, Stdio};
use validator::Validate;

use crate::error::AppError;
use crate::middleware::AuthenticatedUser;
use crate::models::{
    CreateGpgKeyRequest, GpgKey, GpgKeyResponse, GpgKeySubkey, GpgKeySubkeyResponse,
    ParsedGpgKey, ParsedGpgSubkey,
};

/// List GPG keys for the authenticated user
/// GET /api/v1/user/gpg_keys
pub async fn list_gpg_keys(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    // Filter out system keys - they should not be visible to users
    let keys = sqlx::query_as::<_, GpgKey>(
        r#"SELECT * FROM gpg_keys WHERE user_id = $1 AND is_system_key = false ORDER BY created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await?;

    let mut responses = Vec::new();
    for key in keys {
        let subkeys = sqlx::query_as::<_, GpgKeySubkey>(
            r#"SELECT * FROM gpg_key_subkeys WHERE gpg_key_id = $1 ORDER BY created_at"#,
        )
        .bind(key.id)
        .fetch_all(pool.get_ref())
        .await?;

        responses.push(key.to_response(subkeys));
    }

    Ok(HttpResponse::Ok().json(responses))
}

/// Get a specific GPG key
/// GET /api/v1/user/gpg_keys/{id}
pub async fn get_gpg_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let key_id = path.into_inner();

    let key = sqlx::query_as::<_, GpgKey>(
        r#"SELECT * FROM gpg_keys WHERE id = $1 AND user_id = $2"#,
    )
    .bind(key_id)
    .bind(auth.user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("GPG key not found".to_string()))?;

    let subkeys = sqlx::query_as::<_, GpgKeySubkey>(
        r#"SELECT * FROM gpg_key_subkeys WHERE gpg_key_id = $1 ORDER BY created_at"#,
    )
    .bind(key.id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(key.to_response(subkeys)))
}

/// Add a new GPG key
/// POST /api/v1/user/gpg_keys
pub async fn create_gpg_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    body: web::Json<CreateGpgKeyRequest>,
) -> Result<HttpResponse, AppError> {
    body.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let key_data = body.key.trim();

    // Validate this is an ASCII-armored public key
    if !key_data.starts_with("-----BEGIN PGP PUBLIC KEY BLOCK-----") {
        return Err(AppError::BadRequest(
            "Invalid GPG key format. Please provide an ASCII-armored public key.".to_string(),
        ));
    }

    // Parse the GPG key
    let parsed = parse_gpg_key(key_data)?;

    // Check if key already exists
    let existing = sqlx::query_scalar::<_, i64>(
        r#"SELECT id FROM gpg_keys WHERE fingerprint = $1"#,
    )
    .bind(&parsed.fingerprint)
    .fetch_optional(pool.get_ref())
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "This GPG key is already registered".to_string(),
        ));
    }

    // Get user's verified emails to check if any match
    let user_emails: Vec<String> = sqlx::query_scalar(
        r#"SELECT email FROM users WHERE id = $1"#,
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await?;

    // Check if any of the key's emails match the user's email
    let verified = parsed
        .emails
        .iter()
        .any(|key_email| user_emails.iter().any(|ue| ue.eq_ignore_ascii_case(key_email)));

    // Insert the key
    let key = sqlx::query_as::<_, GpgKey>(
        r#"
        INSERT INTO gpg_keys (
            user_id, primary_key_id, fingerprint, public_key, key_algorithm,
            key_size, emails, can_sign, can_encrypt, can_certify,
            key_created_at, key_expires_at, verified
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&parsed.primary_key_id)
    .bind(&parsed.fingerprint)
    .bind(key_data)
    .bind(&parsed.algorithm)
    .bind(parsed.key_size)
    .bind(&parsed.emails)
    .bind(parsed.can_sign)
    .bind(parsed.can_encrypt)
    .bind(parsed.can_certify)
    .bind(parsed.created_at)
    .bind(parsed.expires_at)
    .bind(verified)
    .fetch_one(pool.get_ref())
    .await?;

    // Insert subkeys
    let mut subkeys = Vec::new();
    for subkey in &parsed.subkeys {
        let sk = sqlx::query_as::<_, GpgKeySubkey>(
            r#"
            INSERT INTO gpg_key_subkeys (
                gpg_key_id, key_id, fingerprint, key_algorithm, key_size,
                can_sign, can_encrypt, key_created_at, key_expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
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
        .fetch_one(pool.get_ref())
        .await?;
        subkeys.push(sk);
    }

    info!(
        "User {} added GPG key {} (fingerprint: {})",
        auth.user_id, key.id, key.fingerprint
    );

    Ok(HttpResponse::Created().json(key.to_response(subkeys)))
}

/// Delete a GPG key
/// DELETE /api/v1/user/gpg_keys/{id}
pub async fn delete_gpg_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let key_id = path.into_inner();

    let result = sqlx::query("DELETE FROM gpg_keys WHERE id = $1 AND user_id = $2")
        .bind(key_id)
        .bind(auth.user_id)
        .execute(pool.get_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("GPG key not found".to_string()));
    }

    info!("User {} deleted GPG key {}", auth.user_id, key_id);

    Ok(HttpResponse::NoContent().finish())
}

/// Revoke a GPG key (mark as revoked without deleting)
/// POST /api/v1/user/gpg_keys/{id}/revoke
pub async fn revoke_gpg_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let key_id = path.into_inner();

    let result = sqlx::query(
        "UPDATE gpg_keys SET revoked = true WHERE id = $1 AND user_id = $2",
    )
    .bind(key_id)
    .bind(auth.user_id)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("GPG key not found".to_string()));
    }

    info!("User {} revoked GPG key {}", auth.user_id, key_id);

    // Also mark the key as revoked in signature cache
    sqlx::query(
        "UPDATE gpg_signatures SET verification_status = 'revoked_key' WHERE gpg_key_id = $1",
    )
    .bind(key_id)
    .execute(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "GPG key has been revoked"
    })))
}

/// Parse a GPG public key using gpg command
fn parse_gpg_key(key_data: &str) -> Result<ParsedGpgKey, AppError> {
    // Create a temporary keyring to import the key
    let temp_dir = tempfile::tempdir().map_err(|e| {
        error!("Failed to create temp directory: {}", e);
        AppError::InternalError("Failed to process GPG key".to_string())
    })?;

    let gpg_home = temp_dir.path();

    // Import the key into temporary keyring
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
            warn!("Failed to run gpg import: {}", e);
            AppError::InternalError("Failed to validate GPG key (gpg not available)".to_string())
        })?;

    {
        let stdin = import_cmd.stdin.as_mut().ok_or_else(|| {
            AppError::InternalError("Failed to get gpg stdin".to_string())
        })?;
        stdin.write_all(key_data.as_bytes()).map_err(|e| {
            AppError::InternalError(format!("Failed to write to gpg: {}", e))
        })?;
    }

    let import_output = import_cmd.wait_with_output().map_err(|e| {
        AppError::InternalError(format!("Failed to wait for gpg import: {}", e))
    })?;

    if !import_output.status.success() {
        let stderr = String::from_utf8_lossy(&import_output.stderr);
        debug!("GPG import failed: {}", stderr);
        return Err(AppError::BadRequest(
            "Invalid GPG key: could not import".to_string(),
        ));
    }

    // List keys with colon-delimited output for parsing
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

    if !list_output.status.success() {
        return Err(AppError::InternalError(
            "Failed to parse GPG key information".to_string(),
        ));
    }

    let output = String::from_utf8_lossy(&list_output.stdout);
    parse_gpg_colon_output(&output)
}

/// Parse GPG --with-colons output format
/// See https://git.gnupg.org/cgi-bin/gitweb.cgi?p=gnupg.git;a=blob_plain;f=doc/DETAILS
fn parse_gpg_colon_output(output: &str) -> Result<ParsedGpgKey, AppError> {
    let mut fingerprint = String::new();
    let mut primary_key_id = String::new();
    let mut algorithm = String::new();
    let mut key_size: Option<i32> = None;
    let mut created_at: Option<DateTime<Utc>> = None;
    let mut expires_at: Option<DateTime<Utc>> = None;
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
                // Primary key: pub:validity:key_length:algorithm:key_id:creation_date:expiration_date:....:capabilities
                if fields.len() > 11 {
                    key_size = fields[2].parse().ok();
                    algorithm = algorithm_from_code(fields[3]);
                    primary_key_id = fields[4].to_string();

                    if let Ok(ts) = fields[5].parse::<i64>() {
                        created_at = Utc.timestamp_opt(ts, 0).single();
                    }

                    if !fields[6].is_empty() {
                        if let Ok(ts) = fields[6].parse::<i64>() {
                            expires_at = Utc.timestamp_opt(ts, 0).single();
                        }
                    }

                    // Parse capabilities (field 11)
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
                // Fingerprint line
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
                // User ID: uid:validity:creation_date:expiration_date:hash:....:user_id_string
                if fields.len() > 9 {
                    let uid = fields[9];
                    // Extract email from uid (format: "Name <email@example.com>")
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
                // Subkey: same format as pub
                // Save previous subkey if any
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
                    sk.algorithm = algorithm_from_code(fields[3]);
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

    // Don't forget the last subkey
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
        "2" => "RSA".to_string(), // RSA Encrypt-Only (deprecated)
        "3" => "RSA".to_string(), // RSA Sign-Only (deprecated)
        "16" => "Elgamal".to_string(),
        "17" => "DSA".to_string(),
        "18" => "ECDH".to_string(),
        "19" => "ECDSA".to_string(),
        "22" => "EdDSA".to_string(),
        _ => format!("Unknown({})", code),
    }
}

/// Find GPG key by key ID or fingerprint (for internal use)
pub async fn find_gpg_key_by_id(
    pool: &PgPool,
    key_id: &str,
) -> Result<Option<(GpgKey, Option<GpgKeySubkey>)>, AppError> {
    // First try to find by primary key fingerprint or ID
    let key = sqlx::query_as::<_, GpgKey>(
        r#"
        SELECT * FROM gpg_keys 
        WHERE fingerprint = $1 
           OR primary_key_id = $1
           OR fingerprint LIKE '%' || $1
        "#,
    )
    .bind(key_id)
    .fetch_optional(pool)
    .await?;

    if let Some(key) = key {
        return Ok(Some((key, None)));
    }

    // Try to find by subkey
    let subkey = sqlx::query_as::<_, GpgKeySubkey>(
        r#"
        SELECT * FROM gpg_key_subkeys
        WHERE fingerprint = $1
           OR key_id = $1
           OR fingerprint LIKE '%' || $1
        "#,
    )
    .bind(key_id)
    .fetch_optional(pool)
    .await?;

    if let Some(subkey) = subkey {
        let key = sqlx::query_as::<_, GpgKey>(
            r#"SELECT * FROM gpg_keys WHERE id = $1"#,
        )
        .bind(subkey.gpg_key_id)
        .fetch_one(pool)
        .await?;

        return Ok(Some((key, Some(subkey))));
    }

    Ok(None)
}

/// Verify GPG signature (called by GitLayer)
pub async fn verify_gpg_signature(
    pool: &PgPool,
    signature: &str,
    signed_data: &str,
    committer_email: &str,
) -> Result<(bool, String, Option<String>, Option<i64>, Option<String>), AppError> {
    // Create temp directory for GPG operation
    let temp_dir = tempfile::tempdir().map_err(|e| {
        error!("Failed to create temp directory: {}", e);
        AppError::InternalError("Failed to verify signature".to_string())
    })?;

    let gpg_home = temp_dir.path();

    // Get all GPG keys from the database to import into temp keyring
    let keys: Vec<String> = sqlx::query_scalar("SELECT public_key FROM gpg_keys WHERE revoked = false")
        .fetch_all(pool)
        .await?;

    // Import all keys
    for key in &keys {
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
            let _ = stdin.write_all(key.as_bytes());
        }
        let _ = import_cmd.wait();
    }

    // Write the signed data to a temp file
    let data_file = temp_dir.path().join("data");
    std::fs::write(&data_file, signed_data).map_err(|e| {
        AppError::InternalError(format!("Failed to write temp file: {}", e))
    })?;

    // Write the signature to a temp file
    let sig_file = temp_dir.path().join("sig");
    std::fs::write(&sig_file, signature).map_err(|e| {
        AppError::InternalError(format!("Failed to write temp file: {}", e))
    })?;

    // Verify the signature
    let verify_output = Command::new("gpg")
        .args([
            "--homedir",
            gpg_home.to_str().unwrap(),
            "--batch",
            "--status-fd",
            "1",
            "--verify",
            sig_file.to_str().unwrap(),
            data_file.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| {
            AppError::InternalError(format!("Failed to run gpg verify: {}", e))
        })?;

    let stdout = String::from_utf8_lossy(&verify_output.stdout);
    let stderr = String::from_utf8_lossy(&verify_output.stderr);

    debug!("GPG verify stdout: {}", stdout);
    debug!("GPG verify stderr: {}", stderr);

    // Parse the verification result
    let mut key_id = None;
    let mut good_sig = false;

    for line in stdout.lines() {
        if line.contains("[GNUPG:] GOODSIG") {
            good_sig = true;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 2 {
                key_id = Some(parts[2].to_string());
            }
        } else if line.contains("[GNUPG:] VALIDSIG") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 2 && key_id.is_none() {
                key_id = Some(parts[2].to_string());
            }
        }
    }

    if !good_sig {
        // Check if signature is bad or key is unknown
        if stdout.contains("NODATA") || stdout.contains("NO_PUBKEY") {
            return Ok((false, "unknown_key".to_string(), key_id, None, None));
        }
        if stdout.contains("BADSIG") {
            return Ok((false, "bad_signature".to_string(), key_id, None, None));
        }
        if stdout.contains("EXPKEYSIG") {
            return Ok((false, "expired_key".to_string(), key_id, None, None));
        }
        if stdout.contains("REVKEYSIG") {
            return Ok((false, "revoked_key".to_string(), key_id, None, None));
        }
        return Ok((false, "unknown_key".to_string(), key_id, None, None));
    }

    // Signature is valid, now check if the key belongs to a user and verify email
    if let Some(ref kid) = key_id {
        if let Ok(Some((gpg_key, _subkey))) = find_gpg_key_by_id(pool, kid).await {
            // Get the user
            let user: Option<(i64, String)> = sqlx::query_as(
                "SELECT id, username FROM users WHERE id = $1",
            )
            .bind(gpg_key.user_id)
            .fetch_optional(pool)
            .await?;

            if let Some((user_id, username)) = user {
                // Check if committer email matches any email in the key
                let email_matches = gpg_key
                    .emails
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(committer_email));

                if gpg_key.revoked {
                    return Ok((false, "revoked_key".to_string(), key_id, Some(user_id), Some(username)));
                }

                if let Some(expires) = gpg_key.key_expires_at {
                    if expires < Utc::now() {
                        return Ok((false, "expired_key".to_string(), key_id, Some(user_id), Some(username)));
                    }
                }

                if email_matches && gpg_key.verified {
                    return Ok((true, "verified".to_string(), key_id, Some(user_id), Some(username)));
                } else if email_matches {
                    return Ok((true, "unverified".to_string(), key_id, Some(user_id), Some(username)));
                } else {
                    return Ok((false, "bad_email".to_string(), key_id, Some(user_id), Some(username)));
                }
            }
        }
    }

    // Valid signature but key not in our system
    Ok((true, "unverified".to_string(), key_id, None, None))
}
