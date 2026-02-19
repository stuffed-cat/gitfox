//! Git command parsing and execution

use std::collections::HashMap;
use std::process::Stdio;

use regex::Regex;
use tokio::process::Command;
use tracing::{debug, error, info};

use crate::api::AccessInfo;
use crate::error::ShellError;

/// Supported Git actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitAction {
    /// git-upload-pack (clone, fetch, pull)
    UploadPack,
    /// git-receive-pack (push)
    ReceivePack,
    /// git-upload-archive (git archive --remote)
    UploadArchive,
    /// git-lfs-authenticate
    LfsAuthenticate,
}

impl GitAction {
    /// Check if this action requires write access
    pub fn requires_write(&self) -> bool {
        matches!(self, GitAction::ReceivePack)
    }

    /// Get the git binary name for this action
    pub fn binary_name(&self) -> &'static str {
        match self {
            GitAction::UploadPack => "git-upload-pack",
            GitAction::ReceivePack => "git-receive-pack",
            GitAction::UploadArchive => "git-upload-archive",
            GitAction::LfsAuthenticate => "git-lfs-authenticate",
        }
    }
}

/// Parsed Git command
#[derive(Debug)]
pub struct GitCommand {
    /// The git action to perform
    pub action: GitAction,
    /// The repository path (e.g., "owner/repo")
    pub repo_path: String,
    /// Additional arguments (for LFS)
    pub extra_args: Vec<String>,
}

impl GitCommand {
    /// Parse an SSH command into a GitCommand
    pub fn parse(command: &str) -> Result<Self, ShellError> {
        let command = command.trim();

        // Check for empty command
        if command.is_empty() {
            return Err(ShellError::InvalidCommand("Empty command".to_string()));
        }

        // Parse the command
        // Format: git-upload-pack 'owner/repo.git'
        //         git-receive-pack 'owner/repo.git'
        //         git upload-pack 'owner/repo.git'
        //         git-lfs-authenticate owner/repo.git download

        // Handle "git upload-pack" vs "git-upload-pack"
        let normalized = command.replace("git upload-pack", "git-upload-pack")
            .replace("git receive-pack", "git-receive-pack")
            .replace("git upload-archive", "git-upload-archive");

        let parts: Vec<&str> = normalized.split_whitespace().collect();

        if parts.is_empty() {
            return Err(ShellError::InvalidCommand("Empty command".to_string()));
        }

        let action = match parts[0] {
            "git-upload-pack" => GitAction::UploadPack,
            "git-receive-pack" => GitAction::ReceivePack,
            "git-upload-archive" => GitAction::UploadArchive,
            "git-lfs-authenticate" => GitAction::LfsAuthenticate,
            cmd => {
                return Err(ShellError::InvalidCommand(format!(
                    "Unknown git command: {}",
                    cmd
                )));
            }
        };

        // Get the repository path
        if parts.len() < 2 {
            return Err(ShellError::InvalidCommand(
                "Missing repository path".to_string(),
            ));
        }

        // Extract repo path, removing quotes if present
        let repo_path = Self::extract_repo_path(parts[1])?;

        // Extract extra args (for LFS)
        let extra_args: Vec<String> = parts
            .iter()
            .skip(2)
            .map(|s| s.to_string())
            .collect();

        debug!(
            "Parsed command: action={:?}, repo={}, extra_args={:?}",
            action, repo_path, extra_args
        );

        Ok(GitCommand {
            action,
            repo_path,
            extra_args,
        })
    }

    /// Extract and validate the repository path
    fn extract_repo_path(raw_path: &str) -> Result<String, ShellError> {
        // Remove surrounding quotes
        let path = raw_path
            .trim_matches('\'')
            .trim_matches('"')
            .trim_start_matches('/')
            .trim_end_matches(".git");

        // Validate the path format: should be "owner/repo" or "namespace/subgroup/repo"
        let path_regex = Regex::new(r"^[a-zA-Z0-9_.-]+(/[a-zA-Z0-9_.-]+)+$").unwrap();

        if !path_regex.is_match(path) {
            return Err(ShellError::InvalidCommand(format!(
                "Invalid repository path format: {}",
                path
            )));
        }

        // Check for path traversal attempts
        if path.contains("..") || path.contains("//") {
            return Err(ShellError::InvalidCommand(
                "Invalid repository path: path traversal detected".to_string(),
            ));
        }

        Ok(path.to_string())
    }

