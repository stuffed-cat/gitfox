//! Project settings models
//!
//! Models for project-level settings:
//! - Branch protection rules
//! - CI/CD variables
//! - Pipeline triggers
//! - Deploy keys
//! - Project access tokens

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

// =========================================
// Branch Protection Rules
// =========================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BranchProtectionRule {
    pub id: i64,
    pub project_id: i64,
    pub branch_pattern: String,
    pub require_review: bool,
    pub required_reviewers: i32,
    pub require_ci_pass: bool,
    pub allow_force_push: bool,
    pub allow_deletion: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateBranchProtectionRequest {
    #[validate(length(min = 1, max = 255))]
    pub branch_pattern: String,
    #[serde(default)]
    pub require_review: bool,
    #[serde(default = "default_required_reviewers")]
    pub required_reviewers: i32,
    #[serde(default)]
    pub require_ci_pass: bool,
    #[serde(default)]
    pub allow_force_push: bool,
    #[serde(default)]
    pub allow_deletion: bool,
}

fn default_required_reviewers() -> i32 {
    1
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateBranchProtectionRequest {
    pub require_review: Option<bool>,
    pub required_reviewers: Option<i32>,
    pub require_ci_pass: Option<bool>,
    pub allow_force_push: Option<bool>,
    pub allow_deletion: Option<bool>,
}

// =========================================
// CI/CD Variables
// =========================================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CiVariable {
    pub id: i64,
    pub project_id: i64,
    pub key: String,
    #[serde(skip_serializing)]
    pub value_encrypted: String,
    pub protected: bool,
    pub masked: bool,
    pub file: bool,
    pub environment_scope: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response format for CI variables (never expose actual value)
#[derive(Debug, Clone, Serialize)]
pub struct CiVariableResponse {
    pub id: i64,
    pub key: String,
    pub protected: bool,
    pub masked: bool,
    pub file: bool,
    pub environment_scope: String,
    pub created_at: DateTime<Utc>,
}

impl From<CiVariable> for CiVariableResponse {
    fn from(v: CiVariable) -> Self {
        Self {
            id: v.id,
            key: v.key,
            protected: v.protected,
            masked: v.masked,
            file: v.file,
            environment_scope: v.environment_scope,
            created_at: v.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCiVariableRequest {
    #[validate(length(min = 1, max = 255))]
    pub key: String,
    #[validate(length(min = 1))]
    pub value: String,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub masked: bool,
    #[serde(default)]
    pub file: bool,
    #[serde(default = "default_environment_scope")]
    pub environment_scope: String,
}

fn default_environment_scope() -> String {
    "*".to_string()
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateCiVariableRequest {
    pub value: Option<String>,
    pub protected: Option<bool>,
    pub masked: Option<bool>,
    pub file: Option<bool>,
    pub environment_scope: Option<String>,
}

// =========================================
// Pipeline Triggers
// =========================================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PipelineTrigger {
    pub id: i64,
    pub project_id: i64,
    pub description: Option<String>,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub token_preview: String,
    pub created_by: i64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Response when creating a trigger (includes full token only once)
#[derive(Debug, Clone, Serialize)]
pub struct CreatePipelineTriggerResponse {
    pub id: i64,
    pub description: Option<String>,
    pub token: String,  // Full token, shown only at creation time
    pub token_preview: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreatePipelineTriggerRequest {
    #[validate(length(max = 255))]
    pub description: Option<String>,
}

// =========================================
// Deploy Keys
// =========================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeployKey {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub key_type: String,
    pub public_key: String,
    pub fingerprint: String,
    pub can_push: bool,
    pub created_by: i64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateDeployKeyRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(length(min = 1))]
    pub key: String,  // Full public key content
    #[serde(default)]
    pub can_push: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateDeployKeyRequest {
    pub title: Option<String>,
    pub can_push: Option<bool>,
}

// =========================================
// Project Access Tokens
// =========================================

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ProjectAccessToken {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub token_preview: String,
    pub scopes: Vec<String>,
    pub role: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
}

/// Response when creating a project access token (includes full token only once)
#[derive(Debug, Clone, Serialize)]
pub struct CreateProjectAccessTokenResponse {
    pub id: i64,
    pub name: String,
    pub token: String,  // Full token, shown only at creation time
    pub scopes: Vec<String>,
    pub role: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateProjectAccessTokenRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub scopes: Vec<String>,
    #[serde(default = "default_token_role")]
    pub role: String,  // maintainer, developer, reporter
    pub expires_at: Option<DateTime<Utc>>,
}

fn default_token_role() -> String {
    "developer".to_string()
}

// =========================================
// Validators
// =========================================

pub mod validators {
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        /// CI variable key must be uppercase letters, numbers, and underscores
        pub static ref CI_VAR_KEY_REGEX: Regex = Regex::new(r"^[A-Z][A-Z0-9_]*$").unwrap();
    }
}
