use actix_web::{web, HttpRequest, HttpResponse};
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use uuid::Uuid;
use validator::Validate;
use webauthn_rs::prelude::*;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::validate_token;
use crate::models::{
    Claims, CreateUserRequest, LoginRequest, LoginResponse, TwoFactorRequiredResponse,
    UserInfo, VerifyTwoFactorRequest, WebAuthnAuthStartRequest, WebAuthnAuthFinishRequest,
    TokenScope,
};
use crate::services::{two_factor, SmtpService, SystemConfigService, UserService};

pub async fn register(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
    body: web::Json<CreateUserRequest>,
) -> AppResult<HttpResponse> {
    let user = UserService::create_user(pool.get_ref(), body.into_inner()).await?;
    
    // Check if email confirmation is required
    let require_email_confirmation = SystemConfigService::get(pool.get_ref(), redis.get_ref(), "require_email_confirmation")
        .await
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    
    if require_email_confirmation && config.smtp.is_configured() {
        // Generate confirmation token and send email
        let token = UserService::generate_email_confirmation_token(pool.get_ref(), user.id).await?;
        
        if let Err(e) = SmtpService::send_email_confirmation(
            &config.smtp,
            &user.email,
            &user.username,
            &token,
            &config.base_url,
        ).await {
            log::error!("Failed to send confirmation email: {}", e);
            // Don't fail registration if email fails, but log it
        }
    }
    
    Ok(HttpResponse::Created().json(UserInfo::from(user)))
}

