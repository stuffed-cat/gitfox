//! SSH handler for Git operations

use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use russh::server::{Auth, Handler, Msg, Session};
use russh::{Channel, ChannelId, CryptoVec};
use russh_keys::key::PublicKey;
use russh_keys::PublicKeyBase64;
use sqlx::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use super::session::{GitAction, GitSession};
use crate::config::Config;

/// SSH handler for processing Git commands
pub struct GitSshHandler {
    session: GitSession,
    client_addr: SocketAddr,
}

impl GitSshHandler {
    /// Create a new handler
    pub fn new(config: Arc<Config>, pool: Arc<PgPool>, client_addr: SocketAddr) -> Self {
        Self {
            session: GitSession::new(config, pool),
            client_addr,
        }
    }

    /// Authenticate a user by their public key
    async fn authenticate_key(&self, key: &PublicKey) -> Option<(i64, String, i64)> {
        // Compute fingerprint
        let fingerprint = compute_key_fingerprint(key);
        
        debug!("SSH auth attempt");

        // Look up key in database
        let result = sqlx::query_as::<_, (i64, i64, String)>(
            r#"
            SELECT k.id, k.user_id, u.username
            FROM ssh_keys k
            JOIN users u ON k.user_id = u.id
            WHERE k.fingerprint = $1
              AND (k.expires_at IS NULL OR k.expires_at > NOW())
              AND u.is_active = true
            "#,
        )
        .bind(&fingerprint)
        .fetch_optional(self.session.pool.as_ref())
        .await;

        match result {
            Ok(Some((key_id, user_id, username))) => {
                info!(
                    "SSH key authentication successful: user={}, key_id={}",
                    username, key_id
                );
                
                // Update last_used_at
                let _ = sqlx::query("UPDATE ssh_keys SET last_used_at = NOW() WHERE id = $1")
                    .bind(key_id)
                    .execute(self.session.pool.as_ref())
                    .await;

                Some((user_id, username, key_id))
            }
            Ok(None) => {
                debug!("SSH key not found: {}", fingerprint);
                None
            }
            Err(e) => {
                error!("Database error during SSH authentication: {}", e);
                None
            }
        }
    }

    // Note: Access control is now handled by gitfox-shell via internal API
    // The check_access method has been removed in favor of delegating to gitfox-shell

    /// Execute a Git command on the given channel via gitfox-shell
    /// This ensures all access control is handled by gitfox-shell
    async fn execute_git_command_on_channel(
        &mut self,
        channel_id: ChannelId,
        session: &mut Session,
        _action: GitAction,
        repo_path: &str,
        original_command: &str,
    ) -> Result<(), String> {
        let config = &self.session.config;
        let key_id = self.session.key_id.ok_or("No SSH key ID available")?;
        
        info!(
            "Executing gitfox-shell for command '{}' on {} for user {:?} (key_id={})",
            original_command,
            repo_path,
            self.session.username,
            key_id
        );

        // Spawn gitfox-shell process
        // gitfox-shell reads SSH_ORIGINAL_COMMAND and handles access control
        // key_id is passed in format "key-123" as expected by internal API
        let mut child = Command::new(&config.gitfox_shell_path)
            .arg(format!("key-{}", key_id))
            .env("SSH_ORIGINAL_COMMAND", original_command)
            .env("GITFOX_API_URL", &config.internal_api_url)
            .env("GITFOX_API_SECRET", &config.shell_secret)
            .env("GITFOX_REPOS_PATH", &config.git_repos_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn gitfox-shell: {}", e))?;

        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();

        // Create channel for stdin data
        let (stdin_tx, mut stdin_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(100);
        
        // Store the sender so data callback can use it
        {
            let mut stdin_map = self.session.git_stdin.lock().await;
            stdin_map.insert(channel_id, stdin_tx);
        }

        // Get session handle for async operations
        let session_handle = session.handle();
        let git_stdin = self.session.git_stdin.clone();

        // Spawn a task to handle the entire git process lifecycle
        tokio::spawn(async move {
            // Handle stdin from client
            let stdin_handle = tokio::spawn(async move {
                while let Some(data) = stdin_rx.recv().await {
                    debug!("Writing {} bytes to git stdin", data.len());
                    if stdin.write_all(&data).await.is_err() {
                        break;
                    }
                }
                debug!("Git stdin closed");
                drop(stdin);
            });

            // Send stdout to channel
            let stdout_session = session_handle.clone();
            let stdout_handle = tokio::spawn(async move {
                let mut buf = vec![0u8; 32768];
                loop {
                    match stdout.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            debug!("Read {} bytes from git stdout", n);
                            let data = CryptoVec::from_slice(&buf[..n]);
                            if stdout_session.data(channel_id, data).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Error reading from git stdout: {}", e);
                            break;
                        }
                    }
                }
                debug!("Git stdout closed");
            });

            // Send stderr to channel
            let stderr_session = session_handle.clone();
            let stderr_handle = tokio::spawn(async move {
                let mut buf = vec![0u8; 4096];
                loop {
                    match stderr.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            debug!("Read {} bytes from git stderr", n);
                            let data = CryptoVec::from_slice(&buf[..n]);
                            if stderr_session.extended_data(channel_id, 1, data).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Error reading from git stderr: {}", e);
                            break;
                        }
                    }
                }
                debug!("Git stderr closed");
            });

            // Wait for the process to complete
            let status = child.wait().await;
            
            // Wait for stdout/stderr to finish
            let _ = stdout_handle.await;
            let _ = stderr_handle.await;
            stdin_handle.abort();

            // Remove stdin sender
            {
                let mut stdin_map = git_stdin.lock().await;
                stdin_map.remove(&channel_id);
            }

            // Send exit status
            match status {
                Ok(status) => {
                    let exit_code = status.code().unwrap_or(1) as u32;
                    let _ = session_handle.exit_status_request(channel_id, exit_code).await;
                    let _ = session_handle.close(channel_id).await;
                    if status.success() {
                        info!("Git command completed successfully");
                    } else {
                        error!("Git command failed with exit code {}", exit_code);
                    }
                }
                Err(e) => {
                    error!("Failed to wait for git process: {}", e);
                    let _ = session_handle.exit_status_request(channel_id, 1).await;
                    let _ = session_handle.close(channel_id).await;
                }
            }
        });

        // Return immediately - the git process runs in the background
        Ok(())
    }
}

