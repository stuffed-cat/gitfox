//! API client for communicating with GitFox backend

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::error::ShellError;

/// Information about the user's access to a repository
#[derive(Debug, Clone, Deserialize)]
pub struct AccessInfo {
    /// User ID
    pub user_id: i64,

    /// Username
    pub username: String,

    /// Whether the user can write to the repository
    pub can_write: bool,

    /// Project ID (if it's a project repository)
    pub project_id: Option<i64>,

    /// Repository ID
    pub repository_id: Option<i64>,

    /// LFS token for Git LFS operations
    pub lfs_token: Option<String>,

    /// Base URL for LFS
    pub base_url: Option<String>,

    /// Repository status (active, archived, etc.)
    pub repository_status: Option<String>,
}

/// Request to check access
#[derive(Debug, Serialize)]
struct AccessCheckRequest {
    key_id: String,
    repo_path: String,
    action: String,
    protocol: String,
}

/// Response from access check
#[derive(Debug, Deserialize)]
struct AccessCheckResponse {
    status: bool,
    message: Option<String>,
    
    #[serde(flatten)]
    access_info: Option<AccessInfo>,
}

/// API response for key lookup
#[derive(Debug, Deserialize)]
pub struct SshKeyInfo {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub key_type: String,
    pub key: String,
}

/// Client for communicating with GitFox API
pub struct ApiClient {
    client: reqwest::Client,
    config: Config,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(config: &Config) -> Result<Self, ShellError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "X-GitFox-Shell-Token",
            HeaderValue::from_str(&config.api_secret).map_err(|e| {
                ShellError::Config(format!("Invalid API secret: {}", e))
            })?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.api_timeout_secs))
            .build()
            .map_err(|e| ShellError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    /// Check if a key has access to a repository
    pub async fn check_access(
        &self,
        key_id: &str,
        repo_path: &str,
        needs_write: bool,
    ) -> Result<AccessInfo, ShellError> {
        let url = self.config.api_endpoint("/allowed");
        let action = if needs_write { "git-receive-pack" } else { "git-upload-pack" };

        let request = AccessCheckRequest {
            key_id: key_id.to_string(),
            repo_path: repo_path.to_string(),
            action: action.to_string(),
            protocol: "ssh".to_string(),
        };

        debug!("Checking access at {} with request: {:?}", url, request);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to connect to API: {}", e);
                ShellError::Api(format!("Failed to connect to GitFox API: {}", e))
            })?;

        let status = response.status();
        debug!("API response status: {}", status);

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(ShellError::RepoNotFound(repo_path.to_string()));
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let body: AccessCheckResponse = response.json().await.map_err(|e| {
                ShellError::Api(format!("Failed to parse API response: {}", e))
            })?;
            return Err(ShellError::AccessDenied(
                body.message.unwrap_or_else(|| "Access denied".to_string()),
            ));
        }

        if !status.is_success() {
            error!("API returned error status: {}", status);
            return Err(ShellError::Api(format!(
                "API returned error status: {}",
                status
            )));
        }

        let body: AccessCheckResponse = response.json().await.map_err(|e| {
            error!("Failed to parse API response: {}", e);
            ShellError::Api(format!("Failed to parse API response: {}", e))
        })?;

        if !body.status {
            return Err(ShellError::AccessDenied(
                body.message.unwrap_or_else(|| "Access denied".to_string()),
            ));
        }

        body.access_info.ok_or_else(|| {
            ShellError::Api("Invalid API response: missing access info".to_string())
        })
    }

    /// Look up SSH key by key ID
    pub async fn get_key(&self, key_id: &str) -> Result<SshKeyInfo, ShellError> {
        let url = self.config.api_endpoint(&format!("/keys/{}", key_id));

        debug!("Looking up key at {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to connect to API: {}", e);
                ShellError::Api(format!("Failed to connect to GitFox API: {}", e))
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(ShellError::AccessDenied("SSH key not found".to_string()));
        }

        if !status.is_success() {
            return Err(ShellError::Api(format!(
                "API returned error status: {}",
                status
            )));
        }

        response.json().await.map_err(|e| {
            ShellError::Api(format!("Failed to parse API response: {}", e))
        })
    }

    /// Look up SSH key by fingerprint
    pub async fn find_key_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<SshKeyInfo, ShellError> {
        let url = self.config.api_endpoint("/keys/find");

        debug!("Looking up key by fingerprint at {}", url);

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "fingerprint": fingerprint }))
            .send()
            .await
            .map_err(|e| {
                error!("Failed to connect to API: {}", e);
                ShellError::Api(format!("Failed to connect to GitFox API: {}", e))
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(ShellError::AccessDenied("SSH key not found".to_string()));
        }

        if !status.is_success() {
            return Err(ShellError::Api(format!(
                "API returned error status: {}",
                status
            )));
        }

        response.json().await.map_err(|e| {
            ShellError::Api(format!("Failed to parse API response: {}", e))
        })
    }

    /// Notify post-receive hook
    pub async fn post_receive(
        &self,
        key_id: &str,
        repo_path: &str,
        changes: &[(String, String, String)], // (old_sha, new_sha, ref_name)
    ) -> Result<(), ShellError> {
        let url = self.config.api_endpoint("/post-receive");

        let body = serde_json::json!({
            "key_id": key_id,
            "repo_path": repo_path,
            "changes": changes.iter().map(|(old, new, ref_name)| {
                serde_json::json!({
                    "old_sha": old,
                    "new_sha": new,
                    "ref": ref_name,
                })
            }).collect::<Vec<_>>(),
        });

        debug!("Sending post-receive notification to {}", url);

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                warn!("Failed to send post-receive notification: {}", e);
                ShellError::Api(format!("Failed to send post-receive notification: {}", e))
            })?;

        if !response.status().is_success() {
            warn!(
                "Post-receive notification failed with status: {}",
                response.status()
            );
        }

        Ok(())
    }
}
