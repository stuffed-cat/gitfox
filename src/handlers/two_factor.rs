use actix_web::{web, HttpResponse};
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use validator::Validate;
use webauthn_rs::prelude::*;

use crate::config::Config;
use crate::error::AppResult;
use crate::middleware::auth::AuthenticatedUser;
use crate::models::{
    DisableTotpRequest, EnableTotpRequest, TwoFactorStatus, UserTotp,
    UserWebAuthnCredential, WebAuthnCredentialInfo, WebAuthnRegisterFinishRequest,
    WebAuthnRegisterStartRequest,
};
use crate::services::two_factor;

/// GET /api/v1/user/two-factor/status
/// Get 2FA status for current user
pub async fn get_two_factor_status(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    // Check TOTP status
    let totp_enabled = sqlx::query_scalar::<_, bool>(
        "SELECT enabled FROM user_totp WHERE user_id = $1"
    )
    .bind(auth.user_id)
    .fetch_optional(pool.as_ref())
    .await?
    .unwrap_or(false);
    
    // Get WebAuthn credentials
    let credentials = sqlx::query_as::<_, UserWebAuthnCredential>(
        "SELECT * FROM user_webauthn_credentials WHERE user_id = $1 ORDER BY created_at DESC"
    )
    .bind(auth.user_id)
    .fetch_all(pool.as_ref())
    .await?;
    
    let webauthn_credentials: Vec<WebAuthnCredentialInfo> = credentials
        .into_iter()
        .map(|c| WebAuthnCredentialInfo {
            id: c.id,
            name: c.name,
            created_at: c.created_at,
            last_used_at: c.last_used_at,
        })
        .collect();
    
    // Count unused recovery codes
    let recovery_codes_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_recovery_codes WHERE user_id = $1 AND used = false"
    )
    .bind(auth.user_id)
    .fetch_one(pool.as_ref())
    .await?;
    
    // Check if any 2FA method is enabled
    let enabled = totp_enabled || !webauthn_credentials.is_empty();
    
    let status = TwoFactorStatus {
        enabled,
        totp_enabled,
        webauthn_credentials,
        recovery_codes_count,
    };
    
    Ok(HttpResponse::Ok().json(status))
}

/// POST /api/v1/user/two-factor/totp/setup
/// Setup TOTP for current user
pub async fn setup_totp(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let response = two_factor::setup_totp(pool.as_ref(), redis.as_ref(), auth.user_id, &auth.username).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// POST /api/v1/user/two-factor/totp/enable
/// Enable TOTP after verifying code
pub async fn enable_totp(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    auth: AuthenticatedUser,
    req: web::Json<EnableTotpRequest>,
) -> AppResult<HttpResponse> {
    req.validate()?;
    
    two_factor::enable_totp(pool.as_ref(), redis.as_ref(), auth.user_id, &req.state_key, &req.totp_code).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "TOTP enabled successfully"
    })))
}

/// POST /api/v1/user/two-factor/totp/disable
/// Disable TOTP
pub async fn disable_totp(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    req: web::Json<DisableTotpRequest>,
) -> AppResult<HttpResponse> {
    req.validate()?;
    
    two_factor::disable_totp(pool.as_ref(), auth.user_id, &req.totp_code).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "TOTP disabled successfully"
    })))
}

/// POST /api/v1/user/two-factor/recovery-codes/regenerate
/// Regenerate recovery codes
pub async fn regenerate_recovery_codes(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    // Verify user has 2FA enabled
    let totp = sqlx::query_as::<_, UserTotp>(
        "SELECT * FROM user_totp WHERE user_id = $1 AND enabled = true"
    )
    .bind(auth.user_id)
    .fetch_optional(pool.as_ref())
    .await?;
    
    if totp.is_none() {
        return Err(crate::error::AppError::BadRequest(
            "2FA must be enabled to regenerate recovery codes".to_string()
        ));
    }
    
    let response = two_factor::regenerate_recovery_codes(pool.as_ref(), auth.user_id).await?;
    
    Ok(HttpResponse::Ok().json(response))
}

/// GET /api/v1/user/two-factor/recovery-codes/count
/// Get count of unused recovery codes
pub async fn get_recovery_codes_count(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_recovery_codes WHERE user_id = $1 AND used = false"
    )
    .bind(auth.user_id)
    .fetch_one(pool.as_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "count": count
    })))
}

