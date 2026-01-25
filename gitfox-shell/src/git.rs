//! Git utilities for GitFox Shell

use std::process::Command;
use tracing::{debug, warn};

/// Get Git version
pub fn git_version() -> Option<String> {
    let output = Command::new("git").arg("--version").output().ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

/// Check if a path is a valid Git repository
pub fn is_git_repository(path: &str) -> bool {
    let git_dir = std::path::Path::new(path);

    // Check if it's a bare repository
    if git_dir.join("HEAD").exists() && git_dir.join("objects").exists() {
        return true;
    }

    // Check if it has a .git directory
    if git_dir.join(".git").exists() {
        return true;
    }

    false
}

/// Initialize a bare Git repository
pub fn init_bare_repository(path: &str) -> std::io::Result<()> {
    debug!("Initializing bare repository at {}", path);

    // Create the directory
    std::fs::create_dir_all(path)?;

    // Initialize as bare repository
    let output = Command::new("git")
        .args(["init", "--bare", path])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Failed to initialize repository: {}", stderr);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("git init failed: {}", stderr),
        ));
    }

    Ok(())
}

/// Compute SSH key fingerprint
pub fn compute_key_fingerprint(key: &str) -> Option<String> {
    use std::process::Stdio;

    // Use ssh-keygen to compute the fingerprint
    let mut child = Command::new("ssh-keygen")
        .args(["-l", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut()?;
        stdin.write_all(key.as_bytes()).ok()?;
    }

    let output = child.wait_with_output().ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Parse the fingerprint from output like: "256 SHA256:xxx user@host (ED25519)"
        stdout
            .split_whitespace()
            .nth(1)
            .map(|s| s.to_string())
    } else {
        None
    }
}

/// Parse SSH authorized_keys options
pub fn parse_authorized_keys_options(line: &str) -> Option<(String, String)> {
    // Format: command="gitfox-shell key-123",no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty ssh-rsa AAAA...
    
    let parts: Vec<&str> = line.splitn(2, "ssh-").collect();
    if parts.len() != 2 {
        return None;
    }

    let options = parts[0].trim();
    let key = format!("ssh-{}", parts[1]);

    Some((options.to_string(), key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authorized_keys() {
        let line = r#"command="gitfox-shell key-123",no-port-forwarding ssh-rsa AAAAB..."#;
        let (options, key) = parse_authorized_keys_options(line).unwrap();
        
        assert!(options.contains("gitfox-shell key-123"));
        assert!(key.starts_with("ssh-rsa"));
    }
}
