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
            base_url: None,
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