    /// Execute the git command
    pub async fn execute(&self, repo_path: &str, access_info: &AccessInfo) -> Result<(), ShellError> {
        info!("Executing {:?} on {}", self.action, repo_path);

        // Check if the repository exists
        let repo_metadata = std::fs::metadata(repo_path);
        if repo_metadata.is_err() || !repo_metadata.unwrap().is_dir() {
            return Err(ShellError::RepoNotFound(self.repo_path.clone()));
        }

        match self.action {
            GitAction::UploadPack | GitAction::ReceivePack | GitAction::UploadArchive => {
                self.execute_git_command(repo_path, access_info).await
            }
            GitAction::LfsAuthenticate => {
                self.execute_lfs_authenticate(repo_path, access_info).await
            }
        }
    }

    /// Execute a standard git command
    async fn execute_git_command(
        &self,
        repo_path: &str,
        access_info: &AccessInfo,
    ) -> Result<(), ShellError> {
        let binary = self.action.binary_name();

        // For receive-pack, we need to capture ref changes and trigger CI/CD
        let capture_refs = self.action == GitAction::ReceivePack;
        let mut old_refs = std::collections::HashMap::new();

        if capture_refs {
            // Capture current refs before push
            if let Ok(refs) = Self::get_current_refs(repo_path).await {
                old_refs = refs;
            }
        }

        // Build environment variables for git hooks
        let mut env_vars = vec![
            ("GL_ID".to_string(), format!("user-{}", access_info.user_id)),
            ("GL_USERNAME".to_string(), access_info.username.clone()),
            ("GL_REPOSITORY".to_string(), self.repo_path.clone()),
            ("GL_PROTOCOL".to_string(), "ssh".to_string()),
            ("GITFOX_USER_ID".to_string(), access_info.user_id.to_string()),
            ("GITFOX_USERNAME".to_string(), access_info.username.clone()),
            ("GITFOX_REPO_PATH".to_string(), self.repo_path.clone()),
        ];

        // Add project info if available
        if let Some(project_id) = access_info.project_id {
            env_vars.push(("GL_PROJECT_PATH".to_string(), self.repo_path.clone()));
            env_vars.push(("GITFOX_PROJECT_ID".to_string(), project_id.to_string()));
        }

        debug!(
            "Executing: {} {} with env: {:?}",
            binary, repo_path, env_vars
        );

        // Execute the git command, connecting stdin/stdout/stderr
        let mut cmd = Command::new(binary);
        cmd.arg(repo_path)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        // Set environment variables
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let status = cmd
            .status()
            .await
            .map_err(|e| ShellError::GitExecution(format!("Failed to execute {}: {}", binary, e)))?;

        if status.success() {
            info!("Git command completed successfully");
            
            // For receive-pack, trigger post-receive processing
            if capture_refs {
                if let Err(e) = Self::trigger_post_receive(
                    repo_path,
                    &old_refs,
                    access_info,
                    &self.repo_path
                ).await {
                    error!("Failed to trigger post-receive: {}", e);
                    // Don't fail the push if post-receive trigger fails
                }
            }
            
            Ok(())
        } else {
            let exit_code = status.code().unwrap_or(-1);
            error!("Git command failed with exit code: {}", exit_code);
            Err(ShellError::GitExecution(format!(
                "Git command exited with code {}",
                exit_code
            )))
        }
    }

