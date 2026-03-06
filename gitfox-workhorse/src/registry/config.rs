//! Registry 配置

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Registry 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// 是否启用 Registry
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Registry 独立域名（如 registry.gitfox.studio）
    /// 为空则使用主域名的 /v2/ 和 /npm/ 路径
    pub domain: Option<String>,

    /// 是否启用 Docker Registry
    #[serde(default = "default_true")]
    pub docker_enabled: bool,

    /// 是否启用 npm Registry
    #[serde(default = "default_true")]
    pub npm_enabled: bool,

    /// 是否启用 Cargo Registry
    #[serde(default = "default_true")]
    pub cargo_enabled: bool,

    /// 存储路径
    #[serde(default = "default_storage_path")]
    pub storage_path: PathBuf,

    /// 最大包文件大小 (bytes)，默认 512MB
    #[serde(default = "default_max_size")]
    pub max_package_size: u64,

    /// Token 过期时间 (秒)，默认 1 小时
    #[serde(default = "default_token_expires")]
    pub token_expires: u64,
}

fn default_true() -> bool {
    true
}

fn default_storage_path() -> PathBuf {
    PathBuf::from("./packages")
}

fn default_max_size() -> u64 {
    512 * 1024 * 1024 // 512MB
}

fn default_token_expires() -> u64 {
    3600 // 1 小时
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            domain: None,
            docker_enabled: true,
            npm_enabled: true,
            cargo_enabled: true,
            storage_path: default_storage_path(),
            max_package_size: default_max_size(),
            token_expires: default_token_expires(),
        }
    }
}

impl RegistryConfig {
    /// 从环境变量加载
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("REGISTRY_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            domain: std::env::var("REGISTRY_DOMAIN").ok().filter(|s| !s.is_empty()),
            docker_enabled: std::env::var("REGISTRY_DOCKER_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            npm_enabled: std::env::var("REGISTRY_NPM_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            cargo_enabled: std::env::var("REGISTRY_CARGO_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            storage_path: std::env::var("REGISTRY_STORAGE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| default_storage_path()),
            max_package_size: std::env::var("REGISTRY_MAX_PACKAGE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_max_size),
            token_expires: std::env::var("REGISTRY_TOKEN_EXPIRES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(default_token_expires),
        }
    }

    /// 检查请求的 Host 是否是 Registry 域名
    pub fn is_registry_host(&self, host: &str) -> bool {
        if let Some(ref domain) = self.domain {
            // 移除端口号后比较
            let host_without_port = host.split(':').next().unwrap_or(host);
            let domain_without_port = domain.split(':').next().unwrap_or(domain);
            host_without_port.eq_ignore_ascii_case(domain_without_port)
        } else {
            false
        }
    }
}