#[async_trait]
impl Handler for GitSshHandler {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        debug!("Channel open session request from {}", self.client_addr);
        Ok(true)
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        debug!("Public key auth attempt for user '{}' from {}", user, self.client_addr);

        // We only accept 'git' user
        if user != "git" {
            warn!("Rejected auth for non-git user: {}", user);
            return Ok(Auth::Reject { proceed_with_methods: None });
        }

        match self.authenticate_key(key).await {
            Some((user_id, username, key_id)) => {
                self.session.authenticate(user_id, username, key_id);
                Ok(Auth::Accept)
            }
            None => {
                Ok(Auth::Reject { proceed_with_methods: None })
            }
        }
    }

    async fn auth_none(&mut self, user: &str) -> Result<Auth, Self::Error> {
        // Never accept auth_none
        Ok(Auth::Reject {
            proceed_with_methods: Some(russh::MethodSet::PUBLICKEY),
        })
    }

    async fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> Result<Auth, Self::Error> {
        // Don't accept password auth
        Ok(Auth::Reject {
            proceed_with_methods: Some(russh::MethodSet::PUBLICKEY),
        })
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let command = String::from_utf8_lossy(data).to_string();
        debug!("Exec request: '{}' from {:?}", command, self.session.username);

        if !self.session.is_authenticated() {
            warn!("Exec request from unauthenticated session");
            session.channel_failure(channel);
            return Ok(());
        }

        // Parse the Git command to validate it's a git operation
        let (action, repo_path) = match GitSession::parse_git_command(&command) {
            Some(parsed) => parsed,
            None => {
                // Not a git command, show welcome message
                let msg = format!(
                    "Welcome to GitFox, {}!\n\n\
                     GitFox does not provide shell access.\n\
                     Please use Git over SSH for repository operations.\n\n\
                     Example:\n  git clone git@{}:{}.git\n",
                    self.session.get_username().unwrap_or("user"),
                    self.session.config.ssh_host,
                    "owner/repo"
                );
                session.data(channel, CryptoVec::from_slice(msg.as_bytes()));
                session.close(channel);
                return Ok(());
            }
        };

        // Execute via gitfox-shell which handles access control
        session.channel_success(channel);
        
        // Execute the command through gitfox-shell
        if let Err(e) = self.execute_git_command_on_channel(channel, session, action, &repo_path, &command).await {
            error!("Git command failed: {}", e);
            let err_msg = format!("GitFox: {}\n", e);
            session.extended_data(channel, 1, CryptoVec::from_slice(err_msg.as_bytes()));
            session.exit_status_request(channel, 1);
            session.close(channel);
        }

        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        // Forward data to the git process stdin
        debug!("Received {} bytes of data on channel {:?}", data.len(), channel);
        
        let stdin_map = self.session.git_stdin.lock().await;
        if let Some(tx) = stdin_map.get(&channel) {
            if let Err(e) = tx.send(data.to_vec()).await {
                error!("Failed to send data to git stdin: {}", e);
            }
        }
        
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Channel EOF: {:?}", channel);
        
        // Remove stdin sender to signal EOF to git process
        let mut stdin_map = self.session.git_stdin.lock().await;
        stdin_map.remove(&channel);
        
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Channel close: {:?}", channel);
        
        // Remove stdin sender if not already removed
        let mut stdin_map = self.session.git_stdin.lock().await;
        stdin_map.remove(&channel);
        
        Ok(())
    }
}

/// Compute the SHA256 fingerprint of a public key (compatible with ssh-keygen format)
fn compute_key_fingerprint(key: &PublicKey) -> String {
    use base64::Engine;
    use russh_keys::PublicKeyBase64;
    
    // 使用 russh 的 public_key_base64() 获取标准 OpenSSH 格式的 base64 编码
    // 这和 ssh-keygen 使用的格式一致
    let key_base64 = key.public_key_base64();
    
    // 解码 base64 得到 wire format blob
    let blob = match base64::engine::general_purpose::STANDARD.decode(&key_base64) {
        Ok(b) => b,
        Err(_) => return "invalid".to_string(),
    };
    
    // 计算 SHA256 hash
    let hash = ring::digest::digest(&ring::digest::SHA256, &blob);
    
    // 使用标准 base64 编码（无 padding），和 ssh-keygen 一致
    let fingerprint = base64::engine::general_purpose::STANDARD_NO_PAD.encode(hash.as_ref());
    
    format!("SHA256:{}", fingerprint)
}
