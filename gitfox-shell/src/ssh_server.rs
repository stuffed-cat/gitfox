//! SSH Server implementation for gitfox-shell
//!
//! This module implements a standalone SSH server (like gitlab-shell)
//! that handles Git operations over SSH.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use russh::server::{self, Auth, Handler, Msg, Session};
use russh::{Channel, ChannelId, CryptoVec, MethodSet};
use russh_keys::key::PublicKey;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

use crate::auth_client::AuthClient;
use crate::command::{GitAction, GitCommand};
use crate::config::Config;
use crate::error::ShellError;

/// SSH Server configuration
pub struct SshServerConfig {
    /// SSH listen address
    pub listen_addr: SocketAddr,
    /// Path to SSH host key
    pub host_key_path: PathBuf,
    /// Application config
    pub app_config: Config,
}

/// SSH Server
pub struct SshServer {
    config: Arc<SshServerConfig>,
}

impl SshServer {
    pub fn new(config: SshServerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Start the SSH server
    pub async fn run(self) -> Result<(), ShellError> {
        let host_key = load_or_generate_host_key(&self.config.host_key_path).await?;
        
        let russh_config = Arc::new(russh::server::Config {
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![host_key],
            ..Default::default()
        });

        info!("Starting SSH server on {}", self.config.listen_addr);

        let listener = TcpListener::bind(&self.config.listen_addr)
            .await
            .map_err(|e| ShellError::Ssh(format!("Failed to bind: {}", e)))?;

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("New connection from {}", peer_addr);
                    let config = russh_config.clone();
                    let app_config = self.config.clone();
                    
                    tokio::spawn(async move {
                        let handler = SshHandler {
                            config: app_config,
                            peer_addr: Some(peer_addr),
                            user: None,
                            key_id: None,
                            channels: HashMap::new(),
                        };
                        
                        if let Err(e) = server::run_stream(config, stream, handler).await {
                            error!("SSH session error from {}: {:?}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Load or generate SSH host key
async fn load_or_generate_host_key(path: &PathBuf) -> Result<russh_keys::key::KeyPair, ShellError> {
    if path.exists() {
        // Load existing key
        let key_data = tokio::fs::read(path)
            .await
            .map_err(|e| ShellError::Ssh(format!("Failed to read host key: {}", e)))?;
        
        russh_keys::decode_secret_key(&String::from_utf8_lossy(&key_data), None)
            .map_err(|e| ShellError::Ssh(format!("Failed to decode host key: {}", e)))
    } else {
        // Generate new key
        info!("Generating new SSH host key at {:?}", path);
        let key = russh_keys::key::KeyPair::generate_ed25519()
            .ok_or_else(|| ShellError::Ssh("Failed to generate host key".to_string()))?;
        
        // Save the key - for now just warn that key should be pre-generated
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ShellError::Ssh(format!("Failed to create key directory: {}", e)))?;
        }
        
        // Note: In production, pre-generate the key using ssh-keygen
        warn!("Generated ephemeral host key - pre-generate with ssh-keygen for production");
        
        Ok(key)
    }
}

/// Per-connection SSH handler
struct SshHandler {
    config: Arc<SshServerConfig>,
    peer_addr: Option<SocketAddr>,
    user: Option<String>,
    key_id: Option<String>,
    channels: HashMap<ChannelId, ChannelState>,
}

struct ChannelState {
    /// Environment variables set by client
    env: HashMap<String, String>,
    /// Git command being executed
    git_command: Option<GitCommand>,
}

#[async_trait]
impl Handler for SshHandler {
    type Error = ShellError;

    /// Handle public key authentication
    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        debug!("Public key auth attempt for user: {}", user);
        
        // Extract key fingerprint for lookup
        let key_fingerprint = public_key.fingerprint();
        debug!("Key fingerprint: {}", key_fingerprint);
        
        // Check with auth service
        let auth_result = self.verify_public_key(user, public_key).await;
        
        match auth_result {
            Ok(key_id) => {
                info!("Auth success for user {} with key {}", user, key_id);
                self.user = Some(user.to_string());
                self.key_id = Some(key_id);
                Ok(Auth::Accept)
            }
            Err(e) => {
                warn!("Auth failed for user {}: {}", user, e);
                Ok(Auth::Reject {
                    proceed_with_methods: Some(MethodSet::PUBLICKEY),
                })
            }
        }
    }

    /// Handle channel open
    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let channel_id = channel.id();
        debug!("Channel opened: {:?}", channel_id);
        
        self.channels.insert(channel_id, ChannelState {
            env: HashMap::new(),
            git_command: None,
        });
        
        Ok(true)
    }

    /// Handle environment variable request
    async fn env_request(
        &mut self,
        channel: ChannelId,
        variable_name: &str,
        variable_value: &str,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Env request: {}={}", variable_name, variable_value);
        
        if let Some(state) = self.channels.get_mut(&channel) {
            state.env.insert(variable_name.to_string(), variable_value.to_string());
        }
        
        Ok(())
    }

    /// Handle exec request (git-upload-pack, git-receive-pack)
    async fn exec_request(
        &mut self,
        channel: ChannelId,
        command: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let command_str = String::from_utf8_lossy(command);
        info!("Exec request: {}", command_str);
        
        let key_id = self.key_id.clone()
            .ok_or_else(|| ShellError::Auth("Not authenticated".to_string()))?;
        
        // Parse git command
        let git_command = match GitCommand::parse(&command_str) {
            Ok(cmd) => cmd,
            Err(e) => {
                error!("Failed to parse command: {}", e);
                session.channel_failure(channel);
                return Ok(());
            }
        };
        
        // Check access
        let access_result = self.check_access(&key_id, &git_command).await;
        
        match access_result {
            Ok(access_info) => {
                info!(
                    "Access granted for user {} on repo {} (write: {})",
                    access_info.user_id, git_command.repo_path, access_info.can_write
                );
                
                // Store command state
                if let Some(state) = self.channels.get_mut(&channel) {
                    state.git_command = Some(git_command.clone());
                }
                
                // Execute the command
                let result = self.execute_git_command(
                    channel,
                    session,
                    &git_command,
                    &access_info,
                ).await;
                
                if let Err(e) = result {
                    error!("Git command execution failed: {}", e);
                    session.channel_failure(channel);
                }
            }
            Err(e) => {
                warn!("Access denied: {}", e);
                // Send error message
                let error_msg = format!("GitFox: Access denied - {}\n", e);
                session.data(channel, CryptoVec::from_slice(error_msg.as_bytes()));
                session.channel_failure(channel);
            }
        }
        
        Ok(())
    }

    /// Handle data from client
    async fn data(
        &mut self,
        _channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Received {} bytes of data", data.len());
        // Data is handled by the git process stdin
        Ok(())
    }

    /// Handle channel close
    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Channel closed: {:?}", channel);
        self.channels.remove(&channel);
        Ok(())
    }
}

impl SshHandler {
    /// Verify public key with auth service
    async fn verify_public_key(
        &self,
        _user: &str,
        public_key: &PublicKey,
    ) -> Result<String, ShellError> {
        let config = &self.config.app_config;
        
        let key_fingerprint = public_key.fingerprint();
        
        if config.use_grpc_auth {
            let auth_addr = config.auth_grpc_address.as_ref()
                .ok_or_else(|| ShellError::Config("AUTH_GRPC_ADDRESS not configured".to_string()))?;
            
            let mut auth_client = AuthClient::connect(auth_addr, config.api_secret.clone())
                .await
                .map_err(|e| ShellError::Auth(format!("Failed to connect to Auth service: {}", e)))?;
            
            // Find key by fingerprint
            let key_info = auth_client
                .find_ssh_key(&key_fingerprint)
                .await
                .map_err(|e| ShellError::Auth(format!("Key lookup failed: {}", e)))?;
            
            match key_info {
                Some(info) => Ok(format!("key-{}", info.id)),
                None => Err(ShellError::Auth("Public key not found".to_string())),
            }
        } else {
            // Legacy HTTP API mode
            Err(ShellError::Auth("HTTP key lookup not implemented for SSH server mode".to_string()))
        }
    }

    /// Check access for git command
    async fn check_access(
        &self,
        key_id: &str,
        git_command: &GitCommand,
    ) -> Result<AccessInfo, ShellError> {
        let config = &self.config.app_config;
        
        let auth_addr = config.auth_grpc_address.as_ref()
            .ok_or_else(|| ShellError::Config("AUTH_GRPC_ADDRESS not configured".to_string()))?;
        
        let mut auth_client = AuthClient::connect(auth_addr, config.api_secret.clone())
            .await
            .map_err(|e| ShellError::Auth(format!("Failed to connect to Auth service: {}", e)))?;
        
        let action = if git_command.action.requires_write() {
            "git-receive-pack"
        } else {
            "git-upload-pack"
        };
        
        let response = auth_client
            .check_ssh_access(key_id, &git_command.repo_path, action)
            .await
            .map_err(|e| ShellError::Auth(format!("Auth check failed: {}", e)))?;
        
        if !response.status {
            return Err(ShellError::AccessDenied(response.message));
        }
        
        Ok(AccessInfo {
            user_id: response.user_id,
            username: response.username,
            can_write: response.can_write,
            gitlayer_address: if response.gitlayer_address.is_empty() {
                None
            } else {
                Some(response.gitlayer_address)
            },
        })
    }

    /// Execute git command
    async fn execute_git_command(
        &self,
        channel: ChannelId,
        session: &mut Session,
        git_command: &GitCommand,
        access_info: &AccessInfo,
    ) -> Result<(), ShellError> {
        let config = &self.config.app_config;
        
        // Build repository path
        let repo_path = config.repo_path(&git_command.repo_path);
        
        // GitLayer is required for all Git operations
        let gitlayer_addr = access_info.gitlayer_address.as_ref()
            .or(config.gitlayer_address.as_ref())
            .ok_or_else(|| ShellError::Config(
                "GITLAYER_ADDRESS not configured. GitLayer is required for all Git operations.".to_string()
            ))?;
        
        self.execute_via_gitlayer(
            channel,
            session,
            git_command,
            gitlayer_addr,
            &repo_path,
        ).await
    }

    /// Execute git via GitLayer gRPC
    async fn execute_via_gitlayer(
        &self,
        channel: ChannelId,
        session: &mut Session,
        git_command: &GitCommand,
        gitlayer_addr: &str,
        repo_path: &str,
    ) -> Result<(), ShellError> {
        debug!("Executing via GitLayer at {}", gitlayer_addr);
        
        // TODO: Implement proper GitLayer streaming
        // For now, fall back to direct execution
        // The full implementation requires bidirectional stream handling
        // between SSH client and GitLayer gRPC service
        
        warn!("GitLayer streaming not yet implemented, falling back to direct execution");
        
        let actual_repo_path = format!("{}/{}.git", self.config.app_config.repos_path, repo_path);
        
        self.execute_git_directly(channel, session, git_command, &actual_repo_path).await
    }

    /// Execute git directly (fallback when GitLayer is not available)
    async fn execute_git_directly(
        &self,
        channel: ChannelId,
        session: &mut Session,
        git_command: &GitCommand,
        repo_path: &str,
    ) -> Result<(), ShellError> {
        debug!("Executing git directly for {}", repo_path);
        
        let git_bin = git_command.action.binary_name();
        
        let mut cmd = Command::new(git_bin);
        cmd.arg(repo_path);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        
        let mut child = cmd
            .spawn()
            .map_err(|e| ShellError::Git(format!("Failed to spawn git: {}", e)))?;
        
        // TODO: Properly handle bidirectional data transfer
        // This is a simplified version
        
        if let Some(mut stdout) = child.stdout.take() {
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf)
                .await
                .map_err(|e| ShellError::Git(format!("Failed to read git output: {}", e)))?;
            session.data(channel, CryptoVec::from_slice(&buf));
        }
        
        let status = child
            .wait()
            .await
            .map_err(|e| ShellError::Git(format!("Failed to wait for git: {}", e)))?;
        
        let exit_code = status.code().unwrap_or(1) as u32;
        session.exit_status_request(channel, exit_code);
        
        if exit_code == 0 {
            session.channel_success(channel);
        } else {
            session.channel_failure(channel);
        }
        
        Ok(())
    }
}

/// Access information from auth check
#[derive(Debug, Clone)]
pub struct AccessInfo {
    pub user_id: i64,
    pub username: String,
    pub can_write: bool,
    pub gitlayer_address: Option<String>,
}
