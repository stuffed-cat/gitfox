//! SSH Server implementation for gitfox-shell
//!
//! This module implements a standalone SSH server (like gitlab-shell)
//! that handles Git operations over SSH via GitLayer gRPC.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use russh::server::{self, Auth, Handler, Msg, Session};
use russh::{Channel, ChannelId, CryptoVec, MethodSet};
use russh_keys::key::PublicKey;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

use crate::auth_client::AuthClient;
use crate::command::{GitAction, GitCommand};
use crate::config::Config;
use crate::error::ShellError;
use crate::gitlayer_client::{GitLayerClient, GitLayerConfig};

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

        let listener = tokio::net::TcpListener::bind(&self.config.listen_addr)
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
    /// Sender for forwarding SSH data to GitLayer
    /// None if no git command is being executed
    data_sender: Option<mpsc::Sender<Vec<u8>>>,
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
            data_sender: None,
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
        channel: ChannelId,
        data: &[u8],
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Received {} bytes of data on channel {:?}", data.len(), channel);
        
        // Forward data to the GitLayer stream if active
        if let Some(state) = self.channels.get(&channel) {
            if let Some(sender) = &state.data_sender {
                if let Err(e) = sender.send(data.to_vec()).await {
                    debug!("Failed to forward data to GitLayer: {} (channel might be closed)", e);
                }
            }
        }
        
        Ok(())
    }

    /// Handle EOF from client
    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Channel EOF: {:?}", channel);
        
        // Close the data sender to signal EOF to GitLayer
        if let Some(state) = self.channels.get_mut(&channel) {
            state.data_sender = None;
        }
        
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
        
        // 这些地址在 Config::load() 中已验证为必需
        let auth_addr = config.auth_grpc_address.as_ref().unwrap();
        
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
    }

    /// Check access for git command
    async fn check_access(
        &self,
        key_id: &str,
        git_command: &GitCommand,
    ) -> Result<AccessInfo, ShellError> {
        let config = &self.config.app_config;
        
        // 这些地址在 Config::load() 中已验证为必需
        let auth_addr = config.auth_grpc_address.as_ref().unwrap();
        
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
            project_id: if response.project_id > 0 { Some(response.project_id) } else { None },
            gitlayer_address: if response.gitlayer_address.is_empty() {
                None
            } else {
                Some(response.gitlayer_address)
            },
        })
    }

    /// Execute git command
    async fn execute_git_command(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
        git_command: &GitCommand,
        access_info: &AccessInfo,
    ) -> Result<(), ShellError> {
        let config = &self.config.app_config;
        
        // GitLayer is required for all Git operations
        // 地址在 Config::load() 中已验证为必需
        let gitlayer_addr = access_info.gitlayer_address.as_ref()
            .or(config.gitlayer_address.as_ref())
            .unwrap()
            .clone();
        
        // Create channel for SSH -> GitLayer data forwarding
        let (tx, rx) = mpsc::channel::<Vec<u8>>(1024);
        
        // Store sender in channel state for data callback to use
        if let Some(state) = self.channels.get_mut(&channel) {
            state.data_sender = Some(tx);
        }
        
        // Get session handle for sending data back from spawned task
        let handle = session.handle();
        
        // Build environment variables for GitLayer
        let mut env_vars = HashMap::new();
        env_vars.insert("GL_ID".to_string(), format!("user-{}", access_info.user_id));
        env_vars.insert("GL_USERNAME".to_string(), access_info.username.clone());
        env_vars.insert("GL_REPOSITORY".to_string(), git_command.repo_path.clone());
        env_vars.insert("GL_PROTOCOL".to_string(), "ssh".to_string());
        env_vars.insert("GITFOX_USER_ID".to_string(), access_info.user_id.to_string());
        env_vars.insert("GITFOX_USERNAME".to_string(), access_info.username.clone());
        
        if let Some(project_id) = access_info.project_id {
            env_vars.insert("GL_PROJECT_PATH".to_string(), git_command.repo_path.clone());
            env_vars.insert("GITFOX_PROJECT_ID".to_string(), project_id.to_string());
        }
        
        let repo_path = git_command.repo_path.clone();
        let action = git_command.action;
        
        // Spawn task to handle GitLayer communication
        tokio::spawn(async move {
            let result = execute_gitlayer_streaming(
                channel,
                handle.clone(),
                &gitlayer_addr,
                &repo_path,
                action,
                env_vars,
                rx,
            ).await;
            
            match result {
                Ok(exit_code) => {
                    debug!("GitLayer streaming completed with exit code {}", exit_code);
                    let _ = handle.exit_status_request(channel, exit_code).await;
                    if exit_code == 0 {
                        let _ = handle.close(channel).await;
                    }
                }
                Err(e) => {
                    error!("GitLayer streaming error: {}", e);
                    let error_msg = format!("GitFox: Git operation failed - {}\r\n", e);
                    let _ = handle.data(channel, CryptoVec::from_slice(error_msg.as_bytes())).await;
                    let _ = handle.exit_status_request(channel, 1).await;
                    let _ = handle.close(channel).await;
                }
            }
        });
        
        // exec_request returns immediately, streaming happens in background task
        session.channel_success(channel);
        Ok(())
    }
}

