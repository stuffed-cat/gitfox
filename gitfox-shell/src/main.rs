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
mod command;
mod config;
mod error;
mod git;

use std::env;
use std::process::ExitCode;

use clap::Parser;
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::api::ApiClient;
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

    // Setup logging
    let log_level = if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
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

    // Create API client
    let api_client = ApiClient::new(&config)?;

    // Verify access
    let access_info = api_client
        .check_access(key_id, &git_command.repo_path, git_command.action.requires_write())
        .await?;

    info!(
        "Access granted for user {} on repo {} (write: {})",
        access_info.user_id, git_command.repo_path, access_info.can_write
    );

    // Build the full repository path
    let full_repo_path = config.repo_path(&git_command.repo_path);
    debug!("Full repository path: {}", full_repo_path);

    // Execute the git command
    git_command.execute(&full_repo_path, &access_info).await?;

    Ok(())
}
