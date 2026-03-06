//! GitFox 统一配置模块
//!
//! 设计原则：
//! 1. 最小化配置：只包含启动必需的配置，业务配置留给 system_configs + Admin UI
//! 2. 单一来源：一个 gitfox.toml 文件生成所有组件配置
//! 3. 版本化：支持配置迁移
//! 4. 用户友好：清晰的注释和合理的默认值
//!
//! 配置层次：
//! ```
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   system_configs 表（数据库）                 │
//! │  可热更新：站点名称、注册开关、WebIDE、CI/CD 设置等           │
//! │  通过 Admin -> Settings UI 管理                              │
//! └─────────────────────────────────────────────────────────────┘
//!                               ↑ 运行时读写
//!
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  gitfox.toml（启动配置）                      │
//! │  启动必需：数据库、端口、密钥、路径等                         │
//! │  由 `gitfox init` 生成，`gitfox start` 读取                  │
//! └─────────────────────────────────────────────────────────────┘
//!                               ↓ 启动时生成环境变量
//!
//! ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐
//! │  Backend  │  │ Workhorse │  │ GitLayer  │  │   Shell   │
//! └───────────┘  └───────────┘  └───────────┘  └───────────┘
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// 配置文件版本
pub const CONFIG_VERSION: &str = "1.1.1";

/// 统一配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFoxConfig {
    /// 配置文件版本（用于迁移）
    #[serde(default = "default_version")]
    pub version: String,

    /// 数据库配置
    pub database: DatabaseConfig,

    /// Redis 配置
    pub redis: RedisConfig,

    /// 安全密钥
    pub secrets: SecretsConfig,

    /// 对外服务配置
    pub server: ServerConfig,

    /// 内部服务配置
    #[serde(default)]
    pub internal: InternalConfig,

    /// 数据目录配置
    #[serde(default)]
    pub paths: PathsConfig,

    /// 初始管理员配置
    #[serde(default)]
    pub admin: AdminConfig,

    /// SMTP 邮件配置
    #[serde(default)]
    pub smtp: SmtpConfig,

    /// OAuth 第三方登录配置
    #[serde(default)]
    pub oauth: OAuthConfig,

    /// WebAuthn 配置
    #[serde(default)]
    pub webauthn: WebauthnConfig,

    /// PAT 配置
    #[serde(default)]
    pub pat: PatConfig,

    /// 日志配置
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Package Registry 配置
    #[serde(default)]
    pub registry: RegistryConfig,

    /// 服务启用配置（控制哪些组件启动）
    #[serde(default)]
    pub services: ServicesConfig,

    /// Workhorse 配置
    #[serde(default)]
    pub workhorse: WorkhorseConfig,

    /// GitLayer 配置
    #[serde(default)]
    pub gitlayer: GitLayerConfig,

    /// 内置服务配置（GitLab Omnibus 风格）
    #[serde(default)]
    pub bundled: BundledConfig,

    /// WebIDE 配置
    #[serde(default)]
    pub webide: WebideConfig,
}

fn default_version() -> String {
    CONFIG_VERSION.to_string()
}

// ═══════════════════════════════════════════════════════════════════════
// 数据库和缓存
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL 连接字符串
    /// 格式: postgres://user:password@host:port/database
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis 连接字符串
    /// 格式: redis://host:port 或 redis://:password@host:port
    #[serde(default = "default_redis_url")]
    pub url: String,
}

fn default_redis_url() -> String {
    "redis://localhost:6379".to_string()
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: default_redis_url(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 安全密钥
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// JWT 签名密钥（用于用户认证 Token）
    pub jwt: String,

    /// JWT Token 过期时间（秒）
    #[serde(default = "default_jwt_expiration")]
    pub jwt_expiration: u64,

    /// 组件间通信密钥（gitfox-shell 等内部服务认证）
    pub internal: String,
}

fn default_jwt_expiration() -> u64 {
    86400 // 24 小时
}

// ═══════════════════════════════════════════════════════════════════════
// 对外服务配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 公开访问的 URL（用于 OAuth 回调、邮件链接、克隆 URL 等）
    /// 示例: https://git.example.com 或 http://192.168.1.100:8080
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// HTTP 端口（Workhorse 对外监听）
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// 最大上传文件大小 (字节)
    #[serde(default = "default_max_upload_size")]
    pub max_upload_size: u64,

    /// SSH 配置
    #[serde(default)]
    pub ssh: SshConfig,
}

fn default_base_url() -> String {
    "http://localhost:8080".to_string()
}

fn default_http_port() -> u16 {
    8080
}

fn default_max_upload_size() -> u64 {
    1073741824 // 1GB
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            http_port: default_http_port(),
            max_upload_size: default_max_upload_size(),
            ssh: SshConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    /// 是否启用 SSH 服务
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// SSH 监听地址
    #[serde(default = "default_ssh_host")]
    pub host: String,

    /// SSH 监听端口
    #[serde(default = "default_ssh_port")]
    pub port: u16,

    /// 公开显示的 SSH 地址（前端克隆 URL 中显示）
    /// 如果使用端口映射，这里填写外部地址
    #[serde(default = "default_ssh_public_host")]
    pub public_host: String,

    /// 公开显示的 SSH 端口
    #[serde(default = "default_ssh_port")]
    pub public_port: u16,
}

fn default_true() -> bool {
    true
}

fn default_ssh_host() -> String {
    "0.0.0.0".to_string()
}

fn default_ssh_port() -> u16 {
    2222
}

