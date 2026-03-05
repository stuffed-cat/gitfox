//! Auth gRPC Client for Workhorse
//!
//! 通过 gRPC 调用主应用的 Auth 服务进行 HTTP Git 权限认证。
//! 符合 GitLab 架构：Workhorse 通过 RPC 与主应用通信进行权限验证，
//! 然后直接通过 GitLayer 处理 Git 操作。

use tonic::transport::Channel;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{debug, info};

// 导入生成的 proto 代码
pub mod auth_proto {
    tonic::include_proto!("gitfox.auth");
}

use auth_proto::auth_service_client::AuthServiceClient;
use auth_proto::*;

/// Auth gRPC 客户端
#[derive(Clone)]
pub struct AuthClient {
    client: AuthServiceClient<Channel>,
    shell_token: String,
}

/// HTTP 访问检查结果
#[derive(Debug, Clone)]
pub struct HttpAccessResult {
    pub allowed: bool,
    pub message: String,
    pub user_id: i64,
    pub username: String,
    pub can_write: bool,
    pub project_id: i64,
    pub repository_path: String,
    pub gitlayer_address: String,
}

/// GPG签名验证结果
#[derive(Debug, Clone)]
pub struct GpgVerifyResult {
    pub valid: bool,
    pub status: String,
    pub message: String,
    pub key_id: String,
    pub signer_user_id: i64,
    pub signer_username: String,
}

/// GPG密钥查找结果
#[derive(Debug, Clone)]
pub struct GpgKeyInfo {
    pub found: bool,
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub primary_key_id: String,
    pub fingerprint: String,
    pub emails: Vec<String>,
    pub verified: bool,
    pub revoked: bool,
    pub expired: bool,
}

impl AuthClient {
    /// 创建新的 Auth 客户端
    pub async fn connect(address: &str, shell_token: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        debug!("Connecting to Auth gRPC server at {}", address);
        let client = AuthServiceClient::connect(address.to_string()).await?;
        Ok(Self { client, shell_token })
    }

    /// 添加认证 header
    fn add_auth<T>(&self, request: &mut Request<T>) {
        if let Ok(token) = self.shell_token.parse::<MetadataValue<_>>() {
            request.metadata_mut().insert("x-gitfox-shell-token", token);
        }
    }

    /// 检查 HTTP 访问权限 (JWT 认证)
    pub async fn check_http_access_jwt(
        &mut self,
        repo_path: &str,
        action: &str,
        token: &str,
    ) -> Result<HttpAccessResult, Box<dyn std::error::Error + Send + Sync>> {
        debug!("CheckHTTPAccess (JWT): repo={}, action={}", repo_path, action);

        let mut request = Request::new(HttpAccessRequest {
            repo_path: repo_path.to_string(),
            action: action.to_string(),
            auth: Some(http_access_request::Auth::JwtAuth(JwtAuth {
                token: token.to_string(),
            })),
        });
        self.add_auth(&mut request);

        let response = self.client.check_http_access(request).await?;
        let resp = response.into_inner();

        Ok(HttpAccessResult {
            allowed: resp.status,
            message: resp.message,
            user_id: resp.user_id,
            username: resp.username,
            can_write: resp.can_write,
            project_id: resp.project_id,
            repository_path: resp.repository_path,
            gitlayer_address: resp.gitlayer_address,
        })
    }

    /// 检查 HTTP 访问权限 (Basic 认证)
    pub async fn check_http_access_basic(
        &mut self,
        repo_path: &str,
        action: &str,
        username: &str,
        password: &str,
    ) -> Result<HttpAccessResult, Box<dyn std::error::Error + Send + Sync>> {
        debug!("CheckHTTPAccess (Basic): repo={}, action={}, user={}", repo_path, action, username);

        let mut request = Request::new(HttpAccessRequest {
            repo_path: repo_path.to_string(),
            action: action.to_string(),
            auth: Some(http_access_request::Auth::BasicAuth(BasicAuth {
                username: username.to_string(),
                password: password.to_string(),
            })),
        });
        self.add_auth(&mut request);

        let response = self.client.check_http_access(request).await?;
        let resp = response.into_inner();

        Ok(HttpAccessResult {
            allowed: resp.status,
            message: resp.message,
            user_id: resp.user_id,
            username: resp.username,
            can_write: resp.can_write,
            project_id: resp.project_id,
            repository_path: resp.repository_path,
            gitlayer_address: resp.gitlayer_address,
        })
    }

