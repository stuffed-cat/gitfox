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

    /// Check if user has access to a repository
    async fn check_access(
        &self,
        repo_path: &str,
        needs_write: bool,
    ) -> Result<bool, String> {
        let user_id = self.session.user_id.ok_or("Not authenticated")?;

        debug!("check_access: repo_path='{}', user_id={}, needs_write={}", repo_path, user_id, needs_write);

        // Find the project by matching owner.username/project.name
        let project = sqlx::query_as::<_, (i64, String, i64)>(
            r#"
            SELECT p.id, p.visibility::text, p.owner_id
            FROM projects p
            JOIN users u ON p.owner_id = u.id
            WHERE LOWER(CONCAT(u.username, '/', p.name)) = LOWER($1)
            "#,
        )
        .bind(repo_path)
        .fetch_optional(self.session.pool.as_ref())
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        debug!("check_access: query result for '{}': {:?}", repo_path, project);

        let project = project.ok_or_else(|| format!("Repository '{}' not found", repo_path))?;

        let (project_id, visibility, owner_id) = project;

        // Check if user is owner
        if user_id == owner_id {
            return Ok(true);
        }

        // Check project membership
        let membership = sqlx::query_scalar::<_, String>(
            r#"SELECT role::text FROM project_members WHERE project_id = $1 AND user_id = $2"#,
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(self.session.pool.as_ref())
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        match membership {
            Some(role) => {
                if needs_write {
                    Ok(matches!(role.as_str(), "owner" | "maintainer" | "developer"))
                } else {
                    Ok(true)
                }
            }
            None => {
                // No membership, check visibility
                if visibility == "public" || visibility == "internal" {
                    if needs_write {
                        Err("Write access denied".to_string())
                    } else {
                        Ok(true)
                    }
                } else {
                    Err("Access denied".to_string())
                }
            }
        }
    }

    /// Execute a Git command on the given channel
    async fn execute_git_command_on_channel(
        &self,
        channel_id: ChannelId,
        session: &mut Session,
        action: GitAction,
        repo_path: &str,
    ) -> Result<(), String> {
        let config = &self.session.config;
        let full_path = format!("{}/{}.git", config.git_repos_path, repo_path);

        // Check if repository exists
        if !std::path::Path::new(&full_path).exists() {
            return Err(format!("Repository not found: {}", repo_path));
        }

        let binary = action.binary_name();
        
        info!(
            "Executing {} on {} for user {:?}",
            binary,
            full_path,
            self.session.username
        );

        // Build environment variables
        let user_id = self.session.user_id.unwrap_or(0);
        let username = self.session.username.as_deref().unwrap_or("anonymous");

        // Spawn the git process
        let mut child = Command::new(binary)
            .arg(&full_path)
            .env("GL_ID", format!("user-{}", user_id))
            .env("GL_USERNAME", username)
            .env("GL_REPOSITORY", repo_path)
            .env("GL_PROTOCOL", "ssh")
            .env("GITFOX_USER_ID", user_id.to_string())
            .env("GITFOX_USERNAME", username)
            .env("GITFOX_REPO_PATH", repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", binary, e))?;

        let _stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();

        // Handle stdin from client
        let _stdin_handle = tokio::spawn(async move {
            // This will be fed data from channel_data callbacks
            // For now we just keep stdin open
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        });

        // Send stdout to channel
        let session_handle = session.handle();
        let stdout_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 32768];
            loop {
                match stdout.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = CryptoVec::from_slice(&buf[..n]);
                        if session_handle.data(channel_id, data).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from git stdout: {}", e);
                        break;
                    }
                }
            }
        });

        // Send stderr to channel
        let session_handle2 = session.handle();
        let stderr_handle = tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match stderr.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = CryptoVec::from_slice(&buf[..n]);
                        if session_handle2.extended_data(channel_id, 1, data).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from git stderr: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for the process to complete
        let status = child.wait().await.map_err(|e| format!("Failed to wait: {}", e))?;

        stdout_handle.abort();
        stderr_handle.abort();

        // Send exit status
        let exit_code = status.code().unwrap_or(1) as u32;
        session.exit_status_request(channel_id, exit_code);
        session.close(channel_id);

        if status.success() {
            info!("Git command completed successfully");
            Ok(())
        } else {
            Err(format!("Git command failed with exit code {}", exit_code))
        }
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

        // Parse the Git command
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

        // Check access
        match self.check_access(&repo_path, action.requires_write()).await {
            Ok(true) => {
                // Access granted, execute command
                session.channel_success(channel);
                
                // Execute directly on this channel
                if let Err(e) = self.execute_git_command_on_channel(channel, session, action, &repo_path).await {
                    error!("Git command failed: {}", e);
                    let err_msg = format!("GitFox: {}\n", e);
                    session.extended_data(channel, 1, CryptoVec::from_slice(err_msg.as_bytes()));
                    session.exit_status_request(channel, 1);
                    session.close(channel);
                }
            }
            Ok(false) => {
                debug!("check_access returned Ok(false) for {}", repo_path);
                let err_msg = format!(
                    "GitFox: Permission denied. You don't have access to {}.\n",
                    repo_path
                );
                session.extended_data(channel, 1, CryptoVec::from_slice(err_msg.as_bytes()));
                session.exit_status_request(channel, 1);
                session.close(channel);
            }
            Err(e) => {
                debug!("check_access error for {}: {}", repo_path, e);
                let err_msg = format!(
                    "GitFox: {}.\n",
                    e
                );
                session.extended_data(channel, 1, CryptoVec::from_slice(err_msg.as_bytes()));
                session.exit_status_request(channel, 1);
                session.close(channel);
            }
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
        // This needs to be connected to the spawned process
        debug!("Received {} bytes of data on channel {:?}", data.len(), channel);
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Channel EOF: {:?}", channel);
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Channel close: {:?}", channel);
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
