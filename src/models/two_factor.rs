use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;
use webauthn_rs::prelude::*;

// TOTP Configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserTotp {
    pub id: i64,
    pub user_id: i64,
    pub secret: String, // Base32 encoded
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
}

// TOTP Setup State (stored in Redis during setup)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpSetupState {
    pub secret: String,
    pub recovery_codes: Vec<String>, // Plain text in Redis, hashed when written to DB
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpSetupResponse {
    pub state_key: String, // Redis key for retrieving setup state
    pub secret: String,
    pub qr_code: String, // Data URL of QR code image
    pub backup_codes: Vec<String>,
}

// WebAuthn Credential
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserWebAuthnCredential {
    pub id: i64,
    pub user_id: i64,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub counter: i64,
    pub name: String,
    pub aaguid: Option<Vec<u8>>,
    pub transports: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

// Recovery Code
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRecoveryCode {
    pub id: i64,
    pub user_id: i64,
    pub code_hash: String,
    pub used: bool,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// Two Factor Attempt (for rate limiting)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TwoFactorAttempt {
    pub id: i64,
    pub user_id: i64,
    pub ip_address: std::net::IpAddr,
    pub success: bool,
    pub method: String,
    pub created_at: DateTime<Utc>,
}

// Request/Response structures
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct EnableTotpRequest {
    pub state_key: String, // Redis key from setup_totp response
    #[validate(length(min = 6, max = 6))]
    pub totp_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DisableTotpRequest {
    #[validate(length(min = 6, max = 6))]
    pub totp_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct VerifyTotpRequest {
    #[validate(length(min = 6, max = 6))]
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnRegistrationStart {
    pub challenge: String,
    pub user_id: String,
    pub user_name: String,
    pub rp_name: String,
    pub rp_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WebAuthnRegistrationFinish {
    pub name: String,
    pub credential_id: String,
    pub client_data_json: String,
    pub attestation_object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnAuthenticationStart {
    pub challenge: String,
    pub credentials: Vec<WebAuthnCredentialDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredentialDescriptor {
    pub id: String,
    pub transports: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WebAuthnAuthenticationFinish {
    pub credential_id: String,
    pub client_data_json: String,
    pub authenticator_data: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UseRecoveryCodeRequest {
    #[validate(length(min = 8, max = 12))]
    pub recovery_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryCodesResponse {
    pub codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorStatus {
    pub enabled: bool,
    pub totp_enabled: bool,
    pub webauthn_credentials: Vec<WebAuthnCredentialInfo>,
    pub recovery_codes_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredentialInfo {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DeleteWebAuthnCredentialRequest {
    pub credential_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RenameWebAuthnCredentialRequest {
    pub credential_id: i64,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnRegisterStartRequest {
    // No fields needed - start just generates a challenge
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WebAuthnRegisterFinishRequest {
    pub state_key: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub credential: RegisterPublicKeyCredential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnAuthStartRequest {
    pub temporary_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnAuthFinishRequest {
    pub temporary_token: String,
    pub state_key: String,
    pub credential: PublicKeyCredential,
}
