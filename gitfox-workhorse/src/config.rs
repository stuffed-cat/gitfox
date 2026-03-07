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

    /// GitLayer gRPC 服务地址（必需，用于处理所有 Git 操作）
    /// 如果未配置，Git HTTP 请求将返回错误
    #[serde(default = "default_gitlayer_address")]
    pub gitlayer_address: Option<String>,

    /// Auth gRPC 服务地址（必需，主应用的 gRPC 地址，用于权限认证）
    /// shell/workhorse 依赖此服务进行认证，必须配置
    #[serde(default)]
    pub auth_grpc_address: Option<String>,

    /// 内部 API 认证密钥
    #[serde(default = "default_shell_secret")]
    pub shell_secret: String,

    // ============ LFS 配置 ============
    
    /// 是否启用 LFS 支持
    #[serde(default = "default_true")]
    pub lfs_enabled: bool,

    /// LFS 对象存储路径
    #[serde(default = "default_lfs_storage_path")]
    pub lfs_storage_path: PathBuf,

    /// LFS 最大对象大小 (bytes)，默认 5GB
    #[serde(default = "default_lfs_max_object_size")]
    pub lfs_max_object_size: u64,

    /// LFS 链接过期时间 (秒)，默认 1 小时
    #[serde(default = "default_lfs_link_expires")]
    pub lfs_link_expires: u64,

    /// LFS 外部 URL（用于生成下载/上传链接，默认与 base_url 相同）
    #[serde(default)]
    pub lfs_external_url: Option<String>,

    // ============ Registry 配置 ============
    
    /// 是否启用 Package Registry
    #[serde(default = "default_true")]
    pub registry_enabled: bool,

    /// Registry 独立域名（如 registry.gitfox.studio）
    /// 启用 Registry 时必须配置，不支持使用主域名
    #[serde(default)]
    pub registry_domain: Option<String>,

    /// 是否启用 Docker Registry
    #[serde(default = "default_true")]
    pub registry_docker_enabled: bool,

    /// 是否启用 npm Registry
    #[serde(default = "default_true")]
    pub registry_npm_enabled: bool,

    /// 是否启用 Cargo Registry
    #[serde(default = "default_true")]
    pub registry_cargo_enabled: bool,

    /// Registry 存储路径
    #[serde(default = "default_registry_storage_path")]
    pub registry_storage_path: PathBuf,

    /// Registry 最大包大小 (bytes)，默认 512MB
    #[serde(default = "default_registry_max_size")]
    pub registry_max_size: u64,
}

fn default_registry_storage_path() -> PathBuf {
    PathBuf::from(env::var("REGISTRY_STORAGE_PATH").unwrap_or_else(|_| "./packages".to_string()))
}

fn default_registry_max_size() -> u64 {
    env::var("REGISTRY_MAX_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(512 * 1024 * 1024) // 512MB
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

fn default_true() -> bool {
    true
}

fn default_max_upload_size() -> usize {
    // 默认 10GB (支持大型 Git 仓库 push 操作，如 ICU 4.28GB)
    // 注意：这是单次请求的最大大小，流式传输不会一次性加载到内存
    env::var("WORKHORSE_MAX_UPLOAD_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10 * 1024 * 1024 * 1024)
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

fn default_gitlayer_address() -> Option<String> {
    env::var("GITLAYER_ADDRESS").ok()
}

fn default_auth_grpc_address() -> Option<String> {
    env::var("AUTH_GRPC_ADDRESS")
        .or_else(|_| env::var("GITFOX_AUTH_GRPC_ADDRESS"))
        .ok()
}

fn default_lfs_storage_path() -> PathBuf {
    PathBuf::from(env::var("LFS_STORAGE_PATH").unwrap_or_else(|_| "./lfs-objects".to_string()))
}

fn default_lfs_max_object_size() -> u64 {
    // 默认 5GB
    env::var("LFS_MAX_OBJECT_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5 * 1024 * 1024 * 1024)
}

fn default_lfs_link_expires() -> u64 {
    // 默认 1 小时
    env::var("LFS_LINK_EXPIRES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600)
}

fn default_lfs_external_url() -> Option<String> {
    env::var("LFS_EXTERNAL_URL").ok()
}

fn default_shell_secret() -> String {
    env::var("GITFOX_SHELL_SECRET")
        .or_else(|_| env::var("GITFOX_API_SECRET"))
        .unwrap_or_else(|_| "change-me-in-production".to_string())
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        let gitlayer_address = default_gitlayer_address();
        let auth_grpc_address = default_auth_grpc_address();
        
        Self {
            listen_addr: default_listen_addr(),
            listen_port: default_listen_port(),
            backend_socket: default_backend_socket(),
            backend_url: default_backend_url(),
            frontend_dist_path: default_frontend_dist_path(),
            webide_dist_path: default_webide_dist_path(),
            assets_path: default_assets_path(),

            enable_request_logging: default_true(),
            enable_cors: default_true(),
            max_upload_size: default_max_upload_size(),
            websocket_timeout: default_websocket_timeout(),
            static_cache_control: default_static_cache_control(),
            gitlayer_address,
            auth_grpc_address,
            shell_secret: default_shell_secret(),
            
            // LFS 配置
            lfs_enabled: default_true(),
            lfs_storage_path: default_lfs_storage_path(),
            lfs_max_object_size: default_lfs_max_object_size(),
            lfs_link_expires: default_lfs_link_expires(),
            lfs_external_url: default_lfs_external_url(),
            
            // Registry 配置
            registry_enabled: env::var("REGISTRY_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            registry_domain: env::var("REGISTRY_DOMAIN").ok().filter(|s| !s.is_empty()),
            registry_docker_enabled: env::var("REGISTRY_DOCKER_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            registry_npm_enabled: env::var("REGISTRY_NPM_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            registry_cargo_enabled: env::var("REGISTRY_CARGO_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            registry_storage_path: default_registry_storage_path(),
            registry_max_size: default_registry_max_size(),
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
        
        // 如果启用 Package Registry，必须配置独立域名
        // 不允许 registry 使用主域名，会有兼容性问题（如 /v2/ 可能与其他路由冲突）
        if self.registry_enabled && self.registry_domain.is_none() {
            anyhow::bail!(
                "registry_domain is required when registry is enabled. \
                Package registry must use a dedicated domain (e.g., registry.gitfox.studio) \
                to avoid path conflicts with the main application."
            );
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

        Ok(())
    }
}
