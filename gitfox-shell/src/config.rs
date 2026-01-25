//! Configuration for GitFox Shell

use std::env;
use std::path::PathBuf;

use crate::error::ShellError;

#[derive(Debug, Clone)]
pub struct Config {
    /// Base URL for the GitFox API
    pub api_url: String,

    /// Secret token for internal API authentication
    pub api_secret: String,

    /// Base path for Git repositories
    pub repos_path: String,

    /// Path to git-upload-pack binary
    pub git_upload_pack_path: String,

    /// Path to git-receive-pack binary
    pub git_receive_pack_path: String,

    /// Connection timeout in seconds
    pub api_timeout_secs: u64,

    /// Enable debug logging
    pub debug: bool,
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> Result<Self, ShellError> {
        // Try to load .env file from standard locations
        Self::load_env_file();

        let api_url = env::var("GITFOX_API_URL")
            .or_else(|_| env::var("GITFOX_URL"))
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let api_secret = env::var("GITFOX_API_SECRET").map_err(|_| {
            ShellError::Config("GITFOX_API_SECRET environment variable is not set".to_string())
        })?;

        let repos_path = env::var("GITFOX_REPOS_PATH")
            .unwrap_or_else(|_| "/var/opt/gitfox/repos".to_string());

        let git_upload_pack_path = env::var("GITFOX_GIT_UPLOAD_PACK")
            .unwrap_or_else(|_| "git-upload-pack".to_string());

        let git_receive_pack_path = env::var("GITFOX_GIT_RECEIVE_PACK")
            .unwrap_or_else(|_| "git-receive-pack".to_string());

        let api_timeout_secs = env::var("GITFOX_API_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let debug = env::var("GITFOX_DEBUG")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        Ok(Config {
            api_url,
            api_secret,
            repos_path,
            git_upload_pack_path,
            git_receive_pack_path,
            api_timeout_secs,
            debug,
        })
    }

    /// Try to load .env file from standard locations
    fn load_env_file() {
        // Try current directory
        if dotenv::dotenv().is_ok() {
            return;
        }

        // Try /etc/gitfox/shell.env
        let etc_path = PathBuf::from("/etc/gitfox/shell.env");
        if etc_path.exists() {
            let _ = dotenv::from_path(&etc_path);
            return;
        }

        // Try home directory
        if let Some(home) = home::home_dir() {
            let home_path = home.join(".gitfox").join("shell.env");
            if home_path.exists() {
                let _ = dotenv::from_path(&home_path);
            }
        }
    }

    /// Get the full path to a repository
    pub fn repo_path(&self, relative_path: &str) -> String {
        let clean_path = relative_path
            .trim_start_matches('/')
            .trim_end_matches(".git");

        format!("{}/{}.git", self.repos_path, clean_path)
    }

    /// Get the API endpoint URL
    pub fn api_endpoint(&self, path: &str) -> String {
        format!("{}/api/internal{}", self.api_url.trim_end_matches('/'), path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_path() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            api_secret: "secret".to_string(),
            repos_path: "/var/opt/gitfox/repos".to_string(),
            git_upload_pack_path: "git-upload-pack".to_string(),
            git_receive_pack_path: "git-receive-pack".to_string(),
            api_timeout_secs: 30,
            debug: false,
        };

        assert_eq!(
            config.repo_path("owner/repo"),
            "/var/opt/gitfox/repos/owner/repo.git"
        );
        assert_eq!(
            config.repo_path("/owner/repo"),
            "/var/opt/gitfox/repos/owner/repo.git"
        );
        assert_eq!(
            config.repo_path("owner/repo.git"),
            "/var/opt/gitfox/repos/owner/repo.git"
        );
    }

    #[test]
    fn test_api_endpoint() {
        let config = Config {
            api_url: "http://localhost:8080".to_string(),
            api_secret: "secret".to_string(),
            repos_path: "/var/opt/gitfox/repos".to_string(),
            git_upload_pack_path: "git-upload-pack".to_string(),
            git_receive_pack_path: "git-receive-pack".to_string(),
            api_timeout_secs: 30,
            debug: false,
        };

        assert_eq!(
            config.api_endpoint("/allowed"),
            "http://localhost:8080/api/internal/allowed"
        );
    }
}
