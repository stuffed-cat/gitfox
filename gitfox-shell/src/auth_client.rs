//! Auth gRPC Client
//!
//! 通过 gRPC 调用主应用的 Auth 服务进行权限认证。
//! 这是 GitLab 架构模式：Shell 通过 RPC 与主应用通信，主应用只负责权限管理。

use tonic::transport::Channel;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{debug, info};

// 导入生成的 proto 代码
pub mod auth_proto {
    tonic::include_proto!("gitfox.auth");
}

pub use auth_proto::*;

use auth_proto::auth_service_client::AuthServiceClient;

/// Auth gRPC 客户端
#[derive(Clone)]
pub struct AuthClient {
    client: AuthServiceClient<Channel>,
    shell_token: String,
}

impl AuthClient {
    /// 创建新的 Auth 客户端
    pub async fn connect(address: &str, shell_token: String) -> Result<Self, Box<dyn std::error::Error>> {
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

    /// 检查 SSH 访问权限
    pub async fn check_ssh_access(
        &mut self,
        key_id: &str,
        repo_path: &str,
        action: &str,
    ) -> Result<SshAccessResponse, Box<dyn std::error::Error>> {
        debug!("CheckSSHAccess: key_id={}, repo={}, action={}", key_id, repo_path, action);

        let mut request = Request::new(SshAccessRequest {
            key_id: key_id.to_string(),
            repo_path: repo_path.to_string(),
            action: action.to_string(),
            protocol: "ssh".to_string(),
        });
        self.add_auth(&mut request);

        let response = self.client.check_ssh_access(request).await?;
        let resp = response.into_inner();

        if resp.status {
            info!(
                "SSH access granted for user {} on repo {} (write: {})",
                resp.username, repo_path, resp.can_write
            );
        } else {
            info!("SSH access denied: {}", resp.message);
        }

        Ok(resp)
    }

    /// 通过指纹查找 SSH 密钥
    pub async fn find_ssh_key(
        &mut self,
        fingerprint: &str,
    ) -> Result<Option<SshKeyInfo>, Box<dyn std::error::Error>> {
        debug!("FindSSHKey: fingerprint={}", fingerprint);

        let mut request = Request::new(FindSshKeyRequest {
            fingerprint: fingerprint.to_string(),
        });
        self.add_auth(&mut request);

        let response = self.client.find_ssh_key(request).await?;
        let resp = response.into_inner();

        if resp.found {
            Ok(Some(SshKeyInfo {
                id: resp.id,
                user_id: resp.user_id,
                username: resp.username,
                key_type: resp.key_type,
                public_key: resp.public_key,
            }))
        } else {
            Ok(None)
        }
    }

    /// 通过 ID 获取 SSH 密钥
    pub async fn get_ssh_key(
        &mut self,
        id: i64,
    ) -> Result<Option<SshKeyInfo>, Box<dyn std::error::Error>> {
        let mut request = Request::new(GetSshKeyRequest { id });
        self.add_auth(&mut request);

        let response = self.client.get_ssh_key(request).await?;
        let resp = response.into_inner();

        if resp.found {
            Ok(Some(SshKeyInfo {
                id: resp.id,
                user_id: resp.user_id,
                username: resp.username,
                key_type: resp.key_type,
                public_key: resp.public_key,
            }))
        } else {
            Ok(None)
        }
    }

    /// 通知 post-receive
    pub async fn notify_post_receive(
        &mut self,
        user_id: i64,
        repository: &str,
        project_id: i64,
        changes: Vec<(String, String, String)>, // (old_sha, new_sha, ref_name)
    ) -> Result<PostReceiveResponse, Box<dyn std::error::Error>> {
        debug!(
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

    /// 检查 ref 更新权限
    pub async fn check_ref_update(
        &mut self,
        user_id: i64,
        repository: &str,
        project_id: i64,
        ref_name: &str,
        old_sha: &str,
        new_sha: &str,
        change_type: &str,
    ) -> Result<RefUpdateResponse, Box<dyn std::error::Error>> {
        debug!(
            "CheckRefUpdate: user={}, repo={}, ref={}",
            user_id, repository, ref_name
        );

        let mut request = Request::new(RefUpdateRequest {
            user_id,
            repository: repository.to_string(),
            project_id,
            ref_name: ref_name.to_string(),
            old_sha: old_sha.to_string(),
            new_sha: new_sha.to_string(),
            change_type: change_type.to_string(),
        });
        self.add_auth(&mut request);

        let response = self.client.check_ref_update(request).await?;
        Ok(response.into_inner())
    }

    /// 生成 LFS 认证 token
    pub async fn generate_lfs_token(
        &mut self,
        user_id: i64,
        username: &str,
        repo_path: &str,
        operation: &str,
        project_id: Option<i64>,
    ) -> Result<LfsTokenResponse, Box<dyn std::error::Error>> {
        debug!(
            "GenerateLfsToken: user_id={}, repo={}, operation={}",
            user_id, repo_path, operation
        );

        let mut request = Request::new(LfsTokenRequest {
            user_id,
            username: username.to_string(),
            repo_path: repo_path.to_string(),
            operation: operation.to_string(),
            project_id: project_id.unwrap_or(0),
        });
        self.add_auth(&mut request);

        let response = self.client.generate_lfs_token(request).await?;
        Ok(response.into_inner())
    }
}

/// SSH Key 信息
#[derive(Debug, Clone)]
pub struct SshKeyInfo {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub key_type: String,
    pub public_key: String,
}
