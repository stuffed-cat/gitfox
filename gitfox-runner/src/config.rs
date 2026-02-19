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
    
    /// Default Docker image for docker executor
    #[serde(default = "default_docker_image")]
    pub default_docker_image: String,
    
    /// Clean builds directory after job completion
    #[serde(default = "default_clean_builds")]
    pub clean_builds: bool,
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

fn default_docker_image() -> String {
    String::from("alpine:latest")
}

fn default_clean_builds() -> bool {
    true
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
