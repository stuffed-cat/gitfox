use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::error::{Result, RunnerError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    pub server_url: String,
    pub token: String,
    pub name: String,
    pub tags: Vec<String>,
    pub executor: String,
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
