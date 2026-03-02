//! GitLayer gRPC client for workhorse
//!
//! This module provides a client to communicate with GitLayer
//! for Git HTTP protocol operations.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tracing::{debug, error, info};

// Include generated proto code
pub mod proto {
    tonic::include_proto!("gitlayer");
}

use proto::{
    smart_http_service_client::SmartHttpServiceClient,
    health_service_client::HealthServiceClient,
    Repository, InfoRefsRequest, UploadPackRequest, ReceivePackRequest,
    HealthCheckRequest,
};

/// GitLayer client configuration
#[derive(Debug, Clone)]
pub struct GitLayerConfig {
    /// GitLayer server address (e.g., "http://127.0.0.1:9000")
    pub address: String,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Request timeout in seconds
    pub request_timeout: u64,
}

impl Default for GitLayerConfig {
    fn default() -> Self {
        Self {
            address: "http://127.0.0.1:9000".to_string(),
            connect_timeout: 5,
            request_timeout: 600, // Git operations can be slow
        }
    }
}

/// GitLayer client with connection pooling
#[derive(Clone)]
pub struct GitLayerClient {
    config: GitLayerConfig,
    channel: Arc<RwLock<Option<Channel>>>,
}

impl GitLayerClient {
    /// Create a new GitLayer client
    pub fn new(config: GitLayerConfig) -> Self {
        Self {
            config,
            channel: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Get or create a channel to GitLayer
    async fn get_channel(&self) -> Result<Channel, String> {
        // Check if we have a valid channel
        {
            let read_guard = self.channel.read().await;
            if let Some(ref ch) = *read_guard {
                return Ok(ch.clone());
            }
        }
        
        // Create new channel
        let mut write_guard = self.channel.write().await;
        
        // Double-check after acquiring write lock
        if let Some(ref ch) = *write_guard {
            return Ok(ch.clone());
        }
        
        debug!("Connecting to GitLayer at {}", self.config.address);
        
        let channel = Channel::from_shared(self.config.address.clone())
            .map_err(|e| format!("Invalid address: {}", e))?
            .connect_timeout(Duration::from_secs(self.config.connect_timeout))
            .timeout(Duration::from_secs(self.config.request_timeout))
            .connect()
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;
        
        *write_guard = Some(channel.clone());
        Ok(channel)
    }
    
    /// Check if GitLayer is healthy
    pub async fn health_check(&self) -> Result<bool, String> {
        let channel = self.get_channel().await?;
        let mut client = HealthServiceClient::new(channel);
        
        let response = client
            .check(HealthCheckRequest {})
            .await
            .map_err(|e| format!("Health check failed: {}", e))?;
        
        let resp = response.into_inner();
        debug!("GitLayer version: {}, git2: {}", resp.version, resp.git2_version);
        
        Ok(resp.status == proto::health_check_response::ServingStatus::Serving as i32)
    }
    
    /// Get refs info (git-upload-pack --advertise-refs or git-receive-pack --advertise-refs)
    pub async fn info_refs(
        &self,
        repo_path: &str,
        service: &str,
    ) -> Result<Vec<u8>, String> {
        let channel = self.get_channel().await?;
        let mut client = SmartHttpServiceClient::new(channel);
        
        let repository = Repository {
            storage_path: String::new(),
            relative_path: repo_path.to_string(),
        };
        
        let response = client
            .info_refs(InfoRefsRequest {
                repository: Some(repository),
                service: service.to_string(),
            })
            .await
            .map_err(|e| format!("info_refs failed: {}", e))?;
        
        Ok(response.into_inner().data)
    }
    
    /// Handle git-upload-pack (fetch/clone)
    pub async fn upload_pack(
        &self,
        repo_path: &str,
        data: Vec<u8>,
    ) -> Result<impl tokio_stream::Stream<Item = Result<Vec<u8>, String>>, String> {
        let channel = self.get_channel().await?;
        let mut client = SmartHttpServiceClient::new(channel);
        
        let repository = Repository {
            storage_path: String::new(),
            relative_path: repo_path.to_string(),
        };
        
        // Create request stream with single message
        let request_stream = tokio_stream::once(UploadPackRequest {
            repository: Some(repository),
            data,
        });
        
        let response = client
            .upload_pack(request_stream)
            .await
            .map_err(|e| format!("upload_pack failed: {}", e))?;
        
        let output_stream = response.into_inner().map(|result| {
            result
                .map(|r| r.data)
                .map_err(|e| format!("Stream error: {}", e))
        });
        
        Ok(output_stream)
    }
    
    /// Handle git-receive-pack (push)
    pub async fn receive_pack(
        &self,
        repo_path: &str,
        data: Vec<u8>,
        username: &str,
        user_id: i64,
    ) -> Result<impl tokio_stream::Stream<Item = Result<Vec<u8>, String>>, String> {
        let channel = self.get_channel().await?;
        let mut client = SmartHttpServiceClient::new(channel);
        
        let repository = Repository {
            storage_path: String::new(),
            relative_path: repo_path.to_string(),
        };
        
        // Create request stream with single message
        let request_stream = tokio_stream::once(ReceivePackRequest {
            repository: Some(repository),
            data,
            push_options: vec![],
            user_id,
            username: username.to_string(),
        });
        
        let response = client
            .receive_pack(request_stream)
            .await
            .map_err(|e| format!("receive_pack failed: {}", e))?;
        
        let output_stream = response.into_inner().map(|result| {
            result
                .map(|r| r.data)
                .map_err(|e| format!("Stream error: {}", e))
        });
        
        Ok(output_stream)
    }
}