    /// Get current refs (before push)
    async fn get_current_refs(repo_path: &str) -> Result<std::collections::HashMap<String, String>, ShellError> {
        use std::collections::HashMap;
        
        let output = Command::new("git")
            .args(["show-ref", "--head", "--dereference"])
            .current_dir(repo_path)
            .output()
            .await
            .map_err(|e| ShellError::GitExecution(format!("Failed to get refs: {}", e)))?;

        let mut refs = HashMap::new();
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 2 {
                    refs.insert(parts[1].to_string(), parts[0].to_string());
                }
            }
        }
        Ok(refs)
    }

    /// Trigger post-receive hook via API
    async fn trigger_post_receive(
        repo_path: &str,
        old_refs: &std::collections::HashMap<String, String>,
        access_info: &AccessInfo,
        repository: &str,
    ) -> Result<(), ShellError> {
        use serde_json::json;
        
        // Get current refs (after push)
        let new_refs = Self::get_current_refs(repo_path).await?;
        
        // Detect changes
        let mut changes = Vec::new();
        
        // Find new or updated refs
        for (ref_name, new_sha) in &new_refs {
            if let Some(old_sha) = old_refs.get(ref_name) {
                if old_sha != new_sha {
                    // Updated ref
                    changes.push(json!({
                        "old_sha": old_sha,
                        "new_sha": new_sha,
                        "ref": ref_name
                    }));
                }
            } else {
                // New ref
                changes.push(json!({
                    "old_sha": "0000000000000000000000000000000000000000",
                    "new_sha": new_sha,
                    "ref": ref_name
                }));
            }
        }
        
        // Find deleted refs
        for (ref_name, old_sha) in old_refs {
            if !new_refs.contains_key(ref_name) {
                changes.push(json!({
                    "old_sha": old_sha,
                    "new_sha": "0000000000000000000000000000000000000000",
                    "ref": ref_name
                }));
            }
        }
        
        if changes.is_empty() {
            debug!("No ref changes detected");
            return Ok(());
        }
        
        info!("Detected {} ref changes, triggering post-receive", changes.len());
        
        // Call internal API
        let base_url = access_info.base_url.as_deref().unwrap_or("http://localhost:8080");
        let api_token = std::env::var("GITFOX_SHELL_SECRET")
            .unwrap_or_else(|_| "your-shell-secret".to_string());
        
        let payload = json!({
            "user_id": access_info.user_id.to_string(),
            "repository": repository,
            "project_id": access_info.project_id.map(|id| id.to_string()),
            "changes": changes
        });
        
        let client = reqwest::Client::new();
        let url = format!("{}/api/internal/post-receive", base_url.trim_end_matches('/'));
        
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-GitFox-Shell-Token", api_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ShellError::GitExecution(format!("Failed to call post-receive API: {}", e)))?;
        
        if response.status().is_success() {
            info!("Post-receive API call successful");
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Post-receive API call failed: {} - {}", status, body);
            Err(ShellError::GitExecution(format!(
                "Post-receive API returned {}: {}",
                status, body
            )))
        }
    }

    /// Execute git-lfs-authenticate
    async fn execute_lfs_authenticate(
        &self,
        _repo_path: &str,
        access_info: &AccessInfo,
    ) -> Result<(), ShellError> {
        // Get the operation type (download or upload)
        let operation = self.extra_args.first().map(|s| s.as_str()).unwrap_or("download");

        // Check permissions
        if operation == "upload" && !access_info.can_write {
            return Err(ShellError::AccessDenied(
                "Write access required for LFS upload".to_string(),
            ));
        }

        // Generate LFS authentication response
        let lfs_auth = serde_json::json!({
            "header": {
                "Authorization": format!("Bearer {}", access_info.lfs_token.as_deref().unwrap_or("")),
            },
            "href": format!("{}/{}.git/info/lfs", access_info.base_url.as_deref().unwrap_or(""), self.repo_path),
            "expires_in": 1800,
        });

        println!("{}", serde_json::to_string(&lfs_auth).unwrap());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_upload_pack() {
        let cmd = GitCommand::parse("git-upload-pack 'owner/repo.git'").unwrap();
        assert_eq!(cmd.action, GitAction::UploadPack);
        assert_eq!(cmd.repo_path, "owner/repo");
    }

    #[test]
    fn test_parse_receive_pack() {
        let cmd = GitCommand::parse("git-receive-pack '/owner/repo.git'").unwrap();
        assert_eq!(cmd.action, GitAction::ReceivePack);
        assert_eq!(cmd.repo_path, "owner/repo");
    }

    #[test]
    fn test_parse_git_space_format() {
        let cmd = GitCommand::parse("git upload-pack 'owner/repo'").unwrap();
        assert_eq!(cmd.action, GitAction::UploadPack);
        assert_eq!(cmd.repo_path, "owner/repo");
    }

    #[test]
    fn test_parse_nested_path() {
        let cmd = GitCommand::parse("git-upload-pack 'group/subgroup/repo.git'").unwrap();
        assert_eq!(cmd.repo_path, "group/subgroup/repo");
    }

    #[test]
    fn test_parse_invalid_command() {
        let result = GitCommand::parse("rm -rf /");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_path_traversal() {
        let result = GitCommand::parse("git-upload-pack '../../../etc/passwd'");
        assert!(result.is_err());
    }

    #[test]
    fn test_action_requires_write() {
        assert!(!GitAction::UploadPack.requires_write());
        assert!(GitAction::ReceivePack.requires_write());
        assert!(!GitAction::UploadArchive.requires_write());
    }
}
