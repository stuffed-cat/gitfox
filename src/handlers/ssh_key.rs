//! SSH Key handlers for user-facing API

use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use std::process::{Command, Stdio};
use std::io::Write;
use log::{debug, info, warn};
use validator::Validate;

use crate::error::AppError;
use crate::middleware::AuthenticatedUser;
use crate::models::{CreateSshKeyRequest, SshKey, SshKeyResponse};

/// List SSH keys for the authenticated user
/// GET /api/v1/user/ssh_keys
pub async fn list_ssh_keys(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let keys = sqlx::query_as::<_, SshKey>(
        r#"SELECT * FROM ssh_keys WHERE user_id = $1 ORDER BY created_at DESC"#
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await?;

    let response: Vec<SshKeyResponse> = keys.into_iter().map(Into::into).collect();
    Ok(HttpResponse::Ok().json(response))
}

/// Get a specific SSH key
/// GET /api/v1/user/ssh_keys/{id}
pub async fn get_ssh_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let key_id = path.into_inner();

    let key = sqlx::query_as::<_, SshKey>(
        r#"SELECT * FROM ssh_keys WHERE id = $1 AND user_id = $2"#
    )
    .bind(key_id)
    .bind(auth.user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("SSH key not found".to_string()))?;

    let mut response: SshKeyResponse = key.into();
    // Include the full key when viewing a specific key
    response.public_key = Some(
        sqlx::query_scalar::<_, String>("SELECT public_key FROM ssh_keys WHERE id = $1")
            .bind(key_id)
            .fetch_one(pool.get_ref())
            .await?,
    );

    Ok(HttpResponse::Ok().json(response))
}

/// Add a new SSH key
/// POST /api/v1/user/ssh_keys
pub async fn create_ssh_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    body: web::Json<CreateSshKeyRequest>,
) -> Result<HttpResponse, AppError> {
    body.validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let key_data = body.key.trim();

    // Parse and validate the SSH key
    let (key_type, fingerprint) = parse_ssh_key(key_data)?;

    // Check if key already exists
    let existing = sqlx::query_scalar::<_, i64>(
        r#"SELECT id FROM ssh_keys WHERE fingerprint = $1"#
    )
    .bind(&fingerprint)
    .fetch_optional(pool.get_ref())
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "This SSH key is already registered".to_string(),
        ));
    }

    // Insert the new key
    let key = sqlx::query_as::<_, SshKey>(
        r#"
        INSERT INTO ssh_keys (user_id, title, key_type, public_key, fingerprint)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&body.title)
    .bind(&key_type)
    .bind(key_data)
    .bind(&fingerprint)
    .fetch_one(pool.get_ref())
    .await?;

    info!(
        "User {} added SSH key {} ({})",
        auth.user_id, key.id, fingerprint
    );

    Ok(HttpResponse::Created().json(SshKeyResponse::from(key)))
}

/// Delete an SSH key
/// DELETE /api/v1/user/ssh_keys/{id}
pub async fn delete_ssh_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let key_id = path.into_inner();

    let result = sqlx::query("DELETE FROM ssh_keys WHERE id = $1 AND user_id = $2")
        .bind(key_id)
        .bind(auth.user_id)
        .execute(pool.get_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("SSH key not found".to_string()));
    }

    info!("User {} deleted SSH key {}", auth.user_id, key_id);

    Ok(HttpResponse::NoContent().finish())
}

/// Parse and validate an SSH public key
fn parse_ssh_key(key: &str) -> Result<(String, String), AppError> {
    let parts: Vec<&str> = key.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(AppError::BadRequest(
            "Invalid SSH key format".to_string(),
        ));
    }

    let key_type = parts[0].to_string();

    // Validate key type
    let valid_types = [
        "ssh-rsa",
        "ssh-dss",
        "ssh-ed25519",
        "ecdsa-sha2-nistp256",
        "ecdsa-sha2-nistp384",
        "ecdsa-sha2-nistp521",
        "sk-ssh-ed25519@openssh.com",
        "sk-ecdsa-sha2-nistp256@openssh.com",
    ];

    if !valid_types.contains(&key_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Unsupported SSH key type: {}",
            key_type
        )));
    }

    // Compute fingerprint using ssh-keygen
    let fingerprint = compute_fingerprint(key)?;

    Ok((key_type, fingerprint))
}

/// Compute the fingerprint of an SSH key using ssh-keygen
fn compute_fingerprint(key: &str) -> Result<String, AppError> {
    let mut child = Command::new("ssh-keygen")
        .args(["-l", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            warn!("Failed to run ssh-keygen: {}", e);
            AppError::InternalError("Failed to validate SSH key".to_string())
        })?;

    {
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            AppError::InternalError("Failed to get stdin".to_string())
        })?;
        stdin.write_all(key.as_bytes()).map_err(|e| {
            AppError::InternalError(format!("Failed to write to ssh-keygen: {}", e))
        })?;
    }

    let output = child.wait_with_output().map_err(|e| {
        AppError::InternalError(format!("Failed to wait for ssh-keygen: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("ssh-keygen failed: {}", stderr);
        return Err(AppError::BadRequest(
            "Invalid SSH key: failed to compute fingerprint".to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse fingerprint from: "256 SHA256:xxx user@host (ED25519)"
    stdout
        .split_whitespace()
        .nth(1)
        .map(|s| s.to_string())
        .ok_or_else(|| {
            AppError::InternalError("Failed to parse fingerprint".to_string())
        })
}
