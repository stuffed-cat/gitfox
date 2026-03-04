//! Configuration for GitFox Shell
//!
//! GitFox Shell 现在完全基于 gRPC 架构：
//! - Auth 通过 gRPC 与主应用通信（AUTH_GRPC_ADDRESS）
//! - Git 操作通过 gRPC 与 GitLayer 通信（GITLAYER_ADDRESS）

use std::env;
use std::path::PathBuf;

use crate::error::ShellError;

#[derive(Debug, Clone)]
pub struct Config {
    /// Secret token for internal gRPC authentication (GITFOX_API_SECRET)
    pub api_secret: String,

    /// Base path for Git repositories (用于 repo_path() 方法计算仓库完整路径)
    pub repos_path: String,

    /// Enable debug logging
    pub debug: bool,

    /// GitLayer gRPC server address (必需，用于处理所有 Git 操作)
    /// 环境变量: GITLAYER_ADDRESS
    pub gitlayer_address: Option<String>,

    /// Auth gRPC server address (必需，用于权限认证)
    /// 环境变量: AUTH_GRPC_ADDRESS 或 GITFOX_AUTH_GRPC_ADDRESS
    pub auth_grpc_address: Option<String>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> Result<Self, ShellError> {
        // Try to load .env file from standard locations
        Self::load_env_file();

        let api_secret = env::var("GITFOX_API_SECRET").map_err(|_| {
            ShellError::Config("GITFOX_API_SECRET environment variable is not set".to_string())
        })?;

        let repos_path = env::var("GITFOX_REPOS_PATH")
            .or_else(|_| env::var("GIT_REPOS_PATH"))
            .unwrap_or_else(|_| "/var/opt/gitfox/repos".to_string());

        let debug = env::var("GITFOX_DEBUG")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        let gitlayer_address = env::var("GITLAYER_ADDRESS").ok();

        let auth_grpc_address = env::var("AUTH_GRPC_ADDRESS")
            .or_else(|_| env::var("GITFOX_AUTH_GRPC_ADDRESS"))
            .ok();

        // 验证必需的 gRPC 地址
        if auth_grpc_address.is_none() {
            return Err(ShellError::Config(
                "AUTH_GRPC_ADDRESS or GITFOX_AUTH_GRPC_ADDRESS must be set. \
                 GitFox Shell requires gRPC Auth service for authentication.".to_string()
            ));
        }

        if gitlayer_address.is_none() {
            return Err(ShellError::Config(
                "GITLAYER_ADDRESS must be set. \
                 GitFox Shell requires GitLayer for all Git operations.".to_string()
            ));
        }

        Ok(Config {
            api_secret,
            repos_path,
            debug,
            gitlayer_address,
            auth_grpc_address,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_path() {
        let config = Config {
            api_secret: "secret".to_string(),
            repos_path: "/var/opt/gitfox/repos".to_string(),
            debug: false,
            gitlayer_address: Some("http://localhost:50052".to_string()),
            auth_grpc_address: Some("http://localhost:50051".to_string()),
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
}
