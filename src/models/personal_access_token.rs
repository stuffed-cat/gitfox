use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// Available scopes for Personal Access Tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PatScope {
    /// Read API access
    ReadApi,
    /// Write API access (includes read)
    WriteApi,
    /// Read repository (clone, fetch)
    ReadRepository,
    /// Write repository (push)
    WriteRepository,
    /// Read user profile
    ReadUser,
    /// Write user profile
    WriteUser,
    /// Read registry (container, packages)
    ReadRegistry,
    /// Write registry
    WriteRegistry,
    /// Admin access (all permissions)
    Admin,
}

impl PatScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            PatScope::ReadApi => "read_api",
            PatScope::WriteApi => "write_api",
            PatScope::ReadRepository => "read_repository",
            PatScope::WriteRepository => "write_repository",
            PatScope::ReadUser => "read_user",
            PatScope::WriteUser => "write_user",
            PatScope::ReadRegistry => "read_registry",
            PatScope::WriteRegistry => "write_registry",
            PatScope::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read_api" => Some(PatScope::ReadApi),
            "write_api" => Some(PatScope::WriteApi),
            "read_repository" => Some(PatScope::ReadRepository),
            "write_repository" => Some(PatScope::WriteRepository),
            "read_user" => Some(PatScope::ReadUser),
            "write_user" => Some(PatScope::WriteUser),
            "read_registry" => Some(PatScope::ReadRegistry),
            "write_registry" => Some(PatScope::WriteRegistry),
            "admin" => Some(PatScope::Admin),
            _ => None,
        }
    }

    /// All available scopes
    pub fn all() -> Vec<PatScope> {
        vec![
            PatScope::ReadApi,
            PatScope::WriteApi,
            PatScope::ReadRepository,
            PatScope::WriteRepository,
            PatScope::ReadUser,
            PatScope::WriteUser,
            PatScope::ReadRegistry,
            PatScope::WriteRegistry,
            PatScope::Admin,
        ]
    }

    /// Default scopes for new tokens
    pub fn defaults() -> Vec<PatScope> {
        vec![PatScope::ReadApi, PatScope::ReadRepository, PatScope::ReadUser]
    }
}

/// Personal Access Token stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PersonalAccessToken {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    /// Token prefix for identification (first 8 chars, e.g., "gfpat_ab")
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Response when listing tokens (no sensitive data)
#[derive(Debug, Serialize)]
pub struct PersonalAccessTokenInfo {
    pub id: i64,
    pub name: String,
    /// Token prefix for identification (e.g., "gfpat_ab")
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

impl From<PersonalAccessToken> for PersonalAccessTokenInfo {
    fn from(pat: PersonalAccessToken) -> Self {
        Self {
            id: pat.id,
            name: pat.name,
            token_prefix: pat.token_prefix,
            scopes: pat.scopes,
            expires_at: pat.expires_at,
            last_used_at: pat.last_used_at,
            revoked: pat.revoked_at.is_some(),
            created_at: pat.created_at,
        }
    }
}

/// Response when creating a token (includes the raw token ONCE)
#[derive(Debug, Serialize)]
pub struct CreatePersonalAccessTokenResponse {
    pub id: i64,
    pub name: String,
    /// The raw token value - only returned on creation!
    pub token: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a new PAT
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePatRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be 1-255 characters"))]
    pub name: String,
    /// Scopes for the token (empty = default scopes)
    pub scopes: Option<Vec<String>>,
    /// Expiration in days (null = use default, 0 = never expires)
    pub expires_in_days: Option<u32>,
}

/// Token prefix for PAT (used to identify token type)
pub const PAT_PREFIX: &str = "gfpat_";
