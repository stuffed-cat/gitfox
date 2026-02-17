use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Provider (Dynamic - stored in database)
// External identity providers that GitFox can authenticate against
// ─────────────────────────────────────────────────────────────────────────────

/// OAuth Provider configuration stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthProviderRecord {
    pub id: i64,
    /// Unique provider slug (e.g., "github", "google", "my-company-sso")
    pub name: String,
    /// Display name shown to users
    pub display_name: String,
    /// Provider type: "github", "gitlab", "google", "oidc", "oauth2"
    pub provider_type: String,
    /// OAuth authorization endpoint
    pub authorization_url: String,
    /// OAuth token endpoint
    pub token_url: String,
    /// User info endpoint
    pub userinfo_url: Option<String>,
    /// OIDC issuer URL (for auto-discovery)
    pub issuer_url: Option<String>,
    /// JWKS URI for token verification
    pub jwks_uri: Option<String>,
    /// OAuth client ID
    pub client_id: String,
    /// Encrypted client secret
    #[serde(skip_serializing)]
    pub client_secret_encrypted: String,
    /// Scopes to request (JSON array)
    pub scopes: serde_json::Value,
    /// Field mappings for user info extraction
    pub field_mappings: serde_json::Value,
    /// Whether this provider is enabled
    pub enabled: bool,
    /// Allow user signup via this provider
    pub allow_signup: bool,
    /// Auto-link existing users by email
    pub auto_link_by_email: bool,
    /// Icon name or URL
    pub icon: Option<String>,
    /// Display order
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public info about an OAuth provider (for login page)
#[derive(Debug, Serialize)]
pub struct OAuthProviderInfo {
    pub name: String,
    pub display_name: String,
    pub provider_type: String,
    pub icon: Option<String>,
    pub authorize_url: String,
}

impl OAuthProviderRecord {
    /// Build authorization URL with parameters
    pub fn build_authorize_url(&self, client_id: &str, redirect_uri: &str, state: &str) -> String {
        let scopes: Vec<String> = serde_json::from_value(self.scopes.clone()).unwrap_or_default();
        let scope = scopes.join(" ");
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            self.authorization_url,
            urlencoding::encode(client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scope),
            urlencoding::encode(state)
        )
    }

    pub fn to_info(&self, redirect_uri: &str, state: &str) -> OAuthProviderInfo {
        OAuthProviderInfo {
            name: self.name.clone(),
            display_name: self.display_name.clone(),
            provider_type: self.provider_type.clone(),
            icon: self.icon.clone(),
            authorize_url: self.build_authorize_url(&self.client_id, redirect_uri, state),
        }
    }
}

/// Request to create/update an OAuth provider (admin only)
#[derive(Debug, Deserialize, Validate)]
pub struct CreateOAuthProviderRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 1, max = 255))]
    pub display_name: String,
    pub provider_type: String,
    #[validate(url)]
    pub authorization_url: String,
    #[validate(url)]
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub issuer_url: Option<String>,
    pub jwks_uri: Option<String>,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Option<Vec<String>>,
    pub field_mappings: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub allow_signup: Option<bool>,
    pub auto_link_by_email: Option<bool>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOAuthProviderRequest {
    pub display_name: Option<String>,
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub userinfo_url: Option<String>,
    pub issuer_url: Option<String>,
    pub jwks_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub field_mappings: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub allow_signup: Option<bool>,
    pub auto_link_by_email: Option<bool>,
    pub icon: Option<String>,
    pub sort_order: Option<i32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Application (GitFox as OAuth Provider)
// Third-party apps can authenticate users via GitFox
// ─────────────────────────────────────────────────────────────────────────────

/// OAuth Application registered in GitFox
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthApplication {
    pub id: i64,
    pub owner_id: i64,
    pub name: String,
    /// Client ID (public identifier)
    pub uid: String,
    #[serde(skip_serializing)]
    pub secret_hash: String,
    /// Redirect URIs (JSON array)
    pub redirect_uris: serde_json::Value,
    /// Allowed scopes (JSON array)
    pub scopes: serde_json::Value,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub logo_url: Option<String>,
    pub confidential: bool,
    pub trusted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public info about an OAuth application
#[derive(Debug, Serialize)]
pub struct OAuthApplicationInfo {
    pub id: i64,
    pub name: String,
    pub uid: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub logo_url: Option<String>,
    pub confidential: bool,
    pub trusted: bool,
    pub created_at: DateTime<Utc>,
}

impl From<OAuthApplication> for OAuthApplicationInfo {
    fn from(app: OAuthApplication) -> Self {
        Self {
            id: app.id,
            name: app.name,
            uid: app.uid,
            redirect_uris: serde_json::from_value(app.redirect_uris).unwrap_or_default(),
            scopes: serde_json::from_value(app.scopes).unwrap_or_default(),
            description: app.description,
            homepage_url: app.homepage_url,
            logo_url: app.logo_url,
            confidential: app.confidential,
            trusted: app.trusted,
            created_at: app.created_at,
        }
    }
}

/// Response when creating/regenerating client secret
#[derive(Debug, Serialize)]
pub struct OAuthApplicationWithSecret {
    pub id: i64,
    pub name: String,
    pub uid: String,
    /// Only returned on creation or regeneration!
    pub secret: String,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to create an OAuth application
#[derive(Debug, Deserialize, Validate)]
pub struct CreateOAuthApplicationRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1))]
    pub redirect_uris: Vec<String>,
    pub scopes: Option<Vec<String>>,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub confidential: Option<bool>,
}

