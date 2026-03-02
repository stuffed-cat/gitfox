//! GitLayer configuration

use std::env;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// gRPC listen address
    pub listen_addr: String,
    
    /// Base path for git repositories
    pub storage_path: String,
    
    /// Git binary path (optional, defaults to "git")
    pub git_bin_path: String,
    
    /// Max concurrent operations per repository
    pub max_concurrent_ops: usize,
    
    /// Enable repository caching
    pub enable_cache: bool,
    
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9999".to_string(),
            storage_path: "./repos".to_string(),
            git_bin_path: "git".to_string(),
            max_concurrent_ops: 10,
            enable_cache: true,
            cache_ttl_secs: 60,
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    pub fn load() -> Self {
        let mut config = Self::default();
        
        if let Ok(addr) = env::var("GITLAYER_LISTEN_ADDR") {
            config.listen_addr = addr;
        }
        
        if let Ok(path) = env::var("GITLAYER_STORAGE_PATH") {
            config.storage_path = path;
        } else if let Ok(path) = env::var("GIT_REPOS_PATH") {
            config.storage_path = path;
        }
        
        if let Ok(path) = env::var("GITLAYER_GIT_BIN") {
            config.git_bin_path = path;
        }
        
        if let Ok(val) = env::var("GITLAYER_MAX_CONCURRENT_OPS") {
            config.max_concurrent_ops = val.parse().unwrap_or(10);
        }
        
        if let Ok(val) = env::var("GITLAYER_ENABLE_CACHE") {
            config.enable_cache = val.parse().unwrap_or(true);
        }
        
        if let Ok(val) = env::var("GITLAYER_CACHE_TTL") {
            config.cache_ttl_secs = val.parse().unwrap_or(60);
        }
        
        config
    }
    
    /// Get full repository path
    pub fn repo_path(&self, relative_path: &str) -> String {
        format!("{}/{}", self.storage_path, relative_path)
    }
}
