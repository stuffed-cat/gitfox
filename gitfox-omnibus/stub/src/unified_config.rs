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
use std::fs;
use std::path::Path;

/// 配置文件版本
pub const CONFIG_VERSION: &str = "1.0";

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

    /// 日志配置
    #[serde(default)]
    pub logging: LoggingConfig,
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

    /// 组件间通信密钥（gitfox-shell 等内部服务认证）
    pub internal: String,
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            http_port: default_http_port(),
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
    /// 后端 Unix Socket 路径（优先级高于 backend_port）
    /// 推荐使用，性能更好更安全
    #[serde(default)]
    pub backend_socket: Option<String>,

    /// 后端 API 端口（当不使用 Unix Socket 时）
    #[serde(default = "default_backend_port")]
    pub backend_port: u16,

    /// GitLayer gRPC 端口（Git 操作服务）
    #[serde(default = "default_gitlayer_port")]
    pub gitlayer_port: u16,

    /// Auth gRPC 端口（认证服务）
    #[serde(default = "default_auth_grpc_port")]
    pub auth_grpc_port: u16,
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
            backend_socket: None,
            backend_port: default_backend_port(),
            gitlayer_port: default_gitlayer_port(),
            auth_grpc_port: default_auth_grpc_port(),
        }
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
// 配置加载和保存
// ═══════════════════════════════════════════════════════════════════════

impl GitFoxConfig {
    /// 从 TOML 文件加载配置
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        
        // 检查配置版本，如需迁移则处理
        config.check_and_migrate()?;
        
        Ok(config)
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

    /// 检查配置版本并执行迁移
    fn check_and_migrate(&self) -> Result<()> {
        // 当前版本就是最新版本，无需迁移
        if self.version == CONFIG_VERSION {
            return Ok(());
        }

        // 未来版本迁移逻辑在这里添加
        // match self.version.as_str() {
        //     "0.9" => self.migrate_from_0_9(),
        //     _ => {}
        // }

        Ok(())
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

    /// 生成后端（devops）所需的环境变量
    pub fn to_backend_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // 数据库
        env.insert("DATABASE_URL".to_string(), self.database.url.clone());
        env.insert("REDIS_URL".to_string(), self.redis.url.clone());

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
            env.insert("SERVER_HOST".to_string(), "127.0.0.1".to_string());
            env.insert("SERVER_PORT".to_string(), self.internal.backend_port.to_string());
        }

        // SSH
        env.insert("SSH_ENABLED".to_string(), self.server.ssh.enabled.to_string());
        env.insert("SSH_PUBLIC_HOST".to_string(), self.server.ssh.public_host.clone());
        env.insert("SSH_PUBLIC_PORT".to_string(), self.server.ssh.public_port.to_string());

        // 路径
        env.insert("GIT_REPOS_PATH".to_string(), self.paths.repos.clone());
        env.insert("ASSETS_PATH".to_string(), self.paths.assets.clone());

        // gRPC
        env.insert("GRPC_ENABLED".to_string(), "true".to_string());
        env.insert("GRPC_ADDRESS".to_string(), format!("[::1]:{}", self.internal.auth_grpc_port));

        if self.internal.gitlayer_port > 0 {
            env.insert("GITLAYER_ADDRESS".to_string(), format!("http://127.0.0.1:{}", self.internal.gitlayer_port));
        }

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

    /// 生成 Workhorse TOML 配置内容
    pub fn to_workhorse_toml(&self) -> String {
        let mut config = format!(
            r#"# GitFox Workhorse 配置
# 由 gitfox.toml 自动生成，请勿手动修改

listen_addr = "0.0.0.0"
listen_port = {}

"#,
            self.server.http_port
        );

        // 后端连接
        if let Some(ref socket) = self.internal.backend_socket {
            config.push_str(&format!("backend_socket = \"{}\"\n", socket));
        } else {
            config.push_str(&format!("backend_url = \"http://127.0.0.1:{}\"\n", self.internal.backend_port));
        }

        // 路径
        config.push_str(&format!("\nfrontend_dist_path = \"{}\"\n", self.paths.frontend));
        config.push_str(&format!("webide_dist_path = \"{}\"\n", self.paths.webide));
        config.push_str(&format!("assets_path = \"{}\"\n", self.paths.assets));

        // GitLayer
        config.push_str(&format!("\ngitlayer_address = \"http://127.0.0.1:{}\"\n", self.internal.gitlayer_port));
        config.push_str("use_gitlayer = true\n");

        // Auth gRPC
        config.push_str(&format!("\nauth_grpc_address = \"http://[::1]:{}\"\n", self.internal.auth_grpc_port));
        config.push_str("use_grpc_auth = true\n");

        // 内部认证密钥
        config.push_str(&format!("\nshell_secret = \"{}\"\n", self.secrets.internal));

        config
    }