fn default_ssh_public_host() -> String {
    "localhost".to_string()
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: default_ssh_host(),
            port: default_ssh_port(),
            public_host: default_ssh_public_host(),
            public_port: default_ssh_port(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 内部服务配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalConfig {
    /// 内部服务绑定主机（默认 127.0.0.1）
    /// All-in-one 模式：保持默认
    /// 分布式模式：设置为 0.0.0.0 让服务可被外部访问
    #[serde(default = "default_internal_host")]
    pub host: String,

    /// 后端 Unix Socket 路径（优先级最高）
    /// 推荐用于 all-in-one 模式，性能更好更安全
    #[serde(default)]
    pub backend_socket: Option<String>,

    /// 后端 API URL（完整地址，覆盖 host:backend_port）
    /// 分布式部署时使用，如 "http://backend.internal:8081"
    #[serde(default)]
    pub backend_url: Option<String>,

    /// 后端 API 端口（当不使用 Unix Socket 或 backend_url 时）
    #[serde(default = "default_backend_port")]
    pub backend_port: u16,

    /// GitLayer gRPC 完整地址（覆盖 host:gitlayer_port）
    /// 分布式部署时使用，如 "http://gitlayer.internal:50052"
    #[serde(default)]
    pub gitlayer_address: Option<String>,

    /// GitLayer gRPC 端口（Git 操作服务）
    #[serde(default = "default_gitlayer_port")]
    pub gitlayer_port: u16,

    /// Auth gRPC 完整地址（覆盖 host:auth_grpc_port）
    /// 分布式部署时使用，如 "http://auth.internal:50051"
    #[serde(default)]
    pub auth_grpc_address: Option<String>,

    /// Auth gRPC 端口（认证服务）
    #[serde(default = "default_auth_grpc_port")]
    pub auth_grpc_port: u16,
}

fn default_internal_host() -> String {
    "127.0.0.1".to_string()
}

fn default_backend_port() -> u16 {
    8081
}

fn default_gitlayer_port() -> u16 {
    50052
}

fn default_auth_grpc_port() -> u16 {
    50051
}

impl Default for InternalConfig {
    fn default() -> Self {
        Self {
            host: default_internal_host(),
            backend_socket: None,
            backend_url: None,
            backend_port: default_backend_port(),
            gitlayer_address: None,
            gitlayer_port: default_gitlayer_port(),
            auth_grpc_address: None,
            auth_grpc_port: default_auth_grpc_port(),
        }
    }
}

impl InternalConfig {
    /// 获取后端 URL（优先级：backend_socket > backend_url > host:port）
    pub fn get_backend_url(&self) -> String {
        if let Some(ref url) = self.backend_url {
            url.clone()
        } else {
            format!("http://{}:{}", self.host, self.backend_port)
        }
    }

    /// 获取 GitLayer gRPC 地址
    pub fn get_gitlayer_address(&self) -> String {
        if let Some(ref addr) = self.gitlayer_address {
            addr.clone()
        } else {
            format!("http://{}:{}", self.host, self.gitlayer_port)
        }
    }

    /// 获取 Auth gRPC 地址
    pub fn get_auth_grpc_address(&self) -> String {
        if let Some(ref addr) = self.auth_grpc_address {
            addr.clone()
        } else {
            format!("http://{}:{}", self.host, self.auth_grpc_port)
        }
    }

    /// 获取 Auth gRPC 绑定地址（服务端监听用）
    pub fn get_auth_grpc_listen_address(&self) -> String {
        format!("{}:{}", self.host, self.auth_grpc_port)
    }

    /// 获取 GitLayer gRPC 绑定地址（服务端监听用）
    pub fn get_gitlayer_listen_address(&self) -> String {
        format!("{}:{}", self.host, self.gitlayer_port)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 数据目录配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    /// Git 仓库存储目录
    #[serde(default = "default_repos_path")]
    pub repos: String,

    /// 前端静态文件目录
    #[serde(default = "default_frontend_path")]
    pub frontend: String,

    /// WebIDE 静态文件目录
    #[serde(default = "default_webide_path")]
    pub webide: String,

    /// Assets 目录（用户上传的头像等）
    #[serde(default = "default_assets_path")]
    pub assets: String,

    /// SSH Host Key 路径
    #[serde(default = "default_ssh_host_key_path")]
    pub ssh_host_key: String,

    /// gitfox-shell 二进制路径（omnibus 模式自动设置）
    #[serde(default)]
    pub shell_binary: String,
}

fn default_repos_path() -> String {
    "./repos".to_string()
}

fn default_frontend_path() -> String {
    "./frontend/dist".to_string()
}

fn default_webide_path() -> String {
    "./webide/dist".to_string()
}

fn default_assets_path() -> String {
    "./assets".to_string()
}

fn default_ssh_host_key_path() -> String {
    "./data/ssh/host_key".to_string()
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            repos: default_repos_path(),
            frontend: default_frontend_path(),
            webide: default_webide_path(),
            assets: default_assets_path(),
            ssh_host_key: default_ssh_host_key_path(),
            shell_binary: String::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 初始管理员配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminConfig {
    /// 管理员用户名
    #[serde(default)]
    pub username: Option<String>,

    /// 管理员邮箱
    #[serde(default)]
    pub email: Option<String>,

    /// 管理员密码（初始化时自动生成，首次启动后可删除此字段）
    #[serde(default)]
    pub password: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════
// SMTP 邮件配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// 是否启用 SMTP
    #[serde(default)]
    pub enabled: bool,

    /// SMTP 服务器地址
    #[serde(default)]
    pub host: String,

    /// SMTP 服务器端口
    #[serde(default = "default_smtp_port")]
    pub port: u16,

    /// SMTP 用户名
    #[serde(default)]
    pub username: String,

    /// SMTP 密码
    #[serde(default)]
    pub password: String,

    /// 发件人邮箱
    #[serde(default = "default_from_email")]
    pub from_email: String,

    /// 发件人名称
    #[serde(default = "default_from_name")]
    pub from_name: String,

    /// 使用 TLS (STARTTLS)
    #[serde(default = "default_true")]
    pub use_tls: bool,

    /// 使用 SSL (隐式 TLS)
    #[serde(default)]
    pub use_ssl: bool,
}

fn default_smtp_port() -> u16 {
    587
}

fn default_from_email() -> String {
    "noreply@gitfox.local".to_string()
}

fn default_from_name() -> String {
    "GitFox".to_string()
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: String::new(),
            port: default_smtp_port(),
            username: String::new(),
            password: String::new(),
            from_email: default_from_email(),
            from_name: default_from_name(),
            use_tls: true,
            use_ssl: false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// OAuth 第三方登录配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthConfig {
    #[serde(default)]
    pub github: OAuthProviderConfig,

    #[serde(default)]
    pub gitlab: OAuthProviderConfig,

    #[serde(default)]
    pub google: OAuthProviderConfig,

    #[serde(default)]
    pub azure_ad: OAuthProviderConfig,

    #[serde(default)]
    pub bitbucket: OAuthProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuthProviderConfig {
    #[serde(default)]
    pub client_id: String,

    #[serde(default)]
    pub client_secret: String,

    /// 自建实例的 URL（如 GitLab 自建）
    #[serde(default)]
    pub url: Option<String>,

    /// Azure AD 租户 ID
    #[serde(default)]
    pub tenant_id: Option<String>,
}

impl OAuthProviderConfig {
    pub fn is_enabled(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 日志配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// 日志级别: debug, info, warn, error
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WebAuthn 配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebauthnConfig {
    /// Relying Party 名称（用户认证时显示）
    #[serde(default = "default_webauthn_rp_name")]
    pub rp_name: String,

    /// Relying Party ID（通常是域名）
    #[serde(default)]
    pub rp_id: String,

    /// Relying Party Origin（如 https://example.com）
    #[serde(default)]
    pub rp_origin: String,
}

fn default_webauthn_rp_name() -> String {
    "GitFox".to_string()
}

impl Default for WebauthnConfig {
    fn default() -> Self {
        Self {
            rp_name: default_webauthn_rp_name(),
            rp_id: String::new(),
            rp_origin: String::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// PAT 配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatConfig {
    /// PAT 默认过期天数
    /// 0 = 永不过期（不推荐）
    #[serde(default = "default_pat_default_expiration_days")]
    pub default_expiration_days: u32,

    /// PAT 最大过期天数
    /// 0 = 无限制
    #[serde(default)]
    pub max_expiration_days: u32,
}

fn default_pat_default_expiration_days() -> u32 {
    365 // 默认一年
}

impl Default for PatConfig {
    fn default() -> Self {
        Self {
            default_expiration_days: default_pat_default_expiration_days(),
            max_expiration_days: 0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// WebIDE 配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebideConfig {
    /// WebIDE OAuth 应用客户端 ID
    #[serde(default = "default_webide_client_id")]
    pub client_id: String,

    /// WebIDE OAuth 回调路径
    #[serde(default = "default_webide_redirect_uri_path")]
    pub redirect_uri_path: String,
}

fn default_webide_client_id() -> String {
    "gitfox-webide".to_string()
}

fn default_webide_redirect_uri_path() -> String {
    "/-/ide/oauth/callback".to_string()
}

impl Default for WebideConfig {
    fn default() -> Self {
        Self {
            client_id: default_webide_client_id(),
            redirect_uri_path: default_webide_redirect_uri_path(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Package Registry 配置
// ═══════════════════════════════════════════════════════════════════════

/// Package Registry 配置（Docker/npm 等软件包仓库）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// 是否启用 Package Registry
    #[serde(default)]
    pub enabled: bool,

    /// Registry 独立域名（如 registry.gitfox.studio）
    /// 如果配置了独立域名，Docker/npm 请求将通过此域名处理
    #[serde(default)]
    pub domain: String,

    /// 是否启用 Docker Registry
    #[serde(default = "default_true")]
    pub docker_enabled: bool,

    /// 是否启用 npm Registry
    #[serde(default = "default_true")]
    pub npm_enabled: bool,

    /// Registry 存储路径
    #[serde(default = "default_registry_storage_path")]
    pub storage_path: String,

    /// 最大软件包大小 (字节，默认 500MB)
    #[serde(default = "default_registry_max_size")]
    pub max_package_size: u64,

    /// JWT 密钥（用于 Docker Registry Token 认证）
    /// 如果为空，将使用 secrets.jwt
    #[serde(default)]
    pub jwt_secret: String,
}

fn default_registry_storage_path() -> String {
    "./registry-storage".to_string()
}

fn default_registry_max_size() -> u64 {
    524288000 // 500MB
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            domain: String::new(),
            docker_enabled: true,
            npm_enabled: true,
            storage_path: default_registry_storage_path(),
            max_package_size: default_registry_max_size(),
            jwt_secret: String::new(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 服务启用配置（控制哪些组件启动）
// ═══════════════════════════════════════════════════════════════════════

/// 服务启用配置
/// 
/// 控制哪些 GitFox 核心组件应该启动。
/// 允许用户只运行部分组件，而不是 all-in-one 模式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    /// 是否启动 devops 后端（API + gRPC Auth）
    #[serde(default = "default_true")]
    pub backend: bool,

    /// 是否启动 GitLayer（Git 操作 gRPC 服务）
    #[serde(default = "default_true")]
    pub gitlayer: bool,

    /// 是否启动 gitfox-shell（SSH 服务器）
    /// 注意：也需要 server.ssh.enabled = true
    #[serde(default = "default_true")]
    pub shell: bool,

    /// 是否启动 workhorse（HTTP 反向代理）
    #[serde(default = "default_true")]
    pub workhorse: bool,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            backend: true,
            gitlayer: true,
            shell: true,
            workhorse: true,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Workhorse 配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkhorseConfig {
    /// 监听地址
    #[serde(default = "default_workhorse_listen_addr")]
    pub listen_addr: String,

    /// 启用请求日志
    #[serde(default = "default_true")]
    pub enable_request_logging: bool,

    /// 启用 CORS
    #[serde(default = "default_true")]
    pub enable_cors: bool,

    /// WebSocket 超时时间（秒）
    #[serde(default = "default_websocket_timeout")]
    pub websocket_timeout: u32,

    /// 静态文件缓存控制头
    #[serde(default = "default_static_cache_control")]
    pub static_cache_control: String,
}

fn default_workhorse_listen_addr() -> String {
    "0.0.0.0".to_string()
}

fn default_websocket_timeout() -> u32 {
    3600
}

fn default_static_cache_control() -> String {
    "public, max-age=31536000, immutable".to_string()
}

impl Default for WorkhorseConfig {
    fn default() -> Self {
        Self {
            listen_addr: default_workhorse_listen_addr(),
            enable_request_logging: true,
            enable_cors: true,
            websocket_timeout: default_websocket_timeout(),
            static_cache_control: default_static_cache_control(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// GitLayer 配置
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLayerConfig {
    /// Git 二进制文件路径
    #[serde(default = "default_git_bin")]
    pub git_bin: String,

    /// 最大并发操作数
    #[serde(default = "default_max_concurrent_ops")]
    pub max_concurrent_ops: u32,

    /// 启用缓存
    #[serde(default = "default_true")]
    pub enable_cache: bool,

    /// 缓存 TTL（秒）
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: u32,
}

fn default_git_bin() -> String {
    "git".to_string()
}

fn default_max_concurrent_ops() -> u32 {
    10
}

fn default_cache_ttl() -> u32 {
    60
}

impl Default for GitLayerConfig {
    fn default() -> Self {
        Self {
            git_bin: default_git_bin(),
            max_concurrent_ops: default_max_concurrent_ops(),
            enable_cache: true,
            cache_ttl: default_cache_ttl(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 内置服务配置（GitLab Omnibus 风格）
// ═══════════════════════════════════════════════════════════════════════

/// 内置服务总配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundledConfig {
    /// 全局开关：是否启用内置服务
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 内置 PostgreSQL 配置
    #[serde(default)]
    pub postgresql: BundledPostgresqlConfig,

    /// 内置 Redis 配置
    #[serde(default)]
    pub redis: BundledRedisConfig,

    /// 内置 Nginx 配置
    #[serde(default)]
    pub nginx: BundledNginxConfig,
}

impl Default for BundledConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            postgresql: BundledPostgresqlConfig::default(),
            redis: BundledRedisConfig::default(),
            nginx: BundledNginxConfig::default(),
        }
    }
}

/// 内置 PostgreSQL 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundledPostgresqlConfig {
    /// 使用内置 PostgreSQL
    #[serde(default)]
    pub enabled: bool,

    /// 监听端口
    #[serde(default = "default_pg_port")]
    pub port: u16,

    /// 监听地址
    #[serde(default = "default_localhost")]
    pub host: String,

    /// 数据库名称
    #[serde(default = "default_pg_database")]
    pub database: String,

    /// 数据库用户名
    #[serde(default = "default_pg_username")]
    pub username: String,

    /// 数据库密码
    #[serde(default)]
    pub password: String,

    /// 最大连接数
    #[serde(default = "default_pg_max_connections")]
    pub max_connections: u32,

    /// 共享缓冲区大小 (MB)
    #[serde(default = "default_pg_shared_buffers")]
    pub shared_buffers: u32,

    /// 工作内存 (MB)
    #[serde(default = "default_pg_work_mem")]
    pub work_mem: u32,
}

fn default_pg_port() -> u16 { 5432 }
fn default_localhost() -> String { "127.0.0.1".to_string() }
fn default_pg_database() -> String { "gitfox".to_string() }
fn default_pg_username() -> String { "gitfox".to_string() }
fn default_pg_max_connections() -> u32 { 100 }
fn default_pg_shared_buffers() -> u32 { 256 }
fn default_pg_work_mem() -> u32 { 4 }

impl Default for BundledPostgresqlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_pg_port(),
            host: default_localhost(),
            database: default_pg_database(),
            username: default_pg_username(),
            password: String::new(),
            max_connections: default_pg_max_connections(),
            shared_buffers: default_pg_shared_buffers(),
            work_mem: default_pg_work_mem(),
        }
    }
}

/// 内置 Redis 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundledRedisConfig {
    /// 使用内置 Redis
    #[serde(default)]
    pub enabled: bool,

    /// 监听端口
    #[serde(default = "default_redis_port")]
    pub port: u16,

    /// 监听地址
    #[serde(default = "default_localhost")]
    pub host: String,

    /// 最大内存 (MB)
    #[serde(default = "default_redis_maxmemory")]
    pub maxmemory: u32,

    /// 内存淘汰策略
    #[serde(default = "default_redis_maxmemory_policy")]
    pub maxmemory_policy: String,

    /// 是否开启持久化
    #[serde(default = "default_true")]
    pub persistence: bool,

    /// 持久化模式: rdb, aof, rdb+aof
    #[serde(default = "default_redis_persistence_mode")]
    pub persistence_mode: String,
}

fn default_redis_port() -> u16 { 6379 }
fn default_redis_maxmemory() -> u32 { 256 }
fn default_redis_maxmemory_policy() -> String { "volatile-lru".to_string() }
fn default_redis_persistence_mode() -> String { "rdb".to_string() }

impl Default for BundledRedisConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_redis_port(),
            host: default_localhost(),
            maxmemory: default_redis_maxmemory(),
            maxmemory_policy: default_redis_maxmemory_policy(),
            persistence: true,
            persistence_mode: default_redis_persistence_mode(),
        }
    }
}

/// 内置 Nginx 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundledNginxConfig {
    /// 使用内置 Nginx
    #[serde(default)]
    pub enabled: bool,

    /// HTTP 监听端口
    #[serde(default = "default_nginx_http_port")]
    pub http_port: u16,

    /// HTTPS 监听端口
    #[serde(default = "default_nginx_https_port")]
    pub https_port: u16,

    /// 监听地址
    #[serde(default = "default_nginx_host")]
    pub host: String,

    /// 启用 SSL/TLS
    #[serde(default)]
    pub ssl_enabled: bool,

    /// SSL 证书路径
    #[serde(default)]
    pub ssl_certificate: String,

    /// SSL 证书密钥路径
    #[serde(default)]
    pub ssl_certificate_key: String,

    /// 上游服务器列表（支持 workhorse 集群负载均衡）
    /// 格式: ["127.0.0.1:8080", "127.0.0.1:8081", "192.168.1.10:8080"]
    #[serde(default = "default_nginx_upstream_servers")]
    pub upstream_servers: Vec<String>,

    // ===== 以下字段已弃用，仅用于 v1.1.0 → v1.1.1 迁移兼容 =====
    // 迁移后这些字段会被忽略，新配置应使用 upstream_servers

    /// [DEPRECATED] 上游服务地址 - 请改用 upstream_servers
    #[serde(default, skip_serializing)]
    pub upstream_host: Option<String>,

    /// [DEPRECATED] 上游服务端口 - 请改用 upstream_servers
    #[serde(default, skip_serializing)]
    pub upstream_port: Option<u16>,

    // ===== 弃用字段结束 =====

    /// 客户端最大请求体大小
    #[serde(default = "default_nginx_client_max_body_size")]
    pub client_max_body_size: String,

    /// 静态文件缓存时间
    #[serde(default = "default_nginx_static_cache_time")]
    pub static_cache_time: String,

    /// 启用 gzip 压缩
    #[serde(default = "default_true")]
    pub gzip_enabled: bool,

    /// 工作进程数（0 = auto）
    #[serde(default)]
    pub worker_processes: u32,

    /// 每个 worker 的最大连接数
    #[serde(default = "default_nginx_worker_connections")]
    pub worker_connections: u32,
}

fn default_nginx_http_port() -> u16 { 80 }
fn default_nginx_https_port() -> u16 { 443 }
fn default_nginx_host() -> String { "0.0.0.0".to_string() }
fn default_nginx_upstream_servers() -> Vec<String> { vec!["127.0.0.1:8080".to_string()] }
fn default_nginx_client_max_body_size() -> String { "1g".to_string() }
fn default_nginx_static_cache_time() -> String { "1h".to_string() }
fn default_nginx_worker_connections() -> u32 { 1024 }

impl Default for BundledNginxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            http_port: default_nginx_http_port(),
            https_port: default_nginx_https_port(),
            host: default_nginx_host(),
            ssl_enabled: false,
            ssl_certificate: String::new(),
            ssl_certificate_key: String::new(),
            upstream_servers: default_nginx_upstream_servers(),
            // 弃用字段
            upstream_host: None,
            upstream_port: None,
            client_max_body_size: default_nginx_client_max_body_size(),
            static_cache_time: default_nginx_static_cache_time(),
            gzip_enabled: true,
            worker_processes: 0,
            worker_connections: default_nginx_worker_connections(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 配置加载和保存
// ═══════════════════════════════════════════════════════════════════════

/// 配置加载结果
pub struct LoadResult {
    /// 加载的配置
    pub config: GitFoxConfig,
    /// 原始版本（迁移前）
    pub original_version: String,
    /// 是否执行了迁移
    pub migrated: bool,
}

impl GitFoxConfig {
    /// 从 TOML 文件加载配置（不执行迁移）
    /// 用于需要检查原始版本的场景（如 upgrade 命令）
    pub fn load_raw(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        Ok(config)
    }

    /// 从 TOML 文件加载配置并执行迁移
    /// 返回配置和迁移状态
    pub fn load_with_migration(path: &Path) -> Result<LoadResult> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let mut config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        let original_version = config.version.clone();
        
        // 检查配置版本，如需迁移则处理
        let migrated = config.check_and_migrate()?;
        
        Ok(LoadResult {
            config,
            original_version,
            migrated,
        })
    }

    /// 从 TOML 文件加载配置（兼容旧 API）
    pub fn load(path: &Path) -> Result<Self> {
        let result = Self::load_with_migration(path)?;
        Ok(result.config)
    }

    /// 将相对路径转换为基于 base_dir 的绝对路径
    /// 应在加载配置后调用
    pub fn resolve_paths(&mut self, base_dir: &Path) {
        // 解析路径的辅助函数
        let resolve = |p: &str| -> String {
            let path = Path::new(p);
            if path.is_absolute() {
                p.to_string()
            } else {
                // 相对路径转为绝对路径
                let resolved = base_dir.join(path);
                // canonicalize 可能失败（目录不存在），用 display 兜底
                resolved.canonicalize()
                    .unwrap_or(resolved)
                    .display()
                    .to_string()
            }
        };
        
        self.paths.repos = resolve(&self.paths.repos);
        self.paths.frontend = resolve(&self.paths.frontend);
        self.paths.webide = resolve(&self.paths.webide);
        self.paths.assets = resolve(&self.paths.assets);
        self.paths.ssh_host_key = resolve(&self.paths.ssh_host_key);
    }

    /// 保存配置到 TOML 文件
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        // 添加文件头注释
        let header = r#"# GitFox 配置文件
# 由 `gitfox init` 生成，`gitfox start` 读取
# 业务配置（站点名称、注册设置等）请在 Admin -> Settings 中修改
#
# 文档: https://docs.gitfox.studio/configuration

"#;
        
        fs::write(path, format!("{}{}", header, content))
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        Ok(())
    }

    /// 迁移后保存配置（只更新版本号和迁移修改的字段，保留原文件其他内容）
    /// 这样不会用默认值覆盖用户的配置
    pub fn save_migration(&self, path: &Path) -> Result<()> {
        use toml::Value;
        
        // 读取原始 TOML
        let original_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let mut doc: Value = toml::from_str(&original_content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        // 只更新版本号
        if let Value::Table(ref mut table) = doc {
            table.insert("version".to_string(), Value::String(self.version.clone()));
            
            // 如果有 bundled.nginx 配置，更新 upstream_servers 并移除废弃字段
            if let Some(Value::Table(ref mut bundled)) = table.get_mut("bundled") {
                if let Some(Value::Table(ref mut nginx)) = bundled.get_mut("nginx") {
                    // 如果迁移代码修改了 upstream_servers，更新它
                    if !self.bundled.nginx.upstream_servers.is_empty() 
                        && self.bundled.nginx.upstream_servers != default_nginx_upstream_servers() 
                    {
                        let servers: Vec<Value> = self.bundled.nginx.upstream_servers
                            .iter()
                            .map(|s| Value::String(s.clone()))
                            .collect();
                        nginx.insert("upstream_servers".to_string(), Value::Array(servers));
                    }
                    // 移除废弃字段
                    nginx.remove("upstream_host");
                    nginx.remove("upstream_port");
                }
            }
        }
        
        // 添加文件头注释
        let header = r#"# GitFox 配置文件
# 由 `gitfox init` 生成，`gitfox start` 读取
# 业务配置（站点名称、注册设置等）请在 Admin -> Settings 中修改
#
# 文档: https://docs.gitfox.studio/configuration

"#;
        
        let content = toml::to_string_pretty(&doc)
            .context("Failed to serialize config")?;
        
        fs::write(path, format!("{}{}", header, content))
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        Ok(())
    }

    /// 检查配置版本并执行迁移
    /// 返回 true 表示执行了迁移，需要保存配置文件
    fn check_and_migrate(&mut self) -> Result<bool> {
        // 当前版本就是最新版本，无需迁移
        if self.version == CONFIG_VERSION {
            return Ok(false);
        }
        
        let mut migrated = false;

        // 从 1.0 迁移到 1.1：添加 bundled 配置
        // 如果用户已经配置了外部数据库/Redis，则默认不启用内置服务
        if self.version == "1.0" {
            tracing::info!("Migrating config from 1.0 to 1.1...");
            
            // 检测是否使用了自定义（非默认）数据库配置
            let has_custom_db = !self.database.url.is_empty() 
                && self.database.url != "postgres://postgres:password@localhost:5432/devops"
                && !self.database.url.contains("{{");
            
            // 检测是否使用了自定义（非默认）Redis 配置
            let has_custom_redis = !self.redis.url.is_empty()
                && self.redis.url != "redis://localhost:6379"
                && self.redis.url != "redis://127.0.0.1:6379"
                && !self.redis.url.contains("{{");
            
            // 如果用户已配置外部服务，则禁用对应的内置服务
            if has_custom_db || has_custom_redis {
                tracing::info!(
                    "Detected existing external services (db: {}, redis: {}), disabling bundled services",
                    has_custom_db, has_custom_redis
                );
                
                // 全局禁用内置服务（用户已有外部服务）
                self.bundled.enabled = false;
                self.bundled.postgresql.enabled = false;
                self.bundled.redis.enabled = false;
                self.bundled.nginx.enabled = false;
            } else {
                // 全新安装或使用默认配置，保持内置服务启用
                tracing::info!("No external services detected, bundled services available");
            }
            
            // 更新版本号
            self.version = "1.1".to_string();
            migrated = true;
            tracing::info!("Config migrated to version 1.1");
        }

        // 从 1.1 迁移到 1.1.1：upstream_host/port → upstream_servers
        // 安全迁移：保留旧配置数据，自动转换为新格式
        if self.version == "1.1" {
            tracing::info!("Migrating config from 1.1 to 1.1.1...");
            
            // 检查是否有旧版本的 upstream_host/upstream_port 配置
            // 如果有，转换为 upstream_servers 格式
            if let (Some(host), Some(port)) = (&self.bundled.nginx.upstream_host, self.bundled.nginx.upstream_port) {
                let upstream_addr = format!("{}:{}", host, port);
                
                // 只有当 upstream_servers 还是默认值时才迁移
                // 避免覆盖用户手动配置的 upstream_servers
                if self.bundled.nginx.upstream_servers == default_nginx_upstream_servers() {
                    tracing::info!(
                        "Converting deprecated upstream_host:upstream_port ({}) to upstream_servers",
                        upstream_addr
                    );
                    self.bundled.nginx.upstream_servers = vec![upstream_addr];
                }
                
                // 清除已弃用字段（不会写入新配置）
                self.bundled.nginx.upstream_host = None;
                self.bundled.nginx.upstream_port = None;
            }
            
            // 更新版本号
            self.version = "1.1.1".to_string();
            migrated = true;
            tracing::info!("Config migrated to version 1.1.1");
        }

        Ok(migrated)
    }

    /// 验证配置有效性
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // 检查必填项
        if self.database.url.is_empty() {
            return Err(anyhow::anyhow!("database.url is required"));
        }

        // 检查密钥长度
        if self.secrets.jwt.len() < 32 {
            warnings.push("Warning: secrets.jwt should be at least 32 characters for security".to_string());
        }
        if self.secrets.internal.len() < 32 {
            warnings.push("Warning: secrets.internal should be at least 32 characters for security".to_string());
        }

        // 检查端口冲突
        let http_port = self.server.http_port;
        let backend_port = self.internal.backend_port;
        let gitlayer_port = self.internal.gitlayer_port;
        let auth_grpc_port = self.internal.auth_grpc_port;
        let ssh_port = self.server.ssh.port;

        let ports = vec![
            ("http", http_port),
            ("backend", backend_port),
            ("gitlayer", gitlayer_port),
            ("auth_grpc", auth_grpc_port),
            ("ssh", ssh_port),
        ];

        for i in 0..ports.len() {
            for j in (i + 1)..ports.len() {
                if ports[i].1 == ports[j].1 {
                    return Err(anyhow::anyhow!(
                        "Port conflict: {} and {} both use port {}",
                        ports[i].0, ports[j].0, ports[i].1
                    ));
                }
            }
        }

        // 检查 SMTP 配置完整性
        if self.smtp.enabled {
            if self.smtp.host.is_empty() {
                return Err(anyhow::anyhow!("smtp.host is required when SMTP is enabled"));
            }
        }

        Ok(warnings)
    }

    /// 生成后端（devops）所需的环境变量（HashMap 形式，已废弃，保留用于兼容）
    #[deprecated(note = "使用 to_backend_vars() + fill_template() 代替")]
    pub fn to_backend_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // 当内置服务启用时，根据内置配置生成连接 URL
        let database_url = if self.bundled.enabled && self.bundled.postgresql.enabled {
            let pg = &self.bundled.postgresql;
            let password = if pg.password.is_empty() {
                String::new()
            } else {
                format!(":{}", pg.password)
            };
            format!(
                "postgres://{}{}@{}:{}/{}",
                pg.username, password, pg.host, pg.port, pg.database
            )
        } else {
            self.database.url.clone()
        };

        let redis_url = if self.bundled.enabled && self.bundled.redis.enabled {
            let redis = &self.bundled.redis;
            format!("redis://{}:{}", redis.host, redis.port)
        } else {
            self.redis.url.clone()
        };

        // 数据库
        env.insert("DATABASE_URL".to_string(), database_url);
        env.insert("REDIS_URL".to_string(), redis_url);

        // 安全
        env.insert("JWT_SECRET".to_string(), self.secrets.jwt.clone());
        env.insert("GITFOX_SHELL_SECRET".to_string(), self.secrets.internal.clone());

        // 服务器
        env.insert("GITFOX_BASE_URL".to_string(), self.server.base_url.clone());

        // 后端连接配置
        if let Some(ref socket) = self.internal.backend_socket {
            env.insert("SERVER_CONNECTION_TYPE".to_string(), "unix_socket".to_string());
            env.insert("SERVER_SOCKET_PATH".to_string(), socket.clone());
        } else {
            env.insert("SERVER_CONNECTION_TYPE".to_string(), "tcp".to_string());
            env.insert("SERVER_HOST".to_string(), self.internal.host.clone());
            env.insert("SERVER_PORT".to_string(), self.internal.backend_port.to_string());
        }

        // SSH
        env.insert("SSH_ENABLED".to_string(), self.server.ssh.enabled.to_string());
        env.insert("SSH_PUBLIC_HOST".to_string(), self.server.ssh.public_host.clone());
        env.insert("SSH_PUBLIC_PORT".to_string(), self.server.ssh.public_port.to_string());

        // 路径
        env.insert("GIT_REPOS_PATH".to_string(), self.paths.repos.clone());
        env.insert("ASSETS_PATH".to_string(), self.paths.assets.clone());

        // gRPC - 强制启用（shell/workhorse 依赖 gRPC auth）
        env.insert("GRPC_ADDRESS".to_string(), self.internal.get_auth_grpc_listen_address());

        // GitLayer - 使用配置的完整地址或默认 host:port
        env.insert("GITLAYER_ADDRESS".to_string(), self.internal.get_gitlayer_address());

        // 初始管理员
        if let Some(ref username) = self.admin.username {
            env.insert("INITIAL_ADMIN_USERNAME".to_string(), username.clone());
        }
        if let Some(ref email) = self.admin.email {
            env.insert("INITIAL_ADMIN_EMAIL".to_string(), email.clone());
        }
        if let Some(ref password) = self.admin.password {
            env.insert("INITIAL_ADMIN_PASSWORD".to_string(), password.clone());
        }

        // SMTP
        if self.smtp.enabled {
            env.insert("SMTP_ENABLED".to_string(), "true".to_string());
            env.insert("SMTP_HOST".to_string(), self.smtp.host.clone());
            env.insert("SMTP_PORT".to_string(), self.smtp.port.to_string());
            env.insert("SMTP_USERNAME".to_string(), self.smtp.username.clone());
            env.insert("SMTP_PASSWORD".to_string(), self.smtp.password.clone());
            env.insert("SMTP_FROM_EMAIL".to_string(), self.smtp.from_email.clone());
            env.insert("SMTP_FROM_NAME".to_string(), self.smtp.from_name.clone());
            env.insert("SMTP_USE_TLS".to_string(), self.smtp.use_tls.to_string());
            env.insert("SMTP_USE_SSL".to_string(), self.smtp.use_ssl.to_string());
        }

        // OAuth
        if self.oauth.github.is_enabled() {
            env.insert("OAUTH_GITHUB_CLIENT_ID".to_string(), self.oauth.github.client_id.clone());
            env.insert("OAUTH_GITHUB_CLIENT_SECRET".to_string(), self.oauth.github.client_secret.clone());
        }
        if self.oauth.gitlab.is_enabled() {
            env.insert("OAUTH_GITLAB_CLIENT_ID".to_string(), self.oauth.gitlab.client_id.clone());
            env.insert("OAUTH_GITLAB_CLIENT_SECRET".to_string(), self.oauth.gitlab.client_secret.clone());
            if let Some(ref url) = self.oauth.gitlab.url {
                env.insert("OAUTH_GITLAB_URL".to_string(), url.clone());
            }
        }
        if self.oauth.google.is_enabled() {
            env.insert("OAUTH_GOOGLE_CLIENT_ID".to_string(), self.oauth.google.client_id.clone());
            env.insert("OAUTH_GOOGLE_CLIENT_SECRET".to_string(), self.oauth.google.client_secret.clone());
        }
        if self.oauth.azure_ad.is_enabled() {
            env.insert("OAUTH_AZURE_CLIENT_ID".to_string(), self.oauth.azure_ad.client_id.clone());
            env.insert("OAUTH_AZURE_CLIENT_SECRET".to_string(), self.oauth.azure_ad.client_secret.clone());
            if let Some(ref tenant_id) = self.oauth.azure_ad.tenant_id {
                env.insert("OAUTH_AZURE_TENANT_ID".to_string(), tenant_id.clone());
            }
        }
        if self.oauth.bitbucket.is_enabled() {
            env.insert("OAUTH_BITBUCKET_CLIENT_ID".to_string(), self.oauth.bitbucket.client_id.clone());
            env.insert("OAUTH_BITBUCKET_CLIENT_SECRET".to_string(), self.oauth.bitbucket.client_secret.clone());
        }

        env
    }

    /// 生成 Workhorse 配置变量（用于模板填充）
    pub fn to_workhorse_vars(&self) -> ConfigVars {
        let registry_jwt = if self.registry.jwt_secret.is_empty() {
            &self.secrets.jwt
        } else {
            &self.registry.jwt_secret
        };

        ConfigVars {
            // Workhorse 专用
            listen_addr: self.workhorse.listen_addr.clone(),
            listen_port: self.server.http_port,
            backend_socket: self.internal.backend_socket.clone().unwrap_or_default(),
            backend_url: self.internal.get_backend_url(),
            auth_grpc_address: self.internal.get_auth_grpc_address(),
            gitlayer_address: self.internal.get_gitlayer_address(),
            gitfox_shell_secret: self.secrets.internal.clone(),
            
            // 路径
            frontend_path: self.paths.frontend.clone(),
            webide_path: self.paths.webide.clone(),
            assets_path: self.paths.assets.clone(),
            
            // HTTP 配置
            max_upload_size: self.server.max_upload_size,
            enable_request_logging: self.workhorse.enable_request_logging,
            enable_cors: self.workhorse.enable_cors,
            websocket_timeout: self.workhorse.websocket_timeout,
            static_cache_control: self.workhorse.static_cache_control.clone(),
            
            // Registry
            registry_enabled: self.registry.enabled,
            registry_domain: self.registry.domain.clone(),
            registry_docker_enabled: self.registry.docker_enabled,
            registry_npm_enabled: self.registry.npm_enabled,
            registry_storage_path: self.registry.storage_path.clone(),
            registry_max_size: self.registry.max_package_size,
            registry_jwt_secret: registry_jwt.clone(),
            
            ..Default::default()
        }
    }

    /// 生成 Workhorse TOML 配置内容（使用模板）
    pub fn to_workhorse_toml(&self, template: &str) -> String {
        let vars = self.to_workhorse_vars();
        vars.fill_template(template)
    }

    /// 生成 GitLayer 所需的环境变量（HashMap 形式，已废弃，保留用于兼容）
    #[deprecated(note = "使用 to_gitlayer_vars() + fill_template() 代替")]
    pub fn to_gitlayer_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        env.insert("GITLAYER_LISTEN_ADDR".to_string(), self.internal.get_gitlayer_listen_address());
        env.insert("GITLAYER_STORAGE_PATH".to_string(), self.paths.repos.clone());
        env.insert("RUST_LOG".to_string(), self.logging.level.clone());

        env
    }

    /// 生成 Shell 所需的环境变量（HashMap 形式，已废弃，保留用于兼容）
    #[deprecated(note = "使用 to_shell_vars() + fill_template() 代替")]
    pub fn to_shell_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        env.insert("SSH_LISTEN_ADDR".to_string(), format!("{}:{}", self.server.ssh.host, self.server.ssh.port));
        env.insert("SSH_HOST_KEY_PATH".to_string(), self.paths.ssh_host_key.clone());
        
        // GitLayer gRPC (用于 Git 操作)
        env.insert("GITLAYER_ADDRESS".to_string(), self.internal.get_gitlayer_address());
        
        // 内部 API 认证密钥
        env.insert("GITFOX_API_SECRET".to_string(), self.secrets.internal.clone());
        
        // 仓库路径
        env.insert("GITFOX_REPOS_PATH".to_string(), self.paths.repos.clone());

        // Auth gRPC (用于权限验证)
        env.insert("AUTH_GRPC_ADDRESS".to_string(), self.internal.get_auth_grpc_address());

        env.insert("RUST_LOG".to_string(), self.logging.level.clone());

        env
    }

    /// 生成 Backend 配置变量（用于模板填充）
    pub fn to_backend_vars(&self) -> ConfigVars {
        // 当内置服务启用时，根据内置配置生成连接 URL
        let database_url = if self.bundled.enabled && self.bundled.postgresql.enabled {
            let pg = &self.bundled.postgresql;
            let password = if pg.password.is_empty() {
                String::new()
            } else {
                format!(":{}", pg.password)
            };
            format!(
                "postgres://{}{}@{}:{}/{}",
                pg.username, password, pg.host, pg.port, pg.database
            )
        } else {
            self.database.url.clone()
        };

        let redis_url = if self.bundled.enabled && self.bundled.redis.enabled {
            let redis = &self.bundled.redis;
            format!("redis://{}:{}", redis.host, redis.port)
        } else {
            self.redis.url.clone()
        };

        ConfigVars {
            // 数据库和缓存
            database_url,
            redis_url,

            // 安全密钥
            jwt_secret: self.secrets.jwt.clone(),
            jwt_expiration: self.secrets.jwt_expiration,
            gitfox_shell_secret: self.secrets.internal.clone(),

            // 服务配置
            gitfox_base_url: self.server.base_url.clone(),
            http_port: self.server.http_port,
            max_upload_size: self.server.max_upload_size,

            // SSH 配置
            ssh_enabled: self.server.ssh.enabled,
            ssh_host: self.server.ssh.host.clone(),
            ssh_port: self.server.ssh.port,
            ssh_public_host: self.server.ssh.public_host.clone(),
            ssh_public_port: self.server.ssh.public_port,

            // 内部服务
            server_connection_type: if self.internal.backend_socket.is_some() {
                "unix_socket".to_string()
            } else {
                "tcp".to_string()
            },
            server_socket_path: self.internal.backend_socket.clone().unwrap_or_default(),
            server_host: self.internal.host.clone(),
            server_port: self.internal.backend_port,

            // 路径
            git_repos_path: self.paths.repos.clone(),
            assets_path: self.paths.assets.clone(),
            ssh_host_key_path: self.paths.ssh_host_key.clone(),
            gitfox_shell_path: self.paths.shell_binary.clone(),

            // 管理员
            initial_admin_username: self.admin.username.clone().unwrap_or_default(),
            initial_admin_email: self.admin.email.clone().unwrap_or_default(),
            initial_admin_password: self.admin.password.clone().unwrap_or_default(),

            // SMTP
            smtp_enabled: self.smtp.enabled,
            smtp_host: self.smtp.host.clone(),
            smtp_port: self.smtp.port,
            smtp_username: self.smtp.username.clone(),
            smtp_password: self.smtp.password.clone(),
            smtp_from_email: self.smtp.from_email.clone(),
            smtp_from_name: self.smtp.from_name.clone(),
            smtp_use_tls: self.smtp.use_tls,
            smtp_use_ssl: self.smtp.use_ssl,

            // OAuth
            oauth_github_client_id: self.oauth.github.client_id.clone(),
            oauth_github_client_secret: self.oauth.github.client_secret.clone(),
            oauth_gitlab_client_id: self.oauth.gitlab.client_id.clone(),
            oauth_gitlab_client_secret: self.oauth.gitlab.client_secret.clone(),
            oauth_google_client_id: self.oauth.google.client_id.clone(),
            oauth_google_client_secret: self.oauth.google.client_secret.clone(),

            // WebAuthn
            webauthn_rp_name: self.webauthn.rp_name.clone(),
            webauthn_rp_id: if self.webauthn.rp_id.is_empty() {
                self.server.base_url.replace("http://", "").replace("https://", "").split(':').next().unwrap_or("localhost").to_string()
            } else {
                self.webauthn.rp_id.clone()
            },
            webauthn_rp_origin: if self.webauthn.rp_origin.is_empty() {
                self.server.base_url.clone()
            } else {
                self.webauthn.rp_origin.clone()
            },

            // PAT
            pat_default_expiration_days: self.pat.default_expiration_days,
            pat_max_expiration_days: self.pat.max_expiration_days,

            // 日志
            rust_log: self.logging.level.clone(),

            // gRPC 监听地址
            grpc_address: self.internal.get_auth_grpc_listen_address(),
            
            // GitLayer 地址
            gitlayer_address: self.internal.get_gitlayer_address(),

            // Package Registry
            registry_enabled: self.registry.enabled,
            registry_domain: self.registry.domain.clone(),
            registry_docker_enabled: self.registry.docker_enabled,
            registry_npm_enabled: self.registry.npm_enabled,
            registry_storage_path: self.registry.storage_path.clone(),
            registry_max_size: self.registry.max_package_size,
            registry_jwt_secret: if self.registry.jwt_secret.is_empty() {
                self.secrets.jwt.clone()
            } else {
                self.registry.jwt_secret.clone()
            },

            // WebIDE
            webide_client_id: self.webide.client_id.clone(),
            webide_redirect_uri_path: self.webide.redirect_uri_path.clone(),

            ..Default::default()
        }
    }

    /// 生成 Backend .env 配置内容（使用模板）
    pub fn to_backend_env_template(&self, template: &str) -> String {
        let vars = self.to_backend_vars();
        vars.fill_template(template)
    }

    /// 生成 GitLayer 配置变量（用于模板填充）
    pub fn to_gitlayer_vars(&self) -> ConfigVars {
        ConfigVars {
            // gRPC 监听地址
            gitlayer_listen_addr: self.internal.get_gitlayer_listen_address(),
            
            // 仓库存储路径
            git_repos_path: self.paths.repos.clone(),
            
            // Git 二进制路径
            gitlayer_git_bin: self.gitlayer.git_bin.clone(),
            
            // 性能配置
            gitlayer_max_concurrent_ops: self.gitlayer.max_concurrent_ops,
            gitlayer_enable_cache: self.gitlayer.enable_cache,
            gitlayer_cache_ttl: self.gitlayer.cache_ttl,
            
            // 日志
            rust_log: self.logging.level.clone(),

            ..Default::default()
        }
    }

    /// 生成 GitLayer .env 配置内容（使用模板）
    pub fn to_gitlayer_env_template(&self, template: &str) -> String {
        let vars = self.to_gitlayer_vars();
        vars.fill_template(template)
    }

    /// 生成 Shell 配置变量（用于模板填充）
    pub fn to_shell_vars(&self) -> ConfigVars {
        ConfigVars {
            // SSH 监听地址
            ssh_listen_addr: format!("{}:{}", self.server.ssh.host, self.server.ssh.port),
            ssh_host_key_path: self.paths.ssh_host_key.clone(),
            
            // gRPC 地址
            auth_grpc_address: self.internal.get_auth_grpc_address(),
            gitlayer_address: self.internal.get_gitlayer_address(),
            
            // 内部 API 密钥
            gitfox_shell_secret: self.secrets.internal.clone(),
            
            // 日志
            rust_log: self.logging.level.clone(),
            gitfox_debug: false,

            ..Default::default()
        }
    }

    /// 生成 Shell .env 配置内容（使用模板）
    pub fn to_shell_env_template(&self, template: &str) -> String {
        let vars = self.to_shell_vars();
        vars.fill_template(template)
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 模板处理和配置生成
// ═══════════════════════════════════════════════════════════════════════

/// 配置变量（用于模板替换）
#[derive(Debug, Clone, Default)]
pub struct ConfigVars {
    // 数据库和缓存
    pub database_url: String,
    pub redis_url: String,

    // 安全密钥
    pub jwt_secret: String,
    pub jwt_expiration: u64,
    pub gitfox_shell_secret: String,

    // 服务配置
    pub gitfox_base_url: String,
    pub http_port: u16,
    pub max_upload_size: u64,

    // SSH 配置
    pub ssh_enabled: bool,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_public_host: String,
    pub ssh_public_port: u16,

    // 内部服务
    pub server_connection_type: String,
    pub server_socket_path: String,
    pub server_host: String,
    pub server_port: u16,

    // 路径
    pub git_repos_path: String,
    pub assets_path: String,
    pub frontend_path: String,
    pub webide_path: String,
    pub ssh_host_key_path: String,
    pub gitfox_shell_path: String,

    // 管理员
    pub initial_admin_username: String,
    pub initial_admin_email: String,
    pub initial_admin_password: String,

    // SMTP
    pub smtp_enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from_email: String,
    pub smtp_from_name: String,
    pub smtp_use_tls: bool,
    pub smtp_use_ssl: bool,

    // OAuth
    pub oauth_github_client_id: String,
    pub oauth_github_client_secret: String,
    pub oauth_gitlab_client_id: String,
    pub oauth_gitlab_client_secret: String,
    pub oauth_google_client_id: String,
    pub oauth_google_client_secret: String,

    // WebAuthn
    pub webauthn_rp_name: String,
    pub webauthn_rp_id: String,
    pub webauthn_rp_origin: String,

    // PAT
    pub pat_default_expiration_days: u32,
    pub pat_max_expiration_days: u32,

    // 日志
    pub rust_log: String,

    // 服务启用
    pub services_backend: bool,
    pub services_gitlayer: bool,
    pub services_shell: bool,
    pub services_workhorse: bool,

    // Registry
    pub registry_enabled: bool,
    pub registry_domain: String,
    pub registry_docker_enabled: bool,
    pub registry_npm_enabled: bool,
    pub registry_storage_path: String,
    pub registry_max_size: u64,
    pub registry_jwt_secret: String,

    // Bundled 内置服务
    pub bundled_enabled: bool,
    pub bundled_postgresql_enabled: bool,
    pub bundled_redis_enabled: bool,
    pub bundled_nginx_enabled: bool,

    // Workhorse 专用
    pub listen_addr: String,
    pub listen_port: u16,
    pub backend_socket: String,
    pub backend_url: String,
    pub auth_grpc_address: String,
    pub gitlayer_address: String,
    pub enable_request_logging: bool,
    pub enable_cors: bool,
    pub websocket_timeout: u32,
    pub static_cache_control: String,

    // GitLayer 专用
    pub gitlayer_listen_addr: String,
    pub gitlayer_git_bin: String,
    pub gitlayer_max_concurrent_ops: u32,
    pub gitlayer_enable_cache: bool,
    pub gitlayer_cache_ttl: u32,

    // Shell 专用
    pub ssh_listen_addr: String,
    pub gitfox_debug: bool,
    
    // gRPC 监听地址（给后端用）
    pub grpc_address: String,
    
    // WebIDE
    pub webide_client_id: String,
    pub webide_redirect_uri_path: String,
}

impl ConfigVars {
    /// 将变量填充到模板中
    pub fn fill_template(&self, template: &str) -> String {
        let mut result = template.to_string();

        // 数据库和缓存
        result = result.replace("{{DATABASE_URL}}", &self.database_url);
        result = result.replace("{{REDIS_URL}}", &self.redis_url);

        // 安全密钥
        result = result.replace("{{JWT_SECRET}}", &self.jwt_secret);
        result = result.replace("{{JWT_EXPIRATION}}", &self.jwt_expiration.to_string());
        result = result.replace("{{GITFOX_SHELL_SECRET}}", &self.gitfox_shell_secret);

        // 服务配置
        result = result.replace("{{GITFOX_BASE_URL}}", &self.gitfox_base_url);
        result = result.replace("{{HTTP_PORT}}", &self.http_port.to_string());
        result = result.replace("{{MAX_UPLOAD_SIZE}}", &self.max_upload_size.to_string());

        // SSH 配置
        result = result.replace("{{SSH_ENABLED}}", &self.ssh_enabled.to_string());
        result = result.replace("{{SSH_HOST}}", &self.ssh_host);
        result = result.replace("{{SSH_PORT}}", &self.ssh_port.to_string());
        result = result.replace("{{SSH_PUBLIC_HOST}}", &self.ssh_public_host);
        result = result.replace("{{SSH_PUBLIC_PORT}}", &self.ssh_public_port.to_string());

        // 内部服务
        result = result.replace("{{SERVER_CONNECTION_TYPE}}", &self.server_connection_type);
        result = result.replace("{{SERVER_SOCKET_PATH}}", &self.server_socket_path);
        result = result.replace("{{SERVER_HOST}}", &self.server_host);
        result = result.replace("{{SERVER_PORT}}", &self.server_port.to_string());

        // 路径
        result = result.replace("{{GIT_REPOS_PATH}}", &self.git_repos_path);
        result = result.replace("{{ASSETS_PATH}}", &self.assets_path);
        result = result.replace("{{FRONTEND_PATH}}", &self.frontend_path);
        result = result.replace("{{WEBIDE_PATH}}", &self.webide_path);
        result = result.replace("{{SSH_HOST_KEY_PATH}}", &self.ssh_host_key_path);
        result = result.replace("{{GITFOX_SHELL_PATH}}", &self.gitfox_shell_path);

        // 管理员
        result = result.replace("{{INITIAL_ADMIN_USERNAME}}", &self.initial_admin_username);
        result = result.replace("{{INITIAL_ADMIN_EMAIL}}", &self.initial_admin_email);
        result = result.replace("{{INITIAL_ADMIN_PASSWORD}}", &self.initial_admin_password);

        // SMTP
        result = result.replace("{{SMTP_ENABLED}}", &self.smtp_enabled.to_string());
        result = result.replace("{{SMTP_HOST}}", &self.smtp_host);
        result = result.replace("{{SMTP_PORT}}", &self.smtp_port.to_string());
        result = result.replace("{{SMTP_USERNAME}}", &self.smtp_username);
        result = result.replace("{{SMTP_PASSWORD}}", &self.smtp_password);
        result = result.replace("{{SMTP_FROM_EMAIL}}", &self.smtp_from_email);
        result = result.replace("{{SMTP_FROM_NAME}}", &self.smtp_from_name);
        result = result.replace("{{SMTP_USE_TLS}}", &self.smtp_use_tls.to_string());
        result = result.replace("{{SMTP_USE_SSL}}", &self.smtp_use_ssl.to_string());

        // OAuth
        result = result.replace("{{OAUTH_GITHUB_CLIENT_ID}}", &self.oauth_github_client_id);
        result = result.replace("{{OAUTH_GITHUB_CLIENT_SECRET}}", &self.oauth_github_client_secret);
        result = result.replace("{{OAUTH_GITLAB_CLIENT_ID}}", &self.oauth_gitlab_client_id);
        result = result.replace("{{OAUTH_GITLAB_CLIENT_SECRET}}", &self.oauth_gitlab_client_secret);
        result = result.replace("{{OAUTH_GOOGLE_CLIENT_ID}}", &self.oauth_google_client_id);
        result = result.replace("{{OAUTH_GOOGLE_CLIENT_SECRET}}", &self.oauth_google_client_secret);

        // WebAuthn
        result = result.replace("{{WEBAUTHN_RP_NAME}}", &self.webauthn_rp_name);
        result = result.replace("{{WEBAUTHN_RP_ID}}", &self.webauthn_rp_id);
        result = result.replace("{{WEBAUTHN_RP_ORIGIN}}", &self.webauthn_rp_origin);

        // PAT
        result = result.replace("{{PAT_DEFAULT_EXPIRATION_DAYS}}", &self.pat_default_expiration_days.to_string());
        result = result.replace("{{PAT_MAX_EXPIRATION_DAYS}}", &self.pat_max_expiration_days.to_string());

        // 日志
        result = result.replace("{{RUST_LOG}}", &self.rust_log);

        // 服务启用
        result = result.replace("{{SERVICES_BACKEND}}", &self.services_backend.to_string());
        result = result.replace("{{SERVICES_GITLAYER}}", &self.services_gitlayer.to_string());
        result = result.replace("{{SERVICES_SHELL}}", &self.services_shell.to_string());
        result = result.replace("{{SERVICES_WORKHORSE}}", &self.services_workhorse.to_string());

        // Workhorse 配置 (gitfox.toml)
        result = result.replace("{{WORKHORSE_LISTEN_ADDR}}", &self.listen_addr);
        result = result.replace("{{WORKHORSE_ENABLE_REQUEST_LOGGING}}", &self.enable_request_logging.to_string());
        result = result.replace("{{WORKHORSE_ENABLE_CORS}}", &self.enable_cors.to_string());
        result = result.replace("{{WORKHORSE_WEBSOCKET_TIMEOUT}}", &self.websocket_timeout.to_string());
        result = result.replace("{{WORKHORSE_STATIC_CACHE_CONTROL}}", &self.static_cache_control);

        // Registry
        result = result.replace("{{REGISTRY_ENABLED}}", &self.registry_enabled.to_string());
        result = result.replace("{{REGISTRY_DOMAIN}}", &self.registry_domain);
        result = result.replace("{{REGISTRY_DOCKER_ENABLED}}", &self.registry_docker_enabled.to_string());
        result = result.replace("{{REGISTRY_NPM_ENABLED}}", &self.registry_npm_enabled.to_string());
        result = result.replace("{{REGISTRY_STORAGE_PATH}}", &self.registry_storage_path);
        result = result.replace("{{REGISTRY_MAX_SIZE}}", &self.registry_max_size.to_string());
        result = result.replace("{{REGISTRY_JWT_SECRET}}", &self.registry_jwt_secret);

        // Bundled 内置服务
        result = result.replace("{{BUNDLED_ENABLED}}", &self.bundled_enabled.to_string());
        result = result.replace("{{BUNDLED_POSTGRESQL_ENABLED}}", &self.bundled_postgresql_enabled.to_string());
        result = result.replace("{{BUNDLED_REDIS_ENABLED}}", &self.bundled_redis_enabled.to_string());
        result = result.replace("{{BUNDLED_NGINX_ENABLED}}", &self.bundled_nginx_enabled.to_string());

        // Workhorse 专用
        result = result.replace("{{LISTEN_ADDR}}", &self.listen_addr);
        result = result.replace("{{LISTEN_PORT}}", &self.listen_port.to_string());
        result = result.replace("{{BACKEND_SOCKET}}", &self.backend_socket);
        result = result.replace("{{BACKEND_URL}}", &self.backend_url);
        result = result.replace("{{AUTH_GRPC_ADDRESS}}", &self.auth_grpc_address);
        result = result.replace("{{GITLAYER_ADDRESS}}", &self.gitlayer_address);
        result = result.replace("{{SHELL_SECRET}}", &self.gitfox_shell_secret);
        result = result.replace("{{FRONTEND_DIST_PATH}}", &self.frontend_path);
        result = result.replace("{{WEBIDE_DIST_PATH}}", &self.webide_path);
        result = result.replace("{{ENABLE_REQUEST_LOGGING}}", &self.enable_request_logging.to_string());
        result = result.replace("{{ENABLE_CORS}}", &self.enable_cors.to_string());
        result = result.replace("{{WEBSOCKET_TIMEOUT}}", &self.websocket_timeout.to_string());
        result = result.replace("{{STATIC_CACHE_CONTROL}}", &self.static_cache_control);

        // GitLayer 专用
        result = result.replace("{{GITLAYER_LISTEN_ADDR}}", &self.gitlayer_listen_addr);
        result = result.replace("{{GITLAYER_GIT_BIN}}", &self.gitlayer_git_bin);
        result = result.replace("{{GITLAYER_MAX_CONCURRENT_OPS}}", &self.gitlayer_max_concurrent_ops.to_string());
        result = result.replace("{{GITLAYER_ENABLE_CACHE}}", &self.gitlayer_enable_cache.to_string());
        result = result.replace("{{GITLAYER_CACHE_TTL}}", &self.gitlayer_cache_ttl.to_string());

        // Shell 专用
        result = result.replace("{{SSH_LISTEN_ADDR}}", &self.ssh_listen_addr);
        result = result.replace("{{GITFOX_DEBUG}}", &self.gitfox_debug.to_string());
        
        // gRPC 监听地址（后端/Shell共用）
        result = result.replace("{{GRPC_ADDRESS}}", &self.grpc_address);

        // WebIDE
        result = result.replace("{{WEBIDE_CLIENT_ID}}", &self.webide_client_id);
        result = result.replace("{{WEBIDE_REDIRECT_URI_PATH}}", &self.webide_redirect_uri_path);

        result
    }
}

/// 使用模板文件生成配置
/// template: 模板内容（从嵌入资源加载）
/// vars: 配置变量
pub fn generate_config_template(template: &str, vars: &ConfigVars) -> String {
    vars.fill_template(template)
}

// ═══════════════════════════════════════════════════════════════════════
// 配置迁移 (从 gitfox.env 迁移到 gitfox.toml)
// ═══════════════════════════════════════════════════════════════════════

/// 从 .env 文件解析配置
///
/// 支持的格式：
/// - KEY=value
/// - KEY="value"
/// - KEY='value'
/// - # 注释行
fn parse_env_file(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // 跳过空行和注释
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 查找等号
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let mut value = line[eq_pos + 1..].trim().to_string();

            // 去除引号
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }

            map.insert(key, value);
        }
    }

    map
}

/// 从 gitfox.env 迁移到 GitFoxConfig
///
/// 这个函数读取旧的 .env 格式配置并转换为新的 TOML 结构
pub fn migrate_from_env(env_path: &Path) -> Result<GitFoxConfig> {
    let content = fs::read_to_string(env_path)
        .with_context(|| format!("Failed to read env file: {}", env_path.display()))?;

    let env = parse_env_file(&content);

    // 辅助函数：获取环境变量，带默认值
    let get = |key: &str, default: &str| -> String {
        env.get(key).cloned().unwrap_or_else(|| default.to_string())
    };

    let get_u16 = |key: &str, default: u16| -> u16 {
        env.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };

    let get_u64 = |key: &str, default: u64| -> u64 {
        env.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };

    let get_bool = |key: &str, default: bool| -> bool {
        env.get(key)
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(default)
    };

    // 构建 SSH 配置
    let ssh = SshConfig {
        enabled: get_bool("SSH_ENABLED", true),
        host: get("SSH_HOST", "0.0.0.0"),
        port: get_u16("SSH_PORT", 2222),
        public_host: get("SSH_PUBLIC_HOST", "localhost"),
        public_port: get_u16("SSH_PUBLIC_PORT", 2222),
    };

    // 构建 Internal 配置
    let connection_type = get("SERVER_CONNECTION_TYPE", "tcp");
    let host = get("INTERNAL_HOST", "127.0.0.1");
    let internal = if connection_type == "unix_socket" {
        InternalConfig {
            host: host.clone(),
            backend_socket: Some(get("SERVER_SOCKET_PATH", "/tmp/gitfox-backend.sock")),
            backend_url: env::var("BACKEND_URL").ok(),
            backend_port: 8081,
            gitlayer_address: env::var("GITLAYER_ADDRESS").ok(),
            gitlayer_port: get_u16("GITLAYER_PORT", 50052),
            auth_grpc_address: env::var("AUTH_GRPC_ADDRESS").ok(),
            auth_grpc_port: {
                // 解析 GRPC_ADDRESS 获取端口
                let grpc_addr = get("GRPC_ADDRESS", "127.0.0.1:50051");
                grpc_addr
                    .rsplit(':')
                    .next()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(50051)
            },
        }
    } else {
        InternalConfig {
            host: host.clone(),
            backend_socket: None,
            backend_url: env::var("BACKEND_URL").ok(),
            backend_port: get_u16("SERVER_PORT", 8081),
            gitlayer_address: env::var("GITLAYER_ADDRESS").ok(),
            gitlayer_port: get_u16("GITLAYER_PORT", 50052),
            auth_grpc_address: env::var("AUTH_GRPC_ADDRESS").ok(),
            auth_grpc_port: {
                let grpc_addr = get("GRPC_ADDRESS", "127.0.0.1:50051");
                grpc_addr
                    .rsplit(':')
                    .next()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(50051)
            },
        }
    };

    // 构建配置
    let config = GitFoxConfig {
        version: CONFIG_VERSION.to_string(),

        database: DatabaseConfig {
            url: get("DATABASE_URL", "postgres://postgres:password@localhost:5432/devops"),
        },

        redis: RedisConfig {
            url: get("REDIS_URL", "redis://127.0.0.1:6379"),
        },

        bundled: BundledConfig::default(),

        secrets: SecretsConfig {
            jwt: get("JWT_SECRET", ""),
            internal: get("GITFOX_SHELL_SECRET", ""),
        },

        server: ServerConfig {
            base_url: get("GITFOX_BASE_URL", "http://localhost:8080"),
            http_port: get_u16("HTTP_PORT", 8080),
            max_upload_size: get_u64("MAX_UPLOAD_SIZE", 1073741824), // 1GB
            ssh,
        },

        internal,

        paths: PathsConfig {
            repos: get("GIT_REPOS_PATH", "./repos"),
            frontend: get("FRONTEND_PATH", "./frontend/dist"),
            webide: get("WEBIDE_PATH", "./webide/dist"),
            assets: get("ASSETS_PATH", "./assets"),
            ssh_host_key: get("SSH_HOST_KEY_PATH", "./data/ssh/host_key"),
            shell_binary: get("SHELL_BINARY", ""),
        },

        admin: AdminConfig {
            username: Some(get("INITIAL_ADMIN_USERNAME", "admin")),
            email: Some(get("INITIAL_ADMIN_EMAIL", "admin@localhost")),
            password: Some(get("INITIAL_ADMIN_PASSWORD", "admin123")),
        },

        smtp: SmtpConfig {
            enabled: get_bool("SMTP_ENABLED", false),
            host: get("SMTP_HOST", "smtp.example.com"),
            port: get_u16("SMTP_PORT", 587),
            username: get("SMTP_USERNAME", ""),
            password: get("SMTP_PASSWORD", ""),
            from_email: get("SMTP_FROM_EMAIL", "noreply@example.com"),
            from_name: get("SMTP_FROM_NAME", "GitFox"),
            use_tls: get_bool("SMTP_USE_TLS", true),
            use_ssl: get_bool("SMTP_USE_SSL", false),
        },

        oauth: OAuthConfig {
            github: OAuthProviderConfig {
                client_id: get("OAUTH_GITHUB_CLIENT_ID", ""),
                client_secret: get("OAUTH_GITHUB_CLIENT_SECRET", ""),
                url: None,
                tenant_id: None,
            },
            gitlab: OAuthProviderConfig {
                client_id: get("OAUTH_GITLAB_CLIENT_ID", ""),
                client_secret: get("OAUTH_GITLAB_CLIENT_SECRET", ""),
                url: env.get("OAUTH_GITLAB_URL").cloned(),
                tenant_id: None,
            },
            google: OAuthProviderConfig {
                client_id: get("OAUTH_GOOGLE_CLIENT_ID", ""),
                client_secret: get("OAUTH_GOOGLE_CLIENT_SECRET", ""),
                url: None,
                tenant_id: None,
            },
            azure_ad: OAuthProviderConfig::default(),
            bitbucket: OAuthProviderConfig::default(),
        },

        logging: LoggingConfig {
            level: get("RUST_LOG", "info"),
        },

        registry: RegistryConfig::default(),

        services: ServicesConfig::default(),
    };

    Ok(config)
}

/// 迁移结果
pub struct MigrationResult {
    /// 新配置
    pub config: GitFoxConfig,
    /// 警告信息
    pub warnings: Vec<String>,
    /// 迁移的字段数
    pub migrated_fields: usize,
    /// 迁移的来源文件
    pub sources: Vec<String>,
}

/// Workhorse 配置（旧格式，用于迁移）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct WorkhorseConfig {
    listen_addr: Option<String>,
    listen_port: Option<u16>,
    backend_socket: Option<String>,
    backend_url: Option<String>,
    frontend_dist_path: Option<String>,
    webide_dist_path: Option<String>,
    assets_path: Option<String>,
    max_upload_size: Option<u64>,
    auth_grpc_address: Option<String>,
    gitlayer_address: Option<String>,
}

/// 从 workhorse.toml 解析配置
fn parse_workhorse_toml(path: &Path) -> Result<WorkhorseConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read workhorse config: {}", path.display()))?;
    
    // 先替换模板变量为空（因为旧配置可能还有模板标记）
    let content = content
        .lines()
        .filter(|line| !line.contains("{{"))  // 跳过未替换的模板行
        .collect::<Vec<_>>()
        .join("\n");
    
    toml::from_str(&content)
        .with_context(|| format!("Failed to parse workhorse config: {}", path.display()))
}

/// 从旧版配置迁移（gitfox.env + workhorse.toml）
///
/// 这个函数同时读取 gitfox.env 和 workhorse.toml，合并配置
pub fn migrate_from_legacy(data_dir: &Path) -> Result<MigrationResult> {
    let env_path = data_dir.join("gitfox.env");
    let workhorse_path = data_dir.join("workhorse.toml");
    
    let mut warnings = Vec::new();
    let mut sources = Vec::new();
    let mut migrated_fields = 0;
    
    // 先尝试从 gitfox.env 读取基础配置
    let mut config = if env_path.exists() {
        sources.push(format!("gitfox.env ({})", env_path.display()));
        migrated_fields += 30;
        migrate_from_env(&env_path)?
    } else {
        warnings.push("gitfox.env not found, using defaults".to_string());
        GitFoxConfig {
            version: CONFIG_VERSION.to_string(),
            database: DatabaseConfig {
                url: "postgres://postgres:password@localhost:5432/devops".to_string(),
            },
            redis: RedisConfig::default(),
            secrets: SecretsConfig {
                jwt: String::new(),
                internal: String::new(),
            },
            bundled: BundledConfig::default(),
            server: ServerConfig::default(),
            internal: InternalConfig::default(),
            paths: PathsConfig::default(),
            admin: AdminConfig::default(),
            smtp: SmtpConfig::default(),
            oauth: OAuthConfig::default(),
            logging: LoggingConfig::default(),
            registry: RegistryConfig::default(),
            services: ServicesConfig::default(),
        }
    };
    
    // 然后从 workhorse.toml 补充/覆盖配置
    if workhorse_path.exists() {
        sources.push(format!("workhorse.toml ({})", workhorse_path.display()));
        
        match parse_workhorse_toml(&workhorse_path) {
            Ok(wh) => {
                // HTTP 端口
                if let Some(port) = wh.listen_port {
                    config.server.http_port = port;
                    migrated_fields += 1;
                }
                
                // 后端连接
                if let Some(ref socket) = wh.backend_socket {
                    if !socket.is_empty() && !socket.contains("{{") {
                        config.internal.backend_socket = Some(socket.clone());
                        migrated_fields += 1;
                    }
                }
                if config.internal.backend_socket.is_none() {
                    if let Some(ref url) = wh.backend_url {
                        if !url.contains("{{") {
                            // 从 URL 提取端口
                            if let Some(port_str) = url.rsplit(':').next() {
                                if let Ok(port) = port_str.parse::<u16>() {
                                    config.internal.backend_port = port;
                                    migrated_fields += 1;
                                }
                            }
                        }
                    }
                }
                
                // 静态文件路径
                if let Some(ref path) = wh.frontend_dist_path {
                    if !path.contains("{{") {
                        config.paths.frontend = path.clone();
                        migrated_fields += 1;
                    }
                }
                if let Some(ref path) = wh.webide_dist_path {
                    if !path.contains("{{") {
                        config.paths.webide = path.clone();
                        migrated_fields += 1;
                    }
                }
                if let Some(ref path) = wh.assets_path {
                    if !path.contains("{{") {
                        config.paths.assets = path.clone();
                        migrated_fields += 1;
                    }
                }
                
                // 上传大小
                if let Some(max_size) = wh.max_upload_size {
                    if max_size > 0 {
                        config.server.max_upload_size = max_size;
                        migrated_fields += 1;
                    }
                }
                
                // gRPC 配置
                if let Some(ref addr) = wh.auth_grpc_address {
                    if !addr.contains("{{") {
                        if let Some(port_str) = addr.rsplit(':').next() {
                            if let Ok(port) = port_str.parse::<u16>() {
                                config.internal.auth_grpc_port = port;
                                migrated_fields += 1;
                            }
                        }
                    }
                }
                if let Some(ref addr) = wh.gitlayer_address {
                    if !addr.contains("{{") {
                        if let Some(port_str) = addr.rsplit(':').next() {
                            if let Ok(port) = port_str.parse::<u16>() {
                                config.internal.gitlayer_port = port;
                                migrated_fields += 1;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warnings.push(format!("Failed to parse workhorse.toml: {}", e));
            }
        }
    }
    
    // 检查 secrets 是否为空
    if config.secrets.jwt.is_empty() {
        warnings.push("JWT_SECRET is empty, you need to set it manually".to_string());
    }
    if config.secrets.internal.is_empty() {
        warnings.push("GITFOX_SHELL_SECRET is empty, you need to set it manually".to_string());
    }

    // 检查数据库配置
    if config.database.url.contains("password@") {
        warnings.push("Database password appears to be default, please update it".to_string());
    }
    
    // 检查是否有配置来源
    if sources.is_empty() {
        return Err(anyhow::anyhow!("No configuration files found to migrate"));
    }

    // 从旧版迁移时，检测是否使用了外部服务
    // 如果用户已经配置了外部数据库/Redis，则默认禁用内置服务
    let has_custom_db = !config.database.url.is_empty() 
        && config.database.url != "postgres://postgres:password@localhost:5432/devops"
        && !config.database.url.contains("{{");
    
    let has_custom_redis = !config.redis.url.is_empty()
        && config.redis.url != "redis://localhost:6379"
        && config.redis.url != "redis://127.0.0.1:6379"
        && !config.redis.url.contains("{{");
    
    if has_custom_db || has_custom_redis {
        // 用户已配置外部服务，禁用所有内置服务
        config.bundled.enabled = false;
        config.bundled.postgresql.enabled = false;
        config.bundled.redis.enabled = false;
        config.bundled.nginx.enabled = false;
        
        warnings.push(format!(
            "Detected external services (db: {}, redis: {}), bundled services disabled by default. \
             Edit [bundled] section in gitfox.toml to enable if needed.",
            has_custom_db, has_custom_redis
        ));
    }

    Ok(MigrationResult {
        config,
        warnings,
        migrated_fields,
        sources,
    })
}

/// 执行配置迁移，生成详细报告（兼容旧的 API）
pub fn migrate_with_report(env_path: &Path) -> Result<MigrationResult> {
    // 如果传入的是 gitfox.env 路径，尝试从父目录迁移
    let data_dir = env_path.parent().unwrap_or(Path::new("."));
    migrate_from_legacy(data_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GitFoxConfig {
            version: CONFIG_VERSION.to_string(),
            database: DatabaseConfig {
                url: "postgres://test:test@localhost/gitfox".to_string(),
            },
            redis: RedisConfig::default(),
            secrets: SecretsConfig {
                jwt: "test_jwt_secret_at_least_32_characters_long".to_string(),
                internal: "test_internal_secret_at_least_32_chars".to_string(),
            },
            bundled: BundledConfig::default(),
            server: ServerConfig::default(),
            internal: InternalConfig::default(),
            paths: PathsConfig::default(),
            admin: AdminConfig::default(),
            smtp: SmtpConfig::default(),
            oauth: OAuthConfig::default(),
            logging: LoggingConfig::default(),
            registry: RegistryConfig::default(),
            services: ServicesConfig::default(),
        };

        let warnings = config.validate().unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_port_conflict_detection() {
        let config = GitFoxConfig {
            version: CONFIG_VERSION.to_string(),
            database: DatabaseConfig {
                url: "postgres://test:test@localhost/gitfox".to_string(),
            },
            redis: RedisConfig::default(),
            secrets: SecretsConfig {
                jwt: "test_jwt_secret_at_least_32_characters_long".to_string(),
                internal: "test_internal_secret_at_least_32_chars".to_string(),
            },
            bundled: BundledConfig::default(),
            server: ServerConfig {
                http_port: 8080,
                ..Default::default()
            },
            internal: InternalConfig {
                backend_port: 8080, // 冲突！
                ..Default::default()
            },
            paths: PathsConfig::default(),
            admin: AdminConfig::default(),
            smtp: SmtpConfig::default(),
            oauth: OAuthConfig::default(),
            logging: LoggingConfig::default(),
            registry: RegistryConfig::default(),
            services: ServicesConfig::default(),
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Port conflict"));
    }
}