pub async fn login(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
    body: web::Json<LoginRequest>,
) -> AppResult<HttpResponse> {
    // First, verify username and password
    let user = sqlx::query_as::<_, crate::models::User>(
        "SELECT * FROM users WHERE username = $1 AND is_active = true"
    )
    .bind(&body.username)
    .fetch_optional(pool.as_ref())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    if !bcrypt::verify(&body.password, &user.password_hash)
        .map_err(|e| AppError::InternalError(format!("Password verification failed: {}", e)))? 
    {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }
    
    // Check if email confirmation is required and user hasn't confirmed
    let require_email_confirmation = SystemConfigService::get(pool.get_ref(), redis.get_ref(), "require_email_confirmation")
        .await
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    
    if require_email_confirmation && !user.email_confirmed {
        return Err(AppError::Unauthorized("Please confirm your email address before logging in".to_string()));
    }
    
    // Check if 2FA is required by system policy
    let require_2fa_all = SystemConfigService::get(pool.get_ref(), redis.get_ref(), "require_two_factor")
        .await
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    
    let require_2fa_admin = SystemConfigService::get(pool.get_ref(), redis.get_ref(), "require_two_factor_admin")
        .await
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    
    let is_admin = user.role == crate::models::UserRole::Admin;
    let two_fa_required_by_policy = require_2fa_all || (is_admin && require_2fa_admin);
    
    // If 2FA is required but not enabled, check grace period
    if two_fa_required_by_policy && !user.two_factor_enabled {
        let grace_period_days: i64 = SystemConfigService::get(pool.get_ref(), redis.get_ref(), "two_factor_grace_period_days")
            .await
            .ok()
            .and_then(|v| v.as_i64())
            .unwrap_or(7);
        
        // Set two_factor_required_at if not already set
        if user.two_factor_required_at.is_none() {
            sqlx::query("UPDATE users SET two_factor_required_at = NOW() WHERE id = $1")
                .bind(user.id)
                .execute(pool.as_ref())
                .await?;
        }
        
        // Check if grace period has expired
        if let Some(required_at) = user.two_factor_required_at {
            let grace_period_end = required_at + chrono::Duration::days(grace_period_days);
            let now = Utc::now();
            
            if now > grace_period_end {
                // Grace period has expired, deny login
                return Err(AppError::Unauthorized(
                    "Two-factor authentication is required. Please contact your administrator.".to_string()
                ));
            } else {
                // Grace period is still active, allow login but warn user
                log::warn!("User {} is in 2FA grace period (expires at {})", user.username, grace_period_end);
            }
        }
    }
    
    // Check if 2FA is enabled
    if user.two_factor_enabled {
        // Determine available 2FA methods
        let mut available_methods = Vec::new();
        
        // Check TOTP
        let totp_enabled: bool = sqlx::query_scalar(
            "SELECT enabled FROM user_totp WHERE user_id = $1"
        )
        .bind(user.id)
        .fetch_optional(pool.as_ref())
        .await?
        .unwrap_or(false);
        
        if totp_enabled {
            available_methods.push("totp".to_string());
        }
        
        // Check WebAuthn
        let webauthn_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_webauthn_credentials WHERE user_id = $1"
        )
        .bind(user.id)
        .fetch_one(pool.as_ref())
        .await?;
        
        if webauthn_count > 0 {
            available_methods.push("webauthn".to_string());
        }
        
        // Recovery codes are always available if 2FA is enabled
        available_methods.push("recovery".to_string());
        
        // Generate a temporary token for 2FA verification (valid for 5 minutes)
        let temporary_token = Uuid::new_v4().to_string();
        
        // Store user ID in Redis with the temporary token (expires in 5 minutes)
        let mut redis_conn = redis.get().await
            .map_err(|e| AppError::InternalError(format!("Redis connection failed: {}", e)))?;
        
        deadpool_redis::redis::cmd("SETEX")
            .arg(format!("2fa:{}", temporary_token))
            .arg(300) // 5 minutes
            .arg(user.id)
            .query_async::<_, ()>(&mut redis_conn)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to store 2FA session: {}", e)))?;
        
        return Ok(HttpResponse::Ok().json(TwoFactorRequiredResponse {
            requires_two_factor: true,
            available_methods,
            temporary_token,
        }));
    }
    
    // No 2FA required, generate regular JWT and login
    let response = UserService::login(pool.get_ref(), config.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// POST /api/v1/auth/verify-two-factor - Complete 2FA verification
pub async fn verify_two_factor(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
    body: web::Json<VerifyTwoFactorRequest>,
) -> AppResult<HttpResponse> {
    body.validate()?;
    
    // Get user ID from Redis using the temporary token
    let mut redis_conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection failed: {}", e)))?;
    
    let user_id: Option<i64> = deadpool_redis::redis::cmd("GET")
        .arg(format!("2fa:{}", body.temporary_token))
        .query_async(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get 2FA session: {}", e)))?;
    
    let user_id = user_id.ok_or_else(|| {
        AppError::Unauthorized("Invalid or expired 2FA token".to_string())
    })?;
    
    // Check if account is locked due to too many 2FA failures
    if crate::middleware::is_2fa_locked(redis.get_ref(), user_id).await? {
        return Err(AppError::TooManyRequests(
            "Account temporarily locked due to too many failed 2FA attempts. Please try again in 30 minutes.".to_string()
        ));
    }
    
    // Check rate limit (5 attempts per minute)
    crate::middleware::check_2fa_rate_limit(redis.get_ref(), user_id).await?;
    
    // Verify based on method
    let valid = match body.method.as_str() {
        "totp" => {
            let code = body.code.as_ref()
                .ok_or_else(|| AppError::BadRequest("TOTP code required".to_string()))?;
            two_factor::verify_totp_login(pool.as_ref(), user_id, code).await?
        },
        "recovery" => {
            let code = body.code.as_ref()
                .ok_or_else(|| AppError::BadRequest("Recovery code required".to_string()))?;
            two_factor::use_recovery_code(pool.as_ref(), user_id, code).await?
        },
        "webauthn" => {
            // WebAuthn verification requires a multi-step challenge-response process.
            // The webauthn_response field should contain the serialized PublicKeyCredential.
            // However, WebAuthn authentication needs a prior challenge (from webauthn/start).
            // If client provided state_key and credential, we can verify directly.
            let webauthn_json = body.webauthn_response.as_ref()
                .ok_or_else(|| AppError::BadRequest("WebAuthn response required".to_string()))?;
            
            // Parse the WebAuthn response (contains state_key and credential)
            let webauthn_data: serde_json::Value = serde_json::from_str(webauthn_json)
                .map_err(|e| AppError::BadRequest(format!("Invalid WebAuthn response: {}", e)))?;
            
            let state_key = webauthn_data.get("state_key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::BadRequest("Missing state_key in WebAuthn response".to_string()))?;
            
            let credential_json = webauthn_data.get("credential")
                .ok_or_else(|| AppError::BadRequest("Missing credential in WebAuthn response".to_string()))?;
            
            // Get authentication state from Redis
            let state_json: Option<String> = deadpool_redis::redis::cmd("GET")
                .arg(state_key)
                .query_async(&mut redis_conn)
                .await
                .map_err(|e| AppError::InternalError(format!("Redis error: {}", e)))?;
            
            let state_json = state_json
                .ok_or_else(|| AppError::BadRequest("WebAuthn session expired. Please restart authentication.".to_string()))?;
            
            // Delete the state from Redis
            deadpool_redis::redis::cmd("DEL")
                .arg(state_key)
                .query_async::<_, ()>(&mut redis_conn)
                .await
                .ok();
            
            let auth_state: webauthn_rs::prelude::PasskeyAuthentication = serde_json::from_str(&state_json)
                .map_err(|e| AppError::InternalError(format!("Failed to deserialize state: {}", e)))?;
            
            // Parse credential
            let credential: webauthn_rs::prelude::PublicKeyCredential = serde_json::from_value(credential_json.clone())
                .map_err(|e| AppError::BadRequest(format!("Invalid credential format: {}", e)))?;
            
            // Create WebAuthn instance and verify
            let webauthn = two_factor::create_webauthn(&config)?;
            two_factor::finish_webauthn_authentication(
                pool.as_ref(),
                &webauthn,
                user_id,
                &credential,
                &auth_state,
            ).await?
        },
        _ => {
            return Err(AppError::BadRequest("Invalid 2FA method".to_string()));
        }
    };
    
    if !valid {
        // Record failure and check if account should be locked
        crate::middleware::record_2fa_failure(redis.get_ref(), user_id).await?;
        return Err(AppError::Unauthorized("Invalid 2FA code".to_string()));
    }
    
    // Reset failure counter on successful verification
    crate::middleware::reset_2fa_failures(redis.get_ref(), user_id).await?;
    
    // Delete the temporary token from Redis
    deadpool_redis::redis::cmd("DEL")
        .arg(format!("2fa:{}", body.temporary_token))
        .query_async::<_, ()>(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete 2FA session: {}", e)))?;
    
    // Get user info
    let user = UserService::get_user_by_id(pool.as_ref(), user_id).await?;
    
    // Generate JWT
    let now = Utc::now();
    let exp = now + Duration::seconds(config.jwt_expiration);

    let claims = Claims {
        sub: user.username.clone(),
        user_id: user.id,
        role: user.role.clone(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        scopes: TokenScope::Full, // JWT has full access
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;
    
    Ok(HttpResponse::Ok().json(LoginResponse {
        token,
        user: UserInfo::from(user),
    }))
}

pub async fn me(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    let user = UserService::get_user_by_id(pool.get_ref(), claims.user_id).await?;
    Ok(HttpResponse::Ok().json(UserInfo::from(user)))
}

// ─── Email Confirmation ────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct ConfirmEmailRequest {
    pub token: String,
}

/// POST /api/v1/auth/confirm-email - Confirm email address
pub async fn confirm_email(
    pool: web::Data<PgPool>,
    _redis: web::Data<RedisPool>,
    body: web::Json<ConfirmEmailRequest>,
) -> AppResult<HttpResponse> {
    let user = UserService::confirm_email(pool.get_ref(), &body.token).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Email confirmed successfully",
        "user": UserInfo::from(user)
    })))
}

/// POST /api/v1/auth/resend-confirmation - Resend confirmation email
pub async fn resend_confirmation(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
) -> AppResult<HttpResponse> {
    // Requires authentication
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    let user = UserService::get_user_by_id(pool.get_ref(), claims.user_id).await?;
    
    if user.email_confirmed {
        return Err(AppError::BadRequest("Email is already confirmed".to_string()));
    }
    
    let token = UserService::resend_email_confirmation(pool.get_ref(), claims.user_id).await?;
    
    SmtpService::send_email_confirmation(
        &config.smtp,
        &user.email,
        &user.username,
        &token,
        &config.base_url,
    ).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Confirmation email sent"
    })))
}

