use actix_web::{web, HttpResponse};
use chrono::{Duration, Utc};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::AuthenticatedUser;
use crate::models::{
    CreatePatRequest, CreatePersonalAccessTokenResponse, PersonalAccessToken,
    PersonalAccessTokenInfo, PatScope, PAT_PREFIX,
};

/// Generate a secure random token
fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    format!("{}{}", PAT_PREFIX, hex::encode(bytes))
}

/// Hash a token using SHA-256
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// GET /api/v1/user/access_tokens - List user's personal access tokens
pub async fn list_tokens(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let tokens = sqlx::query_as::<_, PersonalAccessToken>(
        r#"
        SELECT * FROM personal_access_tokens 
        WHERE user_id = $1 
        ORDER BY created_at DESC
        "#
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await?;

    let token_infos: Vec<PersonalAccessTokenInfo> = tokens.into_iter().map(|t| t.into()).collect();
    Ok(HttpResponse::Ok().json(token_infos))
}

/// POST /api/v1/user/access_tokens - Create a new personal access token
pub async fn create_token(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    auth: AuthenticatedUser,
    body: web::Json<CreatePatRequest>,
) -> AppResult<HttpResponse> {
    let req = body.into_inner();

    // Validate scopes
    let scopes: Vec<String> = if let Some(scope_list) = req.scopes {
        // Validate each scope
        for scope in &scope_list {
            if PatScope::from_str(scope).is_none() {
                return Err(AppError::BadRequest(format!("Invalid scope: {}", scope)));
            }
        }
        scope_list
    } else {
        // Use default scopes
        PatScope::defaults().iter().map(|s| s.as_str().to_string()).collect()
    };

    // Calculate expiration
    let expires_at = match req.expires_in_days {
        Some(0) => None, // Never expires
        Some(days) => {
            // Check against max expiration
            if config.pat_max_expiration_days > 0 && days > config.pat_max_expiration_days {
                return Err(AppError::BadRequest(format!(
                    "Expiration cannot exceed {} days",
                    config.pat_max_expiration_days
                )));
            }
            Some(Utc::now() + Duration::days(days as i64))
        }
        None => {
            // Use default expiration
            if config.pat_default_expiration_days > 0 {
                Some(Utc::now() + Duration::days(config.pat_default_expiration_days as i64))
            } else {
                None
            }
        }
    };

    // Generate token
    let raw_token = generate_token();
    let token_hash = hash_token(&raw_token);
    let token_last_four = raw_token.chars().skip(raw_token.len() - 4).collect::<String>();

    // Insert into database
    let pat = sqlx::query_as::<_, PersonalAccessToken>(
        r#"
        INSERT INTO personal_access_tokens 
            (user_id, name, token_hash, token_last_four, scopes, expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        RETURNING *
        "#
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&token_hash)
    .bind(&token_last_four)
    .bind(&scopes)
    .bind(expires_at)
    .fetch_one(pool.get_ref())
    .await?;

    // Return response with raw token (only time it's returned!)
    Ok(HttpResponse::Created().json(CreatePersonalAccessTokenResponse {
        id: pat.id,
        name: pat.name,
        token: raw_token,
        scopes: pat.scopes,
        expires_at: pat.expires_at,
        created_at: pat.created_at,
    }))
}

/// GET /api/v1/user/access_tokens/{id} - Get a specific token's info
pub async fn get_token(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let token_id = path.into_inner();

    let token = sqlx::query_as::<_, PersonalAccessToken>(
        "SELECT * FROM personal_access_tokens WHERE id = $1 AND user_id = $2"
    )
    .bind(token_id)
    .bind(auth.user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Token not found".to_string()))?;

    Ok(HttpResponse::Ok().json(PersonalAccessTokenInfo::from(token)))
}

/// DELETE /api/v1/user/access_tokens/{id} - Revoke a personal access token
pub async fn revoke_token(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let token_id = path.into_inner();

    let result = sqlx::query(
        r#"
        UPDATE personal_access_tokens 
        SET revoked_at = NOW() 
        WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL
        "#
    )
    .bind(token_id)
    .bind(auth.user_id)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Token not found or already revoked".to_string()));
    }

    Ok(HttpResponse::NoContent().finish())
}

/// GET /api/v1/user/access_tokens/scopes - Get available scopes
pub async fn list_scopes() -> AppResult<HttpResponse> {
    let scopes: Vec<serde_json::Value> = PatScope::all()
        .iter()
        .map(|s| serde_json::json!({
            "name": s.as_str(),
            "description": match s {
                PatScope::ReadApi => "Read access to the API",
                PatScope::WriteApi => "Write access to the API (includes read)",
                PatScope::ReadRepository => "Clone and fetch repositories",
                PatScope::WriteRepository => "Push to repositories",
                PatScope::ReadUser => "Read your user profile",
                PatScope::WriteUser => "Modify your user profile",
                PatScope::ReadRegistry => "Read container and package registries",
                PatScope::WriteRegistry => "Push to container and package registries",
                PatScope::Admin => "Full administrative access",
            }
        }))
        .collect();

    Ok(HttpResponse::Ok().json(scopes))
}

// ─────────────────────────────────────────────────────────────────────────────
// Token Validation (used by auth middleware and git_http)
// ─────────────────────────────────────────────────────────────────────────────

/// Validate a PAT and return user_id if valid
pub async fn validate_pat(pool: &PgPool, token: &str) -> AppResult<Option<(i64, Vec<String>)>> {
    // Check if it's a PAT
    if !token.starts_with(PAT_PREFIX) {
        return Ok(None);
    }

    let token_hash = hash_token(token);

    let result = sqlx::query_as::<_, (i64, Vec<String>)>(
        r#"
        SELECT user_id, scopes FROM personal_access_tokens
        WHERE token_hash = $1 
          AND revoked_at IS NULL
          AND (expires_at IS NULL OR expires_at > NOW())
        "#
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    if let Some((user_id, scopes)) = result {
        // Update last_used_at
        sqlx::query("UPDATE personal_access_tokens SET last_used_at = NOW() WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(pool)
            .await?;

        return Ok(Some((user_id, scopes)));
    }

    Ok(None)
}

/// Check if a PAT has a specific scope
pub fn has_scope(scopes: &[String], required: &str) -> bool {
    // Admin scope grants everything
    if scopes.contains(&"admin".to_string()) {
        return true;
    }

    // write_* implies read_*
    if required.starts_with("read_") {
        let write_equivalent = required.replace("read_", "write_");
        if scopes.contains(&write_equivalent) {
            return true;
        }
    }

    scopes.contains(&required.to_string())
}
