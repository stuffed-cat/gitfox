//! GitFox Shell - SSH Server and Git access management
//!
//! This is the SSH server component of GitFox, similar to gitlab-shell.
//! It provides:
//! - Built-in SSH server (no system sshd required)
//! - Git over SSH protocol support
//! - Integration with GitLayer for Git operations
//!
//! Usage:
//!   gitfox-shell server              # Start SSH server (main mode)
//!   gitfox-shell <key-id>            # Legacy mode (called by gitfox main app)

mod api;
mod auth_client;
mod command;
mod config;
mod error;
mod git;
mod gitlayer_client;
mod ssh_server;

use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::api::{AccessInfo, ApiClient};
use crate::auth_client::AuthClient;
use crate::command::GitCommand;
use crate::config::Config;
use crate::error::ShellError;
use crate::ssh_server::{SshServer, SshServerConfig};

#[derive(Parser, Debug)]
#[command(name = "gitfox-shell")]
#[command(author = "GitFox Team")]
#[command(version = "0.1.0")]
#[command(about = "GitFox Shell - SSH server for Git repositories")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the SSH server (main mode)
    Server {
        /// SSH listen address
        #[arg(long, env = "SSH_LISTEN_ADDR", default_value = "0.0.0.0:2222")]
        listen: SocketAddr,

        /// SSH host key path
        #[arg(long, env = "SSH_HOST_KEY_PATH", default_value = "/var/lib/gitfox/ssh_host_key")]
        host_key: PathBuf,
    },

    /// Legacy mode: handle single SSH session (called by gitfox main app with key_id argument)
    #[command(hide = true)]
    Session {
        /// The key ID used for authentication
        key_id: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    // Load .env file FIRST, before parsing args
    // This allows clap to read from environment variables
    dotenv::dotenv().ok();

    let args = Args::parse();

    // Setup logging
    let log_level = if args.debug { Level::DEBUG } else { Level::INFO };

    FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .compact()
        .init();

    // Determine mode based on command or arguments
    let result = match args.command {
        Some(Commands::Server { listen, host_key }) => {
            run_server(listen, host_key).await
        }
        Some(Commands::Session { key_id }) => {
            run_session(&key_id).await
        }
        None => {
            // Check if we have a key_id as first arg (legacy sshd mode)
            // This maintains compatibility with existing sshd configurations
            let legacy_args: Vec<String> = env::args().collect();
            if legacy_args.len() >= 2 && !legacy_args[1].starts_with('-') {
                let key_id = &legacy_args[1];
                run_session(key_id).await
            } else {
                // Default to server mode
                let listen: SocketAddr = env::var("SSH_LISTEN_ADDR")
                    .unwrap_or_else(|_| "0.0.0.0:2222".to_string())
                    .parse()
                    .expect("Invalid SSH_LISTEN_ADDR");
                let host_key: PathBuf = env::var("SSH_HOST_KEY_PATH")
                    .unwrap_or_else(|_| "/var/lib/gitfox/ssh_host_key".to_string())
                    .into();
                run_server(listen, host_key).await
            }
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("GitFox Shell error: {}", e);
            eprintln!("GitFox: {}", e.user_message());
            ExitCode::FAILURE
        }
    }
}

/// Run the SSH server
async fn run_server(listen: SocketAddr, host_key: PathBuf) -> Result<(), ShellError> {
    info!("GitFox Shell starting in server mode");
    info!("Listening on {}", listen);

    let config = Config::load()?;
    debug!("Configuration loaded: {:?}", config);

    let server_config = SshServerConfig {
        listen_addr: listen,
        host_key_path: host_key,
        app_config: config,
    };

    let server = SshServer::new(server_config);
    server.run().await
}

/// Run a single SSH session (legacy sshd mode)
async fn run_session(key_id: &str) -> Result<(), ShellError> {
    // Load configuration
    let config = Config::load()?;
    debug!("Configuration loaded: {:?}", config);

    // Get the original SSH command
    let ssh_command = env::var("SSH_ORIGINAL_COMMAND").ok();
    debug!("SSH_ORIGINAL_COMMAND: {:?}", ssh_command);

    // Parse the git command
    let git_command = match &ssh_command {
        Some(cmd) => GitCommand::parse(cmd)?,
        None => {
            // Interactive shell access is not allowed
            info!("Interactive shell access denied for key_id: {}", key_id);
            println!("Welcome to GitFox, {}!", key_id);
            println!();
            println!("GitFox does not provide shell access.");
            println!("Please use Git over SSH for repository operations.");
            println!();
            println!("Example:");
            println!("  git clone git@gitfox.example.com:owner/repo.git");
            println!();
            return Ok(());
        }
    };

    info!(
        "Processing git command: {:?} for repo: {}",
        git_command.action, git_command.repo_path
    );

    // Verify access via gRPC or HTTP
    let (access_info, gitlayer_addr) = if config.use_grpc_auth {
        check_access_via_grpc(&config, key_id, &git_command).await?
    } else {
        check_access_via_http(&config, key_id, &git_command).await?
    };

    info!(
        "Access granted for user {} on repo {} (write: {})",
        access_info.user_id, git_command.repo_path, access_info.can_write
    );

    // Update config with GitLayer address from auth response if available
    let mut config = config;
    if let Some(addr) = gitlayer_addr {
        config.gitlayer_address = Some(addr);
        config.use_gitlayer = true;
    }

    // Build the full repository path
    let full_repo_path = config.repo_path(&git_command.repo_path);
    debug!("Full repository path: {}", full_repo_path);

    // Execute the git command
    git_command.execute(&full_repo_path, &access_info, &config).await?;

    Ok(())
}

/// Check access via gRPC Auth service (GitLab-style architecture)
async fn check_access_via_grpc(
    config: &Config,
    key_id: &str,
    git_command: &GitCommand,
) -> Result<(AccessInfo, Option<String>), ShellError> {
    let auth_addr = config.auth_grpc_address.as_ref()
        .ok_or_else(|| ShellError::Config("AUTH_GRPC_ADDRESS not configured".to_string()))?;
    
    debug!("Using gRPC auth at {}", auth_addr);
    
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
    
    let access_info = AccessInfo {
        user_id: response.user_id,
        username: response.username,
        can_write: response.can_write,
        project_id: if response.project_id > 0 { Some(response.project_id) } else { None },
        repository_id: None,
        lfs_token: None,
        base_url: None,
        repository_status: if response.repository_status.is_empty() { None } else { Some(response.repository_status) },
    };
    
    let gitlayer_addr = if response.gitlayer_address.is_empty() {
        None
    } else {
        Some(response.gitlayer_address)
    };
    
    Ok((access_info, gitlayer_addr))
}

/// Check access via HTTP API (legacy mode)
async fn check_access_via_http(
    config: &Config,
    key_id: &str,
    git_command: &GitCommand,
) -> Result<(AccessInfo, Option<String>), ShellError> {
    let api_client = ApiClient::new(config)?;
    let access_info = api_client
        .check_access(key_id, &git_command.repo_path, git_command.action.requires_write())
        .await?;
    
    Ok((access_info, None))
}