// ─── Password Reset ────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct VerifyResetTokenRequest {
    pub token: String,
}

/// POST /api/v1/auth/forgot-password - Request password reset email
pub async fn forgot_password(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
    body: web::Json<ForgotPasswordRequest>,
) -> AppResult<HttpResponse> {
    // Always return success to prevent email enumeration
    match UserService::generate_password_reset_token(pool.get_ref(), &body.email).await {
        Ok((user, token)) => {
            if let Err(e) = SmtpService::send_password_reset(
                &config.smtp,
                &user.email,
                &user.username,
                &token,
                &config.base_url,
            ).await {
                log::error!("Failed to send password reset email: {}", e);
            }
        }
        Err(e) => {
            log::debug!("Password reset request for unknown email: {}", e);
        }
    }
    
    // Always return success to prevent email enumeration attacks
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "If an account exists with this email, a password reset link will be sent"
    })))
}

/// POST /api/v1/auth/verify-reset-token - Verify password reset token is valid
pub async fn verify_reset_token(
    pool: web::Data<PgPool>,
    _redis: web::Data<RedisPool>,
    body: web::Json<VerifyResetTokenRequest>,
) -> AppResult<HttpResponse> {
    let user = UserService::verify_password_reset_token(pool.get_ref(), &body.token).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "valid": true,
        "username": user.username
    })))
}