/// POST /api/v1/user/two-factor/webauthn/register/start
/// Start WebAuthn registration
pub async fn webauthn_register_start(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<Config>,
    auth: AuthenticatedUser,
    _req: web::Json<WebAuthnRegisterStartRequest>,
) -> AppResult<HttpResponse> {
    
    let webauthn = two_factor::create_webauthn(&config)?;
    
    // Get existing credentials to exclude them
    let existing = sqlx::query_as::<_, UserWebAuthnCredential>(
        "SELECT * FROM user_webauthn_credentials WHERE user_id = $1"
    )
    .bind(auth.user_id)
    .fetch_all(pool.as_ref())
    .await?;
    
    let mut exclude_credentials = Vec::new();
    for cred in existing {
        exclude_credentials.push(CredentialID::from(cred.credential_id));
    }
    
    let (ccr, reg_state) = two_factor::start_webauthn_registration(
        &webauthn,
        auth.user_id,
        &auth.username,
        exclude_credentials,
    )?;
    
    // Store registration state in Redis (5 minute expiration)
    let state_key = format!("webauthn:reg:{}:{}", auth.user_id, chrono::Utc::now().timestamp());
    let state_json = serde_json::to_string(&reg_state)
        .map_err(|e| crate::error::AppError::InternalError(format!("Failed to serialize state: {}", e)))?;
    
    let mut conn = redis.get().await
        .map_err(|e| crate::error::AppError::InternalError(format!("Redis connection error: {}", e)))?;
    
    redis::cmd("SETEX")
        .arg(&state_key)
        .arg(300) // 5 minutes
        .arg(&state_json)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::InternalError(format!("Redis error: {}", e)))?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "challenge": ccr,
        "state_key": state_key,
    })))
}

/// POST /api/v1/user/two-factor/webauthn/register/finish
/// Finish WebAuthn registration
pub async fn webauthn_register_finish(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<Config>,
    auth: AuthenticatedUser,
    req: web::Json<WebAuthnRegisterFinishRequest>,
) -> AppResult<HttpResponse> {
    req.validate()?;
    
    let webauthn = two_factor::create_webauthn(&config)?;
    
    // Get registration state from Redis
    let mut conn = redis.get().await
        .map_err(|e| crate::error::AppError::InternalError(format!("Redis connection error: {}", e)))?;
    
    let state_json: Option<String> = redis::cmd("GET")
        .arg(&req.state_key)
        .query_async(&mut conn)
        .await
        .map_err(|e| crate::error::AppError::InternalError(format!("Redis error: {}", e)))?;
    
    let state_json = state_json
        .ok_or_else(|| crate::error::AppError::BadRequest("Registration session expired or invalid".to_string()))?;
    
    let reg_state: PasskeyRegistration = serde_json::from_str(&state_json)
        .map_err(|e| crate::error::AppError::InternalError(format!("Failed to deserialize state: {}", e)))?;
    
    // Delete the state from Redis
    redis::cmd("DEL")
        .arg(&req.state_key)
        .query_async::<_, ()>(&mut conn)
        .await
        .ok();
    
    // Finish registration
    two_factor::finish_webauthn_registration(
        pool.as_ref(),
        &webauthn,
        auth.user_id,
        &req.credential,
        &reg_state,
        &req.name,
    ).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "WebAuthn credential registered successfully"
    })))
}

/// DELETE /api/v1/user/two-factor/webauthn/{id}
/// Delete a WebAuthn credential
pub async fn delete_webauthn_credential(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let credential_id = path.into_inner();
    
    two_factor::delete_webauthn_credential(pool.as_ref(), auth.user_id, credential_id).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Credential deleted successfully"
    })))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/user/two-factor")
            .route("/status", web::get().to(get_two_factor_status))
            .route("/totp/setup", web::post().to(setup_totp))
            .route("/totp/enable", web::post().to(enable_totp))
            .route("/totp/disable", web::post().to(disable_totp))
            .route("/recovery-codes/regenerate", web::post().to(regenerate_recovery_codes))
            .route("/recovery-codes/count", web::get().to(get_recovery_codes_count))
            .route("/webauthn/register/start", web::post().to(webauthn_register_start))
            .route("/webauthn/register/finish", web::post().to(webauthn_register_finish))
            .route("/webauthn/{id}", web::delete().to(delete_webauthn_credential))
    );
}