    /// 生成 GitLayer 所需的环境变量
    pub fn to_gitlayer_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        env.insert("GITLAYER_GRPC_ADDRESS".to_string(), format!("0.0.0.0:{}", self.internal.gitlayer_port));
        env.insert("REPO_BASE_PATH".to_string(), self.paths.repos.clone());
        env.insert("RUST_LOG".to_string(), self.logging.level.clone());

        env
    }

    /// 生成 Shell 所需的环境变量
    pub fn to_shell_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        env.insert("SSH_LISTEN_ADDR".to_string(), format!("{}:{}", self.server.ssh.host, self.server.ssh.port));
        env.insert("SSH_HOST_KEY_PATH".to_string(), self.paths.ssh_host_key.clone());
        env.insert("GITLAYER_GRPC_ADDRESS".to_string(), format!("http://127.0.0.1:{}", self.internal.gitlayer_port));
        env.insert("GITFOX_SHELL_SECRET".to_string(), self.secrets.internal.clone());

        // Auth gRPC (用于权限验证)
        env.insert("AUTH_GRPC_ADDRESS".to_string(), format!("http://[::1]:{}", self.internal.auth_grpc_port));

        env.insert("RUST_LOG".to_string(), self.logging.level.clone());

        env
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
    pub webauthn_rp_id: String,
    pub webauthn_rp_origin: String,

    // 日志
    pub rust_log: String,
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
        result = result.replace("{{WEBAUTHN_RP_ID}}", &self.webauthn_rp_id);
        result = result.replace("{{WEBAUTHN_RP_ORIGIN}}", &self.webauthn_rp_origin);

        // 日志
        result = result.replace("{{RUST_LOG}}", &self.rust_log);

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
    let internal = if connection_type == "unix_socket" {
        InternalConfig {
            backend_socket: Some(get("SERVER_SOCKET_PATH", "/tmp/gitfox-backend.sock")),
            backend_port: 8081,
            gitlayer_port: get_u16("GITLAYER_PORT", 50052),
            auth_grpc_port: {
                // 解析 GRPC_ADDRESS 获取端口
                let grpc_addr = get("GRPC_ADDRESS", "[::1]:50051");
                grpc_addr
                    .rsplit(':')
                    .next()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(50051)
            },
        }
    } else {
        InternalConfig {
            backend_socket: None,
            backend_port: get_u16("SERVER_PORT", 8081),
            gitlayer_port: get_u16("GITLAYER_PORT", 50052),
            auth_grpc_port: {
                let grpc_addr = get("GRPC_ADDRESS", "[::1]:50051");
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

        secrets: SecretsConfig {
            jwt: get("JWT_SECRET", ""),
            internal: get("GITFOX_SHELL_SECRET", ""),
        },

        server: ServerConfig {
            base_url: get("GITFOX_BASE_URL", "http://localhost:8080"),
            http_port: get_u16("HTTP_PORT", 8080),
            max_upload_size: get_u64("MAX_UPLOAD_SIZE", 1073741824),
            ssh,
        },

        internal,

        paths: PathsConfig {
            repos: get("GIT_REPOS_PATH", "./repos"),
            frontend: get("FRONTEND_PATH", "./frontend/dist"),
            webide: get("WEBIDE_PATH", "./webide/dist"),
            assets: get("ASSETS_PATH", "./assets"),
            ssh_host_key: get("SSH_HOST_KEY_PATH", "./data/ssh/host_key"),
        },

        admin: AdminConfig {
            username: get("INITIAL_ADMIN_USERNAME", "admin"),
            email: get("INITIAL_ADMIN_EMAIL", "admin@localhost"),
            password: get("INITIAL_ADMIN_PASSWORD", "admin123"),
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
            },
            gitlab: GitLabOAuthConfig {
                client_id: get("OAUTH_GITLAB_CLIENT_ID", ""),
                client_secret: get("OAUTH_GITLAB_CLIENT_SECRET", ""),
                url: env.get("OAUTH_GITLAB_URL").cloned(),
            },
            google: OAuthProviderConfig {
                client_id: get("OAUTH_GOOGLE_CLIENT_ID", ""),
                client_secret: get("OAUTH_GOOGLE_CLIENT_SECRET", ""),
            },
        },

        logging: LoggingConfig {
            level: get("RUST_LOG", "info"),
        },
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
    use_grpc_auth: Option<bool>,
    use_gitlayer: Option<bool>,
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
            server: ServerConfig::default(),
            internal: InternalConfig::default(),
            paths: PathsConfig::default(),
            admin: AdminConfig::default(),
            smtp: SmtpConfig::default(),
            oauth: OAuthConfig::default(),
            logging: LoggingConfig::default(),
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
                if let Some(size) = wh.max_upload_size {
                    config.server.max_upload_size = size;
                    migrated_fields += 1;
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
            server: ServerConfig::default(),
            internal: InternalConfig::default(),
            paths: PathsConfig::default(),
            admin: AdminConfig::default(),
            smtp: SmtpConfig::default(),
            oauth: OAuthConfig::default(),
            logging: LoggingConfig::default(),
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
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Port conflict"));
    }
}