/// POST /api/v1/auth/reset-password - Reset password with token
pub async fn reset_password(
    pool: web::Data<PgPool>,
    body: web::Json<ResetPasswordRequest>,
) -> AppResult<HttpResponse> {
    if body.new_password.len() < 8 {
        return Err(AppError::BadRequest("Password must be at least 8 characters".to_string()));
    }
    
    let user = UserService::reset_password(pool.get_ref(), &body.token, &body.new_password).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Password has been reset successfully",
        "user": UserInfo::from(user)
    })))
}

// ─── WebAuthn Authentication (for Login) ───────────────

/// POST /api/v1/auth/passkey/login/start - Start Passkey direct login (no password required)
pub async fn passkey_login_start(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
) -> AppResult<HttpResponse> {
    // Create WebAuthn instance
    let webauthn = two_factor::create_webauthn(&config)?;
    
    // Get ALL passkeys from ALL users (for usernameless/discoverable credentials)
    // This allows users to log in without entering a username
    let passkeys = two_factor::get_all_passkeys(pool.as_ref()).await?;
    
    if passkeys.is_empty() {
        return Err(AppError::BadRequest("No Passkeys registered in the system".to_string()));
    }
    
    // Start authentication
    let (rcr, auth_state) = two_factor::start_webauthn_authentication(&webauthn, passkeys)?;
    
    // Store authentication state in Redis (5 minute expiration)
    let state_key = format!("passkey:login:{}", Utc::now().timestamp_millis());
    let state_json = serde_json::to_string(&auth_state)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize state: {}", e)))?;
    
    let mut redis_conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection failed: {}", e)))?;
    
    deadpool_redis::redis::cmd("SETEX")
        .arg(&state_key)
        .arg(300) // 5 minutes
        .arg(&state_json)
        .query_async::<_, ()>(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Redis error: {}", e)))?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "challenge": rcr,
        "state_key": state_key,
    })))
}

