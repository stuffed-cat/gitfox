//! GitLayer gRPC client for gitfox-shell
//!
//! This module provides a client interface to communicate with GitLayer
//! for Git operations instead of directly executing git commands.

use std::collections::HashMap;
use std::pin::Pin;

use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;
use tracing::{debug, error, info};

use crate::error::ShellError;

// Include generated proto code
pub mod proto {
    tonic::include_proto!("gitlayer");
}

use proto::{
    ssh_service_client::SshServiceClient,
    health_service_client::HealthServiceClient,
    Repository, SshPackRequest, HealthCheckRequest,
};

/// GitLayer client configuration
#[derive(Debug, Clone)]
pub struct GitLayerConfig {
    /// GitLayer server address (e.g., "http://127.0.0.1:9000")
    pub address: String,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
}

impl Default for GitLayerConfig {
    fn default() -> Self {
        Self {
            address: "http://127.0.0.1:9000".to_string(),
            connect_timeout: 5,
        }
    }
}

/// GitLayer client
pub struct GitLayerClient {
    config: GitLayerConfig,
    channel: Channel,
}

impl GitLayerClient {
    /// Create a new GitLayer client
    pub async fn connect(config: GitLayerConfig) -> Result<Self, ShellError> {
        debug!("Connecting to GitLayer at {}", config.address);
        
        let channel = Channel::from_shared(config.address.clone())
            .map_err(|e| ShellError::GitLayerConnection(format!("Invalid address: {}", e)))?
            .connect_timeout(std::time::Duration::from_secs(config.connect_timeout))
            .connect()
            .await
            .map_err(|e| ShellError::GitLayerConnection(format!("Connection failed: {}", e)))?;
        
        Ok(Self { config, channel })
    }
    
    /// Check if GitLayer is healthy
    pub async fn health_check(&self) -> Result<bool, ShellError> {
        let mut client = HealthServiceClient::new(self.channel.clone());
        
        let response = client
            .check(HealthCheckRequest {})
            .await
            .map_err(|e| ShellError::GitLayerConnection(format!("Health check failed: {}", e)))?;
        
        let resp = response.into_inner();
        debug!("GitLayer version: {}, git2: {}", resp.version, resp.git2_version);
        
        Ok(resp.status == proto::health_check_response::ServingStatus::Serving as i32)
    }
    
    /// Execute SSH upload-pack (git clone/fetch/pull)
    pub async fn ssh_upload_pack(
        &self,
        repo_path: &str,
        env_vars: HashMap<String, String>,
        stdin_stream: impl Stream<Item = Vec<u8>> + Send + 'static,
    ) -> Result<impl Stream<Item = Result<SshPackOutput, ShellError>>, ShellError> {
        let mut client = SshServiceClient::new(self.channel.clone());
        
        let repository = Repository {
            storage_path: String::new(),
            relative_path: repo_path.to_string(),
        };
        
        let repo_clone = repository.clone();
        let env_clone = env_vars.clone();
        
        // Create request stream
        // Git protocol requires server to send refs advertisement first, so we must
        // send the initial request immediately to let GitLayer start the git process
        let request_stream = async_stream::stream! {
            // First message with repository info - send immediately to start git process
            yield SshPackRequest {
                repository: Some(repo_clone.clone()),
                stdin: Vec::new(),
                env_vars: env_clone.clone(),
            };
            
            // Forward stdin data from SSH client to GitLayer
            tokio::pin!(stdin_stream);
            
            while let Some(data) = stdin_stream.next().await {
                if !data.is_empty() {
                    yield SshPackRequest {
                        repository: None,
                        stdin: data,
                        env_vars: HashMap::new(),
                    };
                }
            }
        };
        
        let response = client
            .ssh_upload_pack(request_stream)
            .await
            .map_err(|e| ShellError::GitExecution(format!("upload-pack failed: {}", e)))?;
        
        let output_stream = response.into_inner().map(|result| {
            result
                .map(|r| SshPackOutput {
                    stdout: r.stdout,
                    stderr: r.stderr,
                    exit_code: r.exit_code,
                })
                .map_err(|e| ShellError::GitExecution(format!("Stream error: {}", e)))
        });
        
        Ok(output_stream)
    }
    
