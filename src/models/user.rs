use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Email confirmation fields
    #[serde(default)]
    pub email_confirmed: bool,
    #[serde(skip_serializing)]
    pub email_confirmation_token: Option<String>,
    pub email_confirmation_sent_at: Option<DateTime<Utc>>,
    // Password reset fields
    #[serde(skip_serializing)]
    pub password_reset_token: Option<String>,
    pub password_reset_sent_at: Option<DateTime<Utc>>,
    // Two-factor authentication
    #[serde(default)]
    pub two_factor_enabled: bool,
    pub two_factor_required_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Developer,
    Viewer,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub email_confirmed: bool,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        UserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            role: user.role,
            is_active: user.is_active,
            email_confirmed: user.email_confirmed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user_id: i64,
    pub role: UserRole,
    pub exp: i64,
    pub iat: i64,
}

// Two-factor authentication response during login
#[derive(Debug, Serialize)]
pub struct TwoFactorRequiredResponse {
    pub requires_two_factor: bool,
    pub available_methods: Vec<String>, // ["totp", "webauthn", "recovery"]
    pub temporary_token: String, // Used for completing 2FA verification
}

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyTwoFactorRequest {
    pub temporary_token: String,
    pub method: String, // "totp", "webauthn", or "recovery"
    pub code: Option<String>, // For TOTP and recovery codes
    pub webauthn_response: Option<String>, // For WebAuthn
}
