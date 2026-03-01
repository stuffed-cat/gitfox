use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// GitFox Workhorse 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Workhorse 监听地址
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,

    /// Workhorse 监听端口
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,

    /// 后端 Unix Socket 路径（优先级高于 backend_url）
    #[serde(default)]
    pub backend_socket: Option<String>,

    /// 后端 API 服务器地址
    #[serde(default = "default_backend_url")]
    pub backend_url: String,

    /// 前端 SPA 构建输出目录
    #[serde(default = "default_frontend_dist_path")]
    pub frontend_dist_path: PathBuf,

    /// WebIDE 构建输出目录
    #[serde(default = "default_webide_dist_path")]
    pub webide_dist_path: PathBuf,

    /// Assets 静态文件目录（用户上传的头像等）
    #[serde(default = "default_assets_path")]
    pub assets_path: PathBuf,

    /// Git 仓库路径（用于 Git HTTP 协议）
    #[serde(default = "default_git_repos_path")]
    pub git_repos_path: PathBuf,

    /// 启用请求日志
    #[serde(default = "default_true")]
    pub enable_request_logging: bool,

    /// 启用 CORS
    #[serde(default = "default_true")]
    pub enable_cors: bool,

    /// 最大上传文件大小 (bytes)
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size: usize,

    /// WebSocket 超时时间 (秒)
    #[serde(default = "default_websocket_timeout")]
    pub websocket_timeout: u64,

    /// 静态文件缓存控制头
    #[serde(default = "default_static_cache_control")]
    pub static_cache_control: String,
}

fn default_listen_addr() -> String {
    env::var("WORKHORSE_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string())
}

fn default_listen_port() -> u16 {
    env::var("WORKHORSE_LISTEN_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080)
}

fn default_backend_socket() -> Option<String> {
    env::var("WORKHORSE_BACKEND_SOCKET").ok()
}

fn default_backend_url() -> String {
    env::var("WORKHORSE_BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8081".to_string())
}

fn default_frontend_dist_path() -> PathBuf {
    PathBuf::from(env::var("WORKHORSE_FRONTEND_DIST").unwrap_or_else(|_| "./frontend/dist".to_string()))
}

fn default_webide_dist_path() -> PathBuf {
    PathBuf::from(env::var("WORKHORSE_WEBIDE_DIST").unwrap_or_else(|_| "./webide/dist".to_string()))
}

fn default_assets_path() -> PathBuf {
    PathBuf::from(env::var("WORKHORSE_ASSETS_PATH").unwrap_or_else(|_| "./assets".to_string()))
}

fn default_git_repos_path() -> PathBuf {
    PathBuf::from(env::var("WORKHORSE_GIT_REPOS_PATH").unwrap_or_else(|_| "./repos".to_string()))
}

fn default_true() -> bool {
    true
}

fn default_max_upload_size() -> usize {
    // 默认 500MB (Git push 操作可能需要较大的限制)
    env::var("WORKHORSE_MAX_UPLOAD_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(500 * 1024 * 1024)
}

fn default_websocket_timeout() -> u64 {
    env::var("WORKHORSE_WEBSOCKET_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600) // 1小时
}

fn default_static_cache_control() -> String {
    env::var("WORKHORSE_STATIC_CACHE_CONTROL")
        .unwrap_or_else(|_| "public, max-age=31536000, immutable".to_string())
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            listen_port: default_listen_port(),
            backend_socket: default_backend_socket(),
            backend_url: default_backend_url(),
            frontend_dist_path: default_frontend_dist_path(),
            webide_dist_path: default_webide_dist_path(),
            assets_path: default_assets_path(),
            git_repos_path: default_git_repos_path(),
            enable_request_logging: default_true(),
            enable_cors: default_true(),
            max_upload_size: default_max_upload_size(),
            websocket_timeout: default_websocket_timeout(),
            static_cache_control: default_static_cache_control(),
        }
    }

    /// 从 TOML 文件加载配置
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&content)?;
        config.normalize();
        Ok(config)
    }
    
    /// 规范化配置（将空字符串转为 None）
    pub fn normalize(&mut self) {
        // 空字符串的 backend_socket 视为 None
        if matches!(&self.backend_socket, Some(s) if s.is_empty()) {
            self.backend_socket = None;
        }
    }

    /// 验证配置
    pub fn validate(&self) -> anyhow::Result<()> {
        // 至少需要一种后端连接方式
        if self.backend_socket.is_none() && self.backend_url.is_empty() {
            anyhow::bail!("Either backend_socket or backend_url must be specified");
        }
        
        // 如果使用 HTTP URL，验证格式
        if self.backend_socket.is_none() && !self.backend_url.starts_with("http://") && !self.backend_url.starts_with("https://") {
            anyhow::bail!("backend_url must start with http:// or https://");
        }

        if !self.frontend_dist_path.exists() {
            tracing::warn!(
                "Frontend dist path does not exist: {:?}",
                self.frontend_dist_path
            );
        }

        if !self.webide_dist_path.exists() {
            tracing::warn!(
                "WebIDE dist path does not exist: {:?}",
                self.webide_dist_path
            );
        }

        if !self.assets_path.exists() {
            tracing::warn!("Assets path does not exist: {:?}", self.assets_path);
        }

        if !self.git_repos_path.exists() {
            tracing::warn!("Git repos path does not exist: {:?}", self.git_repos_path);
        }

        Ok(())
    }
}