/// POST /api/v1/auth/passkey/login/finish - Finish Passkey direct login
pub async fn passkey_login_finish(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
    body: web::Json<serde_json::Value>, // Accept raw JSON to extract credential
) -> AppResult<HttpResponse> {
    let state_key = body.get("state_key")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing state_key".to_string()))?;
    
    // Get authentication state from Redis
    let mut redis_conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection failed: {}", e)))?;
    
    let state_json: Option<String> = deadpool_redis::redis::cmd("GET")
        .arg(state_key)
        .query_async(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Redis error: {}", e)))?;
    
    let state_json = state_json
        .ok_or_else(|| AppError::BadRequest("Authentication session expired or invalid".to_string()))?;
    
    let auth_state: PasskeyAuthentication = serde_json::from_str(&state_json)
        .map_err(|e| AppError::InternalError(format!("Failed to deserialize state: {}", e)))?;
    
    // Delete the state from Redis
    deadpool_redis::redis::cmd("DEL")
        .arg(state_key)
        .query_async::<_, ()>(&mut redis_conn)
        .await
        .ok();
    
    // Extract credential from body
    let credential_json = body.get("credential")
        .ok_or_else(|| AppError::BadRequest("Missing credential".to_string()))?;
    
    let credential: PublicKeyCredential = serde_json::from_value(credential_json.clone())
        .map_err(|e| AppError::BadRequest(format!("Invalid credential format: {}", e)))?;
    
    // Create WebAuthn instance
    let webauthn = two_factor::create_webauthn(&config)?;
    
    // Get user_id from authenticated credential
    let user_id = two_factor::finish_webauthn_authentication_and_get_user(
        pool.as_ref(),
        &webauthn,
        &credential,
        &auth_state,
    ).await?;
    
    // Get user info
    let user = UserService::get_user_by_id(pool.as_ref(), user_id).await?;
    
    // Generate JWT
    let now = Utc::now();
    let exp = now + Duration::seconds(config.jwt_expiration);

    let claims = Claims {
        sub: user.username.clone(),
        user_id: user.id,
        role: user.role.clone(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        scopes: TokenScope::Full, // JWT has full access
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;
    
    Ok(HttpResponse::Ok().json(LoginResponse {
        token,
        user: UserInfo::from(user),
    }))
}

/// POST /api/v1/auth/webauthn/start - Start WebAuthn authentication during login (2FA)
pub async fn webauthn_auth_start(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
    body: web::Json<WebAuthnAuthStartRequest>,
) -> AppResult<HttpResponse> {
    // Get user ID from temporary token
    let mut redis_conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection failed: {}", e)))?;
    
    let user_id: Option<i64> = deadpool_redis::redis::cmd("GET")
        .arg(format!("2fa:{}", body.temporary_token))
        .query_async(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get 2FA session: {}", e)))?;
    
    let user_id = user_id.ok_or_else(|| {
        AppError::Unauthorized("Invalid or expired 2FA token".to_string())
    })?;
    
    // Create WebAuthn instance
    let webauthn = two_factor::create_webauthn(&config)?;
    
    // Get user's passkeys
    let passkeys = two_factor::get_user_passkeys(pool.as_ref(), user_id).await?;
    
    if passkeys.is_empty() {
        return Err(AppError::BadRequest("No WebAuthn credentials registered".to_string()));
    }
    
    // Start authentication
    let (rcr, auth_state) = two_factor::start_webauthn_authentication(&webauthn, passkeys)?;
    
    // Store authentication state in Redis (5 minute expiration)
    let state_key = format!("webauthn:auth:{}:{}", user_id, chrono::Utc::now().timestamp());
    let state_json = serde_json::to_string(&auth_state)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize state: {}", e)))?;
    
    deadpool_redis::redis::cmd("SETEX")
        .arg(&state_key)
        .arg(300) // 5 minutes
        .arg(&state_json)
        .query_async::<_, ()>(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Redis error: {}", e)))?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "challenge": rcr,
        "state_key": state_key,
    })))
}

/// POST /api/v1/auth/webauthn/finish - Finish WebAuthn authentication during login
pub async fn webauthn_auth_finish(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    config: web::Data<AppConfig>,
    body: web::Json<WebAuthnAuthFinishRequest>,
) -> AppResult<HttpResponse> {
    // Get user ID from temporary token
    let mut redis_conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection failed: {}", e)))?;
    
    let user_id: Option<i64> = deadpool_redis::redis::cmd("GET")
        .arg(format!("2fa:{}", body.temporary_token))
        .query_async(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get 2FA session: {}", e)))?;
    
    let user_id = user_id.ok_or_else(|| {
        AppError::Unauthorized("Invalid or expired 2FA token".to_string())
    })?;
    
    // Get authentication state from Redis
    let state_json: Option<String> = deadpool_redis::redis::cmd("GET")
        .arg(&body.state_key)
        .query_async(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Redis error: {}", e)))?;
    
    let state_json = state_json
        .ok_or_else(|| AppError::BadRequest("Authentication session expired or invalid".to_string()))?;
    
    let auth_state: PasskeyAuthentication = serde_json::from_str(&state_json)
        .map_err(|e| AppError::InternalError(format!("Failed to deserialize state: {}", e)))?;
    
    // Delete the state from Redis
    deadpool_redis::redis::cmd("DEL")
        .arg(&body.state_key)
        .query_async::<_, ()>(&mut redis_conn)
        .await
        .ok();
    
    // Create WebAuthn instance
    let webauthn = two_factor::create_webauthn(&config)?;
    
    // Finish authentication
    let valid = two_factor::finish_webauthn_authentication(
        pool.as_ref(),
        &webauthn,
        user_id,
        &body.credential,
        &auth_state,
    ).await?;
    
    if !valid {
        return Err(AppError::Unauthorized("WebAuthn authentication failed".to_string()));
    }
    
    // Delete the 2FA temporary token
    deadpool_redis::redis::cmd("DEL")
        .arg(format!("2fa:{}", body.temporary_token))
        .query_async::<_, ()>(&mut redis_conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete 2FA session: {}", e)))?;
    
    // Get user info
    let user = UserService::get_user_by_id(pool.as_ref(), user_id).await?;
    
    // Generate JWT
    let now = Utc::now();
    let exp = now + Duration::seconds(config.jwt_expiration);

    let claims = Claims {
        sub: user.username.clone(),
        user_id: user.id,
        role: user.role.clone(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        scopes: TokenScope::Full, // JWT has full access
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;
    
    Ok(HttpResponse::Ok().json(LoginResponse {
        token,
        user: UserInfo::from(user),
    }))
}
