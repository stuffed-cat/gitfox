//! GitFox integrated SSH server
//!
//! This module provides a built-in SSH server for Git operations,
//! eliminating the need for a separate system sshd configuration.

mod handler;
mod keys;
mod session;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use log::{error, info};
use russh::server::{Config, Server};
use russh_keys::key::KeyPair;
use sqlx::PgPool;
use tokio::net::TcpListener;

use crate::config::Config as AppConfig;

pub use handler::GitSshHandler;
pub use keys::HostKeyManager;
pub use session::GitSession;

/// SSH Server for GitFox
pub struct SshServer {
    config: Arc<AppConfig>,
    pool: Arc<PgPool>,
    host_keys: Vec<KeyPair>,
    bind_addr: SocketAddr,
}

impl SshServer {
    /// Create a new SSH server instance
    pub async fn new(config: Arc<AppConfig>, pool: Arc<PgPool>) -> Result<Self> {
        let bind_addr: SocketAddr = format!("{}:{}", config.ssh_host, config.ssh_port)
            .parse()
            .expect("Invalid SSH bind address");

        // Load or generate host keys
        let host_key_manager = HostKeyManager::new(&config.ssh_host_key_path)?;
        let host_keys = host_key_manager.get_keys().await?;

        Ok(Self {
            config,
            pool,
            host_keys,
            bind_addr,
        })
    }

    /// Start the SSH server
    pub async fn run(self) -> Result<()> {
        info!("Starting GitFox SSH server on {}", self.bind_addr);

        let mut server_config = Config::default();
        server_config.keys = self.host_keys;
        server_config.auth_rejection_time = std::time::Duration::from_secs(3);
        server_config.auth_rejection_time_initial = Some(std::time::Duration::from_secs(0));

        let server_config = Arc::new(server_config);

        let listener = TcpListener::bind(self.bind_addr).await?;
        info!("GitFox SSH server listening on {}", self.bind_addr);

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    let config = server_config.clone();
                    let app_config = self.config.clone();
                    let pool = self.pool.clone();

                    tokio::spawn(async move {
                        let handler = GitSshHandler::new(app_config, pool, addr);
                        
                        if let Err(e) = russh::server::run_stream(config, socket, handler).await {
                            error!("SSH session error from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept SSH connection: {}", e);
                }
            }
        }
    }
}

/// Start the SSH server in a background task
pub fn start_ssh_server(config: Arc<AppConfig>, pool: Arc<PgPool>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        match SshServer::new(config, pool).await {
            Ok(server) => {
                if let Err(e) = server.run().await {
                    error!("SSH server error: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to initialize SSH server: {}", e);
            }
        }
    })
}