    /// 检查 HTTP 访问权限 (无认证，用于公开仓库)
    pub async fn check_http_access_anonymous(
        &mut self,
        repo_path: &str,
        action: &str,
    ) -> Result<HttpAccessResult, Box<dyn std::error::Error + Send + Sync>> {
        debug!("CheckHTTPAccess (anonymous): repo={}, action={}", repo_path, action);

        let mut request = Request::new(HttpAccessRequest {
            repo_path: repo_path.to_string(),
            action: action.to_string(),
            auth: None,
        });
        self.add_auth(&mut request);

        let response = self.client.check_http_access(request).await?;
        let resp = response.into_inner();

        Ok(HttpAccessResult {
            allowed: resp.status,
            message: resp.message,
            user_id: resp.user_id,
            username: resp.username,
            can_write: resp.can_write,
            project_id: resp.project_id,
            repository_path: resp.repository_path,
            gitlayer_address: resp.gitlayer_address,
        })
    }

    /// 通知 post-receive
    pub async fn notify_post_receive(
        &mut self,
        user_id: i64,
        repository: &str,
        project_id: i64,
        changes: Vec<(String, String, String)>, // (old_sha, new_sha, ref_name)
    ) -> Result<PostReceiveResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!(
            "NotifyPostReceive: user_id={}, repo={}, changes={}",
            user_id,
            repository,
            changes.len()
        );

        let mut request = Request::new(PostReceiveRequest {
            user_id,
            repository: repository.to_string(),
            project_id,
            changes: changes
                .into_iter()
                .map(|(old, new, ref_name)| RefChange {
                    old_sha: old,
                    new_sha: new,
                    ref_name,
                })
                .collect(),
        });
        self.add_auth(&mut request);

        let response = self.client.notify_post_receive(request).await?;
        Ok(response.into_inner())
    }
    
    /// 验证GPG签名
    pub async fn verify_gpg_signature(
        &mut self,
        commit_sha: &str,
        signature: &str,
        signed_data: &str,
        committer_email: &str,
        project_id: i64,
    ) -> Result<GpgVerifyResult, Box<dyn std::error::Error + Send + Sync>> {
        debug!(
            "VerifyGpgSignature: commit={}, email={}",
            commit_sha, committer_email
        );
        
        let mut request = Request::new(VerifyGpgSignatureRequest {
            commit_sha: commit_sha.to_string(),
            signature: signature.to_string(),
            signed_data: signed_data.to_string(),
            committer_email: committer_email.to_string(),
            project_id,
        });
        self.add_auth(&mut request);
        
        let response = self.client.verify_gpg_signature(request).await?;
        let resp = response.into_inner();
        
        Ok(GpgVerifyResult {
            valid: resp.valid,
            status: resp.status,
            message: resp.message,
            key_id: resp.key_id,
            signer_user_id: resp.signer_user_id,
            signer_username: resp.signer_username,
        })
    }
    
    /// 查找GPG密钥
    pub async fn find_gpg_key(
        &mut self,
        key_id: &str,
    ) -> Result<GpgKeyInfo, Box<dyn std::error::Error + Send + Sync>> {
        debug!("FindGpgKey: key_id={}", key_id);
        
        let mut request = Request::new(FindGpgKeyRequest {
            key_id: key_id.to_string(),
        });
        self.add_auth(&mut request);
        
        let response = self.client.find_gpg_key(request).await?;
        let resp = response.into_inner();
        
        Ok(GpgKeyInfo {
            found: resp.found,
            id: resp.id,
            user_id: resp.user_id,
            username: resp.username,
            primary_key_id: resp.primary_key_id,
            fingerprint: resp.fingerprint,
            emails: resp.emails,
            verified: resp.verified,
            revoked: resp.revoked,
            expired: resp.expired,
        })
    }
}