    /// Execute SSH receive-pack (git push)
    pub async fn ssh_receive_pack(
        &self,
        repo_path: &str,
        env_vars: HashMap<String, String>,
        stdin_stream: impl Stream<Item = Vec<u8>> + Send + 'static,
    ) -> Result<impl Stream<Item = Result<SshPackOutput, ShellError>>, ShellError> {
        let mut client = SshServiceClient::new(self.channel.clone());
        
        let repository = Repository {
            storage_path: String::new(),
            relative_path: repo_path.to_string(),
        };
        
        let repo_clone = repository.clone();
        let env_clone = env_vars.clone();
        
        // Create request stream
        // Git protocol requires server to send refs advertisement first, so we must
        // send the initial request immediately to let GitLayer start the git process
        let request_stream = async_stream::stream! {
            // First message with repository info - send immediately to start git process
            yield SshPackRequest {
                repository: Some(repo_clone.clone()),
                stdin: Vec::new(),
                env_vars: env_clone.clone(),
            };
            
            // Forward stdin data from SSH client to GitLayer
            tokio::pin!(stdin_stream);
            
            while let Some(data) = stdin_stream.next().await {
                if !data.is_empty() {
                    yield SshPackRequest {
                        repository: None,
                        stdin: data,
                        env_vars: HashMap::new(),
                    };
                }
            }
        };
        
        let response = client
            .ssh_receive_pack(request_stream)
            .await
            .map_err(|e| ShellError::GitExecution(format!("receive-pack failed: {}", e)))?;
        
        let output_stream = response.into_inner().map(|result| {
            result
                .map(|r| SshPackOutput {
                    stdout: r.stdout,
                    stderr: r.stderr,
                    exit_code: r.exit_code,
                })
                .map_err(|e| ShellError::GitExecution(format!("Stream error: {}", e)))
        });
        
        Ok(output_stream)
    }
    
    /// Execute SSH upload-archive (git archive --remote)
    pub async fn ssh_upload_archive(
        &self,
        repo_path: &str,
        env_vars: HashMap<String, String>,
        stdin_stream: impl Stream<Item = Vec<u8>> + Send + 'static,
    ) -> Result<impl Stream<Item = Result<SshPackOutput, ShellError>>, ShellError> {
        let mut client = SshServiceClient::new(self.channel.clone());
        
        let repository = Repository {
            storage_path: String::new(),
            relative_path: repo_path.to_string(),
        };
        
        let repo_clone = repository.clone();
        let env_clone = env_vars.clone();
        
        // Create request stream for upload-archive
        let request_stream = async_stream::stream! {
            // First message with repository info - send immediately
            yield SshPackRequest {
                repository: Some(repo_clone.clone()),
                stdin: Vec::new(),
                env_vars: env_clone.clone(),
            };
            
            // Forward stdin data from SSH client to GitLayer
            tokio::pin!(stdin_stream);
            
            while let Some(data) = stdin_stream.next().await {
                if !data.is_empty() {
                    yield SshPackRequest {
                        repository: None,
                        stdin: data,
                        env_vars: HashMap::new(),
                    };
                }
            }
        };
        
        let response = client
            .ssh_upload_archive(request_stream)
            .await
            .map_err(|e| ShellError::GitExecution(format!("upload-archive failed: {}", e)))?;
        
        let output_stream = response.into_inner().map(|result| {
            result
                .map(|r| SshPackOutput {
                    stdout: r.stdout,
                    stderr: r.stderr,
                    exit_code: r.exit_code,
                })
                .map_err(|e| ShellError::GitExecution(format!("Stream error: {}", e)))
        });
        
        Ok(output_stream)
    }
}

/// Output from SSH pack operations
#[derive(Debug)]
pub struct SshPackOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

/// Create a stdin stream from tokio stdin
pub fn stdin_stream() -> impl Stream<Item = Vec<u8>> + Send + 'static {
    use tokio::io::AsyncReadExt;
    
    async_stream::stream! {
        let mut stdin = tokio::io::stdin();
        let mut buffer = vec![0u8; 65536];
        
        loop {
            match stdin.read(&mut buffer).await {
                Ok(0) => break, // EOF
                Ok(n) => yield buffer[..n].to_vec(),
                Err(e) => {
                    error!("Failed to read stdin: {}", e);
                    break;
                }
            }
        }
    }
}
