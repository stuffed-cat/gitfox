//! GitFox Shell - SSH access and repository management
//!
//! This is the main entry point for SSH Git operations.
//! It handles authentication, authorization, and executes git commands.
//!
//! Usage:
//!   gitfox-shell <key-id>
//!
//! The SSH_ORIGINAL_COMMAND environment variable contains the git command.

mod api;
mod auth_client;
mod command;
mod config;
mod error;
mod git;
mod gitlayer_client;

use std::env;
use std::process::ExitCode;

use clap::Parser;
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::api::{AccessInfo, ApiClient};
use crate::auth_client::AuthClient;
use crate::command::GitCommand;
use crate::config::Config;
use crate::error::ShellError;

#[derive(Parser, Debug)]
#[command(name = "gitfox-shell")]
#[command(author = "GitFox Team")]
#[command(version = "0.1.0")]
#[command(about = "GitFox Shell - SSH access for Git repositories")]
struct Args {
    /// The key ID used for authentication
    #[arg(required = true)]
    key_id: String,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Parse arguments
    let args = Args::parse();

    // Setup logging - only show errors in normal mode, debug shows all
    let log_level = if args.debug {
        Level::DEBUG
    } else {
        Level::ERROR
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .compact()
        .init();

    debug!("GitFox Shell starting with key_id: {}", args.key_id);

    // Run the shell
    match run_shell(&args.key_id).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("GitFox Shell error: {}", e);
            eprintln!("GitFox: {}", e.user_message());
            ExitCode::FAILURE
        }
    }
}

async fn run_shell(key_id: &str) -> Result<(), ShellError> {
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