/// Execute git operation via GitLayer gRPC streaming
///
/// This function handles the bidirectional streaming between SSH client and GitLayer:
/// - Receives data from SSH client via rx channel
/// - Sends data to GitLayer via gRPC stream
/// - Receives responses from GitLayer and sends back to SSH client via handle
async fn execute_gitlayer_streaming(
    channel: ChannelId,
    handle: russh::server::Handle,
    gitlayer_addr: &str,
    repo_path: &str,
    action: GitAction,
    env_vars: HashMap<String, String>,
    rx: mpsc::Receiver<Vec<u8>>,
) -> Result<u32, ShellError> {
    debug!("Starting GitLayer streaming to {} for {}", gitlayer_addr, repo_path);
    
    // Connect to GitLayer
    let gitlayer_config = GitLayerConfig {
        address: gitlayer_addr.to_string(),
        connect_timeout: 10,
    };
    
    let client = GitLayerClient::connect(gitlayer_config).await?;
    
    // Convert receiver to stream
    let stdin_stream = ReceiverStream::new(rx);
    
    // Execute via GitLayer based on action
    let mut exit_code = 0u32;
    
    match action {
        GitAction::UploadPack => {
            let mut output_stream = client.ssh_upload_pack(repo_path, env_vars, stdin_stream).await?;
            
            while let Some(result) = output_stream.next().await {
                match result {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            if let Err(e) = handle.data(channel, CryptoVec::from_slice(&output.stdout)).await {
                                debug!("Failed to send stdout to SSH client: {:?}", e);
                                break;
                            }
                        }
                        if !output.stderr.is_empty() {
                            // SSH extended data type 1 = stderr
                            if let Err(e) = handle.extended_data(channel, 1, CryptoVec::from_slice(&output.stderr)).await {
                                debug!("Failed to send stderr to SSH client: {:?}", e);
                            }
                        }
                        if output.exit_code != 0 {
                            exit_code = output.exit_code as u32;
                        }
                    }
                    Err(e) => {
                        error!("GitLayer stream error: {}", e);
                        return Err(e);
                    }
                }
            }
        }
        GitAction::ReceivePack => {
            let mut output_stream = client.ssh_receive_pack(repo_path, env_vars, stdin_stream).await?;
            
            while let Some(result) = output_stream.next().await {
                match result {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            if let Err(e) = handle.data(channel, CryptoVec::from_slice(&output.stdout)).await {
                                debug!("Failed to send stdout to SSH client: {:?}", e);
                                break;
                            }
                        }
                        if !output.stderr.is_empty() {
                            if let Err(e) = handle.extended_data(channel, 1, CryptoVec::from_slice(&output.stderr)).await {
                                debug!("Failed to send stderr to SSH client: {:?}", e);
                            }
                        }
                        if output.exit_code != 0 {
                            exit_code = output.exit_code as u32;
                        }
                    }
                    Err(e) => {
                        error!("GitLayer stream error: {}", e);
                        return Err(e);
                    }
                }
            }
        }
        GitAction::UploadArchive => {
            // Upload-archive via GitLayer
            // TODO: Implement dedicated UploadArchive in GitLayer if needed
            return Err(ShellError::GitExecution("git-upload-archive via GitLayer not yet implemented".to_string()));
        }
        GitAction::LfsAuthenticate => {
            // LFS authenticate is handled separately in command.rs
            unreachable!("LFS authenticate should be handled separately");
        }
    }
    
    info!("GitLayer streaming completed for {} with exit code {}", repo_path, exit_code);
    Ok(exit_code)
}

/// Access information from auth check
#[derive(Debug, Clone)]
pub struct AccessInfo {
    pub user_id: i64,
    pub username: String,
    pub can_write: bool,
    pub project_id: Option<i64>,
    pub gitlayer_address: Option<String>,
}
