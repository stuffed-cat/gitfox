//! SSH Key model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SshKey {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub key_type: String,
    pub public_key: String,
    pub fingerprint: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSshKeyRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(length(min = 50))]
    pub key: String,
}

#[derive(Debug, Serialize)]
pub struct SshKeyResponse {
    pub id: i64,
    pub title: String,
    pub key_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    pub fingerprint: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<SshKey> for SshKeyResponse {
    fn from(key: SshKey) -> Self {
        SshKeyResponse {
            id: key.id,
            title: key.title,
            key_type: key.key_type,
            public_key: None, // Don't expose the full key by default
            fingerprint: key.fingerprint,
            last_used_at: key.last_used_at,
            expires_at: key.expires_at,
            created_at: key.created_at,
        }
    }
}

/// SSH Key info for internal API (GitFox Shell)
#[derive(Debug, Serialize)]
pub struct SshKeyInternalInfo {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub key_type: String,
    pub key: String,
}

/// Access check response for GitFox Shell
#[derive(Debug, Serialize)]
pub struct AccessCheckResponse {
    pub status: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_write: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lfs_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_status: Option<String>,
}

impl AccessCheckResponse {
    pub fn allowed(
        user_id: i64,
        username: String,
        can_write: bool,
        project_id: Option<i64>,
        base_url: Option<String>,
    ) -> Self {
        Self {
            status: true,
            message: None,
            user_id: Some(user_id),
            username: Some(username),
            can_write: Some(can_write),
            project_id,
            repository_id: project_id,
            lfs_token: None,
            base_url,
            repository_status: Some("active".to_string()),
        }
    }

    pub fn denied(message: &str) -> Self {
        Self {
            status: false,
            message: Some(message.to_string()),
            user_id: None,
            username: None,
            can_write: None,
            project_id: None,
            repository_id: None,
            lfs_token: None,
            base_url: None,
            repository_status: None,
        }
    }
}

/// Access check request from GitFox Shell
#[derive(Debug, Deserialize)]
pub struct AccessCheckRequest {
    pub key_id: String,
    pub repo_path: String,
    pub action: String,
    pub protocol: String,
}

/// Find key by fingerprint request
#[derive(Debug, Deserialize)]
pub struct FindKeyRequest {
    pub fingerprint: String,
}

/// Post-receive notification
#[derive(Debug, Deserialize)]
pub struct PostReceiveRequest {
    pub user_id: String,
    pub repository: String,
    pub project_id: Option<String>,
    pub changes: Vec<RefChange>,
}

#[derive(Debug, Deserialize)]
pub struct RefChange {
    pub old_sha: String,
    pub new_sha: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
}

/// Check ref update request
#[derive(Debug, Deserialize)]
pub struct CheckRefUpdateRequest {
    pub user_id: String,
    pub repository: String,
    pub project_id: Option<String>,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub old_sha: String,
    pub new_sha: String,
    pub change_type: String,
}

#[derive(Debug, Serialize)]
pub struct CheckRefUpdateResponse {
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// HTTP access check request for Workhorse
/// Workhorse 发送此请求来验证用户是否有权限访问仓库
#[derive(Debug, Deserialize)]
pub struct HttpAccessCheckRequest {
    /// Repository path (e.g., "namespace/project" or "user/project")
    pub repo_path: String,
    /// Git action: "git-upload-pack" (pull/clone) or "git-receive-pack" (push)
    pub action: String,
    /// Optional: user ID (if JWT token auth)
    pub user_id: Option<i64>,
    /// Optional: username for basic auth
    pub username: Option<String>,
    /// Optional: password/token for basic auth
    pub password: Option<String>,
}

/// HTTP access check response for Workhorse
#[derive(Debug, Serialize)]
pub struct HttpAccessCheckResponse {
    /// Whether access is allowed
    pub status: bool,
    /// Error message if denied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// User ID if authenticated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    /// Username if authenticated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Whether user has write permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_write: Option<bool>,
    /// Project ID if found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    /// Repository path on disk (e.g., "/repos/namespace/project.git")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_path: Option<String>,
    /// GitLayer address for RPC calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gitlayer_address: Option<String>,
}

impl HttpAccessCheckResponse {
    pub fn allowed(
        user_id: i64,
        username: String,
        can_write: bool,
        project_id: i64,
        repository_path: String,
        gitlayer_address: Option<String>,
    ) -> Self {
        Self {
            status: true,
            message: None,
            user_id: Some(user_id),
            username: Some(username),
            can_write: Some(can_write),
            project_id: Some(project_id),
            repository_path: Some(repository_path),
            gitlayer_address,
        }
    }

    pub fn denied(message: &str) -> Self {
        Self {
            status: false,
            message: Some(message.to_string()),
            user_id: None,
            username: None,
            can_write: None,
            project_id: None,
            repository_path: None,
            gitlayer_address: None,
        }
    }

    /// Anonymous read access (for public repos)
    pub fn anonymous_read(project_id: i64, repository_path: String, gitlayer_address: Option<String>) -> Self {
        Self {
            status: true,
            message: None,
            user_id: None,
            username: None,
            can_write: Some(false),
            project_id: Some(project_id),
            repository_path: Some(repository_path),
            gitlayer_address,
        }
    }
}
