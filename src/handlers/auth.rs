use actix_web::{web, HttpRequest, HttpResponse};
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::validate_token;
use crate::models::{CreateUserRequest, LoginRequest, UserInfo};
use crate::services::{SmtpService, SystemConfigService, UserService};

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
    let response = UserService::login(pool.get_ref(), config.get_ref(), body.into_inner()).await?;
    
    // Check if email confirmation is required and user hasn't confirmed
    let require_email_confirmation = SystemConfigService::get(pool.get_ref(), redis.get_ref(), "require_email_confirmation")
        .await
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    
    if require_email_confirmation && !response.user.email_confirmed {
        return Err(AppError::Unauthorized("Please confirm your email address before logging in".to_string()));
    }
    
    Ok(HttpResponse::Ok().json(response))
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
    ).await?;;
    
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
