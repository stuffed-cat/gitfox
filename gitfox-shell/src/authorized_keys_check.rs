//! GitFox Shell - Authorized Keys Check
//!
//! This binary is called by sshd's AuthorizedKeysCommand to look up
//! SSH keys dynamically from the GitFox database.
//!
//! Usage in sshd_config:
//!   AuthorizedKeysCommand /usr/bin/gitfox-shell-authorized-keys-check %u %k %t
//!   AuthorizedKeysCommandUser git
//!
//! Arguments:
//!   %u - username (should be 'git')
//!   %k - base64-encoded key
//!   %t - key type (ssh-rsa, ssh-ed25519, etc.)

mod api;
mod auth_client;
mod config;
mod error;

use std::env;
use std::process::ExitCode;

use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::api::ApiClient;
use crate::auth_client::AuthClient;
use crate::config::Config;
use crate::error::ShellError;

#[tokio::main]
async fn main() -> ExitCode {
    // Setup minimal logging (to stderr)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::WARN)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .compact()
        .init();

    let args: Vec<String> = env::args().collect();

    // We need at least the key type and key
    if args.len() < 3 {
        error!("Usage: gitfox-shell-authorized-keys-check <username> <key> <key-type>");
        return ExitCode::FAILURE;
    }

    let username = &args[1];
    let key_base64 = &args[2];
    let key_type = args.get(3).map(|s| s.as_str()).unwrap_or("ssh-rsa");

    debug!(
        "Looking up key for user '{}', type '{}'",
        username, key_type
    );

    // Only allow the git user
    if username != "git" {
        debug!("Rejecting non-git user: {}", username);
        return ExitCode::SUCCESS; // Return empty (no keys)
    }

    match lookup_key(key_type, key_base64).await {
        Ok(Some(authorized_key)) => {
            // Output the authorized_keys line
            println!("{}", authorized_key);
            ExitCode::SUCCESS
        }
        Ok(None) => {
            // Key not found, return empty
            debug!("Key not found");
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("Error looking up key: {}", e);
            ExitCode::FAILURE
        }
    }
}

async fn lookup_key(key_type: &str, key_base64: &str) -> Result<Option<String>, ShellError> {
    let config = Config::load()?;
    
    // Construct the full public key
    let full_key = format!("{} {}", key_type, key_base64);

    // Compute fingerprint
    let fingerprint = crate::compute_fingerprint(&full_key)?;

    debug!("Looking up key with fingerprint: {}", fingerprint);

    // Look up the key via gRPC or HTTP
    let key_info = if config.use_grpc_auth {
        lookup_key_via_grpc(&config, &fingerprint).await?
    } else {
        lookup_key_via_http(&config, &fingerprint).await?
    };
    
    let key_info = match key_info {
        Some(info) => info,
        None => return Ok(None),
    };

    info!("Found key {} for user {}", key_info.id, key_info.username);

    // Build the authorized_keys line with restrictions
    let authorized_key = format!(
        r#"command="/usr/bin/gitfox-shell key-{}",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty {}"#,
        key_info.id,
        full_key
    );

    Ok(Some(authorized_key))
}

/// Key info returned from lookup
struct KeyInfo {
    id: i64,
    username: String,
}

/// Look up key via gRPC Auth service
async fn lookup_key_via_grpc(config: &Config, fingerprint: &str) -> Result<Option<KeyInfo>, ShellError> {
    let auth_addr = config.auth_grpc_address.as_ref()
        .ok_or_else(|| ShellError::Config("AUTH_GRPC_ADDRESS not configured".to_string()))?;
    
    let mut auth_client = AuthClient::connect(auth_addr, config.api_secret.clone())
        .await
        .map_err(|e| ShellError::Auth(format!("Failed to connect to Auth service: {}", e)))?;
    
    match auth_client.find_ssh_key(fingerprint).await {
        Ok(Some(info)) => Ok(Some(KeyInfo {
            id: info.id,
            username: info.username,
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(ShellError::Auth(format!("Key lookup failed: {}", e))),
    }
}

/// Look up key via HTTP API (legacy mode)
async fn lookup_key_via_http(config: &Config, fingerprint: &str) -> Result<Option<KeyInfo>, ShellError> {
    let api_client = ApiClient::new(config)?;

    // Look up the key in GitFox
    match api_client.find_key_by_fingerprint(fingerprint).await {
        Ok(info) => Ok(Some(KeyInfo {
            id: info.id,
            username: info.username,
        })),
        Err(ShellError::AccessDenied(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Compute SSH key fingerprint
fn compute_fingerprint(key: &str) -> Result<String, ShellError> {
    use std::process::{Command, Stdio};
    use std::io::Write;

    let mut child = Command::new("ssh-keygen")
        .args(["-l", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| ShellError::Config(format!("Failed to run ssh-keygen: {}", e)))?;

    {
        let stdin = child.stdin.as_mut().ok_or_else(|| {
            ShellError::Config("Failed to get stdin".to_string())
        })?;
        stdin.write_all(key.as_bytes()).map_err(|e| {
            ShellError::Config(format!("Failed to write to ssh-keygen: {}", e))
        })?;
    }

    let output = child.wait_with_output().map_err(|e| {
        ShellError::Config(format!("Failed to wait for ssh-keygen: {}", e))
    })?;

    if !output.status.success() {
        return Err(ShellError::Config("ssh-keygen failed".to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse fingerprint from: "256 SHA256:xxx user@host (ED25519)"
    stdout
        .split_whitespace()
        .nth(1)
        .map(|s| s.to_string())
        .ok_or_else(|| ShellError::Config("Failed to parse fingerprint".to_string()))
}
