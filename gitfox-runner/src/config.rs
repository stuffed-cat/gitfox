use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::error::{Result, RunnerError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    pub server_url: String,
    pub token: String,
    pub name: String,
    pub tags: Vec<String>,
    pub executor: String,
    
    /// Work directory for job builds (default: ./builds)
    #[serde(default = "default_builds_dir")]
    pub builds_dir: PathBuf,
    
    /// Maximum work directory size in MB (default: 10GB)
    #[serde(default = "default_max_work_dir_size")]
    pub max_work_dir_size_mb: u64,
    
    /// Maximum concurrent jobs (default: 1)
    #[serde(default = "default_concurrent_jobs")]
    pub concurrent_jobs: usize,
    
    /// Default container image for docker/kubernetes executor
    /// 注册时如果选择 docker 或 kubernetes 执行器则必须提供
    #[serde(default = "default_image", alias = "default_docker_image")]
    pub default_image: String,

    /// Kubernetes namespace (only used by kubernetes executor, default: "default")
    #[serde(default = "default_kubernetes_namespace")]
    pub kubernetes_namespace: String,

    /// Path to kubeconfig file (only used by kubernetes executor, e.g. ~/.kube/config)
    /// If None, uses the cluster's in-cluster config or KUBECONFIG env var
    #[serde(default)]
    pub kubernetes_config: Option<String>,
    
    /// Clean builds directory after job completion
    #[serde(default = "default_clean_builds")]
    pub clean_builds: bool,
    
    /// Enable security isolation (STRONGLY RECOMMENDED, default: true)
    #[serde(default = "default_security_enabled")]
    pub security_enabled: bool,
    
    /// Script-level timeout in seconds (per command, default: 3600)
    #[serde(default = "default_script_timeout")]
    pub script_timeout_seconds: u64,
    
    /// Network access mode: "full", "restricted", "none" (default: "full")
    #[serde(default = "default_network_mode")]
    pub network_mode: String,
}

fn default_builds_dir() -> PathBuf {
    PathBuf::from("./builds")
}

fn default_max_work_dir_size() -> u64 {
    10 * 1024 // 10 GB
}

fn default_concurrent_jobs() -> usize {
    1
}

fn default_image() -> String {
    String::from("alpine:latest")
}

fn default_kubernetes_namespace() -> String {
    "default".to_string()
}

fn default_clean_builds() -> bool {
    true
}

fn default_security_enabled() -> bool {
    true  // Security MUST be enabled by default
}

fn default_script_timeout() -> u64 {
    3600  // 1 hour per script
}

fn default_network_mode() -> String {
    "full".to_string()
}

impl RunnerConfig {
    /// 从文件加载配置
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RunnerError::Config(format!("Failed to read config file: {}", e)))?;
        
        let config: RunnerConfig = toml::from_str(&content)
            .map_err(|e| RunnerError::Config(format!("Failed to parse config file: {}", e)))?;
        
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| RunnerError::Config(format!("Failed to serialize config: {}", e)))?;
        
        std::fs::write(path, content)
            .map_err(|e| RunnerError::Config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
}
