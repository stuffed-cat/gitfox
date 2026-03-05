//! GPG Key model and related types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// GPG Key stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GpgKey {
    pub id: i64,
    pub user_id: i64,
    pub primary_key_id: String,
    pub fingerprint: String,
    pub public_key: String,
    pub key_algorithm: String,
    pub key_size: Option<i32>,
    pub emails: Vec<String>,
    pub can_sign: bool,
    pub can_encrypt: bool,
    pub can_certify: bool,
    pub key_created_at: Option<DateTime<Utc>>,
    pub key_expires_at: Option<DateTime<Utc>>,
    pub verified: bool,
    pub revoked: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// GPG Key subkey stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GpgKeySubkey {
    pub id: i64,
    pub gpg_key_id: i64,
    pub key_id: String,
    pub fingerprint: String,
    pub key_algorithm: String,
    pub key_size: Option<i32>,
    pub can_sign: bool,
    pub can_encrypt: bool,
    pub key_created_at: Option<DateTime<Utc>>,
    pub key_expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

/// GPG signature verification cache entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GpgSignature {
    pub id: i64,
    pub project_id: i64,
    pub commit_sha: String,
    pub gpg_key_id: Option<i64>,
    pub gpg_key_subkey_id: Option<i64>,
    pub signer_key_id: String,
    pub verification_status: String,
    pub verification_message: Option<String>,
    pub signer_email: Option<String>,
    pub signer_name: Option<String>,
    pub signer_user_id: Option<i64>,
    pub raw_signature: Option<String>,
    pub signed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Verification status for GPG signatures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpgVerificationStatus {
    /// Signature is valid and email matches verified user
    Verified,
    /// Signature is valid but email doesn't match
    Unverified,
    /// Email in commit doesn't match any email in the key
    BadEmail,
    /// Key not found in our system
    UnknownKey,
    /// Cryptographically invalid signature
    BadSignature,
    /// Key has expired
    ExpiredKey,
    /// Key has been revoked
    RevokedKey,
}

impl GpgVerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unverified => "unverified",
            Self::BadEmail => "bad_email",
            Self::UnknownKey => "unknown_key",
            Self::BadSignature => "bad_signature",
            Self::ExpiredKey => "expired_key",
            Self::RevokedKey => "revoked_key",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "verified" => Some(Self::Verified),
            "unverified" => Some(Self::Unverified),
            "bad_email" => Some(Self::BadEmail),
            "unknown_key" => Some(Self::UnknownKey),
            "bad_signature" => Some(Self::BadSignature),
            "expired_key" => Some(Self::ExpiredKey),
            "revoked_key" => Some(Self::RevokedKey),
            _ => None,
        }
    }
}

impl std::fmt::Display for GpgVerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Request to add a new GPG key
#[derive(Debug, Deserialize, Validate)]
pub struct CreateGpgKeyRequest {
    /// ASCII-armored public key
    #[validate(length(min = 100, message = "GPG key appears to be too short"))]
    pub key: String,
}

/// Response for a GPG key (API)
#[derive(Debug, Serialize)]
pub struct GpgKeyResponse {
    pub id: i64,
    pub primary_key_id: String,
    pub fingerprint: String,
    pub key_algorithm: String,
    pub key_size: Option<i32>,
    pub emails: Vec<String>,
    pub can_sign: bool,
    pub can_encrypt: bool,
    pub can_certify: bool,
    pub key_created_at: Option<DateTime<Utc>>,
    pub key_expires_at: Option<DateTime<Utc>>,
    pub verified: bool,
    pub revoked: bool,
    pub subkeys: Vec<GpgKeySubkeyResponse>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Response for a GPG key subkey (API)
#[derive(Debug, Serialize)]
pub struct GpgKeySubkeyResponse {
    pub id: i64,
    pub key_id: String,
    pub fingerprint: String,
    pub key_algorithm: String,
    pub key_size: Option<i32>,
    pub can_sign: bool,
    pub can_encrypt: bool,
    pub key_created_at: Option<DateTime<Utc>>,
    pub key_expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

impl From<GpgKeySubkey> for GpgKeySubkeyResponse {
    fn from(subkey: GpgKeySubkey) -> Self {
        Self {
            id: subkey.id,
            key_id: subkey.key_id,
            fingerprint: subkey.fingerprint,
            key_algorithm: subkey.key_algorithm,
            key_size: subkey.key_size,
            can_sign: subkey.can_sign,
            can_encrypt: subkey.can_encrypt,
            key_created_at: subkey.key_created_at,
            key_expires_at: subkey.key_expires_at,
            revoked: subkey.revoked,
        }
    }
}

impl GpgKey {
    pub fn to_response(self, subkeys: Vec<GpgKeySubkey>) -> GpgKeyResponse {
        GpgKeyResponse {
            id: self.id,
            primary_key_id: self.primary_key_id,
            fingerprint: self.fingerprint,
            key_algorithm: self.key_algorithm,
            key_size: self.key_size,
            emails: self.emails,
            can_sign: self.can_sign,
            can_encrypt: self.can_encrypt,
            can_certify: self.can_certify,
            key_created_at: self.key_created_at,
            key_expires_at: self.key_expires_at,
            verified: self.verified,
            revoked: self.revoked,
            subkeys: subkeys.into_iter().map(Into::into).collect(),
            last_used_at: self.last_used_at,
            created_at: self.created_at,
        }
    }
}

/// Signature verification result for API responses
#[derive(Debug, Serialize)]
pub struct SignatureVerificationResponse {
    pub status: String,
    pub message: Option<String>,
    pub key_id: Option<String>,
    pub signer_name: Option<String>,
    pub signer_email: Option<String>,
    pub signer_user_id: Option<i64>,
    pub signer_username: Option<String>,
    pub signed_at: Option<DateTime<Utc>>,
}

/// Parsed GPG key information (from gpg output)
#[derive(Debug, Clone)]
pub struct ParsedGpgKey {
    pub fingerprint: String,
    pub primary_key_id: String,
    pub algorithm: String,
    pub key_size: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub emails: Vec<String>,
    pub can_sign: bool,
    pub can_encrypt: bool,
    pub can_certify: bool,
    pub subkeys: Vec<ParsedGpgSubkey>,
}

/// Parsed GPG subkey information
#[derive(Debug, Clone)]
pub struct ParsedGpgSubkey {
    pub key_id: String,
    pub fingerprint: String,
    pub algorithm: String,
    pub key_size: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub can_sign: bool,
    pub can_encrypt: bool,
}

/// Used for internal API to verify signatures
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifySignatureRequest {
    pub commit_sha: String,
    pub signature: String,
    pub signed_data: String,
    pub committer_email: String,
}

/// Response for signature verification (internal)
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifySignatureResponse {
    pub valid: bool,
    pub status: String,
    pub message: Option<String>,
    pub key_id: Option<String>,
    pub signer_user_id: Option<i64>,
    pub signer_username: Option<String>,
}

/// GPG key info for lookup (internal)
#[derive(Debug, Serialize, Deserialize)]
pub struct GpgKeyLookup {
    pub key_id: String,
    pub fingerprint: String,
    pub user_id: i64,
    pub username: String,
    pub emails: Vec<String>,
    pub verified: bool,
    pub revoked: bool,
    pub expired: bool,
}