/// Request to update an OAuth application
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOAuthApplicationRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub scopes: Option<Vec<String>>,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Authorization Code
// Short-lived codes exchanged for access tokens
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct OAuthAuthorizationCode {
    pub id: i64,
    pub application_id: i64,
    pub user_id: i64,
    pub code_hash: String,
    pub redirect_uri: String,
    pub scopes: serde_json::Value,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Access Token (issued by GitFox as provider)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct OAuthAccessToken {
    pub id: i64,
    pub application_id: i64,
    pub user_id: i64,
    pub token_hash: String,
    pub refresh_token_hash: Option<String>,
    pub scopes: serde_json::Value,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Token response for OAuth token endpoint
#[derive(Debug, Serialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Identity (GitFox as OAuth Client - linked external accounts)
// Links users to their external OAuth provider accounts
// ─────────────────────────────────────────────────────────────────────────────

/// External OAuth identity linked to a GitFox user
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthIdentity {
    pub id: i64,
    pub user_id: i64,
    /// Reference to oauth_providers.id
    pub provider_id: i64,
    /// External user ID from provider
    pub external_uid: String,
    pub external_username: Option<String>,
    pub external_email: Option<String>,
    pub external_avatar_url: Option<String>,
    #[serde(skip_serializing)]
    pub access_token_encrypted: Option<String>,
    #[serde(skip_serializing)]
    pub refresh_token_encrypted: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub raw_info: Option<serde_json::Value>,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Public info about a linked OAuth identity
#[derive(Debug, Serialize, FromRow)]
pub struct OAuthIdentityInfo {
    pub id: i64,
    pub provider_id: i64,
    pub provider_name: String,
    pub provider_display_name: String,
    pub external_username: Option<String>,
    pub external_email: Option<String>,
    pub external_avatar_url: Option<String>,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Authorization Request/Response
// ─────────────────────────────────────────────────────────────────────────────

/// OAuth authorization request params
#[derive(Debug, Deserialize)]
pub struct OAuthAuthorizeRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    /// PKCE code challenge
    pub code_challenge: Option<String>,
    /// PKCE code challenge method (plain or S256)
    pub code_challenge_method: Option<String>,
}

/// OAuth token exchange request
#[derive(Debug, Deserialize)]
pub struct OAuthTokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub refresh_token: Option<String>,
    /// PKCE code verifier
    pub code_verifier: Option<String>,
}

/// Info returned during OAuth login flow
#[derive(Debug, Serialize)]
pub struct OAuthLoginResponse {
    pub token: String,
    pub user: crate::models::UserInfo,
    /// Whether this is a new user (just registered)
    pub is_new_user: bool,
}

/// Request to link an OAuth provider to existing account
#[derive(Debug, Deserialize)]
pub struct LinkOAuthRequest {
    pub provider: String,
    pub code: String,
    pub state: Option<String>,
    pub redirect_uri: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Available OAuth Scopes (for GitFox as provider)
// ─────────────────────────────────────────────────────────────────────────────

/// OAuth scopes that applications can request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OAuthScope {
    /// Read user profile (username, email, avatar)
    ReadUser,
    /// Read API (general read access)
    ReadApi,
    /// Write API (general write access)
    WriteApi,
    /// Read repositories the user has access to
    ReadRepository,
    /// Write to repositories
    WriteRepository,
    /// Access user's email
    Email,
    /// OpenID Connect profile
    OpenId,
    /// Full access
    Api,
}

impl OAuthScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            OAuthScope::ReadUser => "read_user",
            OAuthScope::ReadApi => "read_api",
            OAuthScope::WriteApi => "write_api",
            OAuthScope::ReadRepository => "read_repository",
            OAuthScope::WriteRepository => "write_repository",
            OAuthScope::Email => "email",
            OAuthScope::OpenId => "openid",
            OAuthScope::Api => "api",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read_user" => Some(OAuthScope::ReadUser),
            "read_api" => Some(OAuthScope::ReadApi),
            "write_api" => Some(OAuthScope::WriteApi),
            "read_repository" => Some(OAuthScope::ReadRepository),
            "write_repository" => Some(OAuthScope::WriteRepository),
            "email" => Some(OAuthScope::Email),
            "openid" => Some(OAuthScope::OpenId),
            "api" => Some(OAuthScope::Api),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            OAuthScope::ReadUser => "Read your user profile",
            OAuthScope::ReadApi => "Read access to the API",
            OAuthScope::WriteApi => "Write access to the API",
            OAuthScope::ReadRepository => "Read your repositories",
            OAuthScope::WriteRepository => "Write to your repositories",
            OAuthScope::Email => "Access your email address",
            OAuthScope::OpenId => "Authenticate using OpenID Connect",
            OAuthScope::Api => "Full API access",
        }
    }

    /// All available scopes
    pub fn all() -> Vec<Self> {
        vec![
            Self::ReadUser,
            Self::ReadApi,
            Self::WriteApi,
            Self::ReadRepository,
            Self::WriteRepository,
            Self::Email,
            Self::OpenId,
            Self::Api,
        ]
    }
}

/// Response for GET /oauth/providers - list available OAuth providers
#[derive(Debug, Serialize)]
pub struct OAuthProvidersResponse {
    pub providers: Vec<OAuthProviderInfo>,
}
