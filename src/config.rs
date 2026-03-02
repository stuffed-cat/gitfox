use std::env;

/// SMTP configuration for sending emails
#[derive(Clone, Debug, Default)]
pub struct SmtpConfig {
    /// SMTP server host
    pub host: Option<String>,
    /// SMTP server port (default: 587 for TLS, 465 for SSL, 25 for plain)
    pub port: u16,
    /// SMTP username for authentication
    pub username: Option<String>,
    /// SMTP password for authentication
    pub password: Option<String>,
    /// Sender email address (From header)
    pub from_email: String,
    /// Sender display name
    pub from_name: String,
    /// Use TLS encryption (STARTTLS)
    pub use_tls: bool,
    /// Use SSL encryption (implicit TLS)
    pub use_ssl: bool,
    /// Enable SMTP (if false, emails won't be sent)
    pub enabled: bool,
}

impl SmtpConfig {
    pub fn from_env() -> Self {
        Self {
            host: env::var("SMTP_HOST").ok(),
            port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            username: env::var("SMTP_USERNAME").ok(),
            password: env::var("SMTP_PASSWORD").ok(),
            from_email: env::var("SMTP_FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@gitfox.local".to_string()),
            from_name: env::var("SMTP_FROM_NAME")
                .unwrap_or_else(|_| "GitFox".to_string()),
            use_tls: env::var("SMTP_USE_TLS")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            use_ssl: env::var("SMTP_USE_SSL")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
            enabled: env::var("SMTP_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
        }
    }

    /// Check if SMTP is properly configured and enabled
    pub fn is_configured(&self) -> bool {
        self.enabled && self.host.is_some()
    }
}

/// OAuth provider configuration
#[derive(Clone, Debug, Default)]
pub struct OAuthProviderConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    /// Custom base URL (for self-hosted GitLab, etc.)
    pub base_url: Option<String>,
    /// Azure AD tenant ID
    pub tenant_id: Option<String>,
}

impl OAuthProviderConfig {
    pub fn is_enabled(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }
}

/// All OAuth providers configuration
#[derive(Clone, Debug, Default)]
pub struct OAuthConfig {
    pub github: OAuthProviderConfig,
    pub gitlab: OAuthProviderConfig,
    pub google: OAuthProviderConfig,
    pub azure_ad: OAuthProviderConfig,
    pub bitbucket: OAuthProviderConfig,
}

impl OAuthConfig {
    pub fn from_env() -> Self {
        Self {
            github: OAuthProviderConfig {
                client_id: env::var("OAUTH_GITHUB_CLIENT_ID").ok(),
                client_secret: env::var("OAUTH_GITHUB_CLIENT_SECRET").ok(),
                base_url: None,
                tenant_id: None,
            },
            gitlab: OAuthProviderConfig {
                client_id: env::var("OAUTH_GITLAB_CLIENT_ID").ok(),
                client_secret: env::var("OAUTH_GITLAB_CLIENT_SECRET").ok(),
                base_url: env::var("OAUTH_GITLAB_URL").ok(),
                tenant_id: None,
            },
            google: OAuthProviderConfig {
                client_id: env::var("OAUTH_GOOGLE_CLIENT_ID").ok(),
                client_secret: env::var("OAUTH_GOOGLE_CLIENT_SECRET").ok(),
                base_url: None,
                tenant_id: None,
            },
            azure_ad: OAuthProviderConfig {
                client_id: env::var("OAUTH_AZURE_CLIENT_ID").ok(),
                client_secret: env::var("OAUTH_AZURE_CLIENT_SECRET").ok(),
                base_url: None,
                tenant_id: env::var("OAUTH_AZURE_TENANT_ID").ok(),
            },
            bitbucket: OAuthProviderConfig {
                client_id: env::var("OAUTH_BITBUCKET_CLIENT_ID").ok(),
                client_secret: env::var("OAUTH_BITBUCKET_CLIENT_SECRET").ok(),
                base_url: None,
                tenant_id: None,
            },
        }
    }

    /// Get list of enabled OAuth providers
    pub fn enabled_providers(&self) -> Vec<&'static str> {
        let mut providers = Vec::new();
        if self.github.is_enabled() { providers.push("github"); }
        if self.gitlab.is_enabled() { providers.push("gitlab"); }
        if self.google.is_enabled() { providers.push("google"); }
        if self.azure_ad.is_enabled() { providers.push("azure_ad"); }
        if self.bitbucket.is_enabled() { providers.push("bitbucket"); }
        providers
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    /// Unix socket path for backend (优先级高于 TCP)
    pub server_socket_path: Option<String>,
    pub assets_path: String,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiration: i64,
    pub git_repos_path: String,
    /// Secret token for GitFox Shell internal API authentication
    pub shell_secret: String,
    /// Base URL for the GitFox instance (used for LFS, webhooks, etc.)
    pub base_url: String,
    /// Internal API URL for gitfox-shell (defaults to http://127.0.0.1:<port>)
    pub internal_api_url: String,
    /// SSH server enabled
    pub ssh_enabled: bool,
    /// SSH server bind host (what the server listens on)
    pub ssh_host: String,
    /// SSH server bind port (what the server listens on)
    pub ssh_port: u16,
    /// SSH public host (for external access, e.g., git@gitfox.example.com)
    pub ssh_public_host: String,
    /// SSH public port (for external access, e.g., 22 or 2222)
    pub ssh_public_port: u16,
    /// SSH host key path (without extension)
    pub ssh_host_key_path: String,
    /// Path to gitfox-shell binary
    pub gitfox_shell_path: String,
    /// Initial admin username (only used on first startup when no admin exists)
    pub initial_admin_username: Option<String>,
    /// Initial admin email
    pub initial_admin_email: Option<String>,
    /// Initial admin password
    pub initial_admin_password: Option<String>,
    /// OAuth configuration for external providers
    pub oauth: OAuthConfig,
    /// PAT default expiration in days (0 = no expiration)
    pub pat_default_expiration_days: u32,
    /// PAT maximum expiration in days (0 = no limit)
    pub pat_max_expiration_days: u32,
    /// SMTP configuration for email sending
    pub smtp: SmtpConfig,
    /// WebAuthn RP Name (Relying Party Display Name)
    pub webauthn_rp_name: String,
    /// WebAuthn RP ID (domain without protocol/port, e.g., "example.com")
    pub webauthn_rp_id: String,
    /// WebAuthn Origin (full URL including protocol, e.g., "https://example.com")
    pub webauthn_origin: String,
    /// Instance identifier for multi-instance deployment (hostname:pid)
    pub instance_id: String,
    /// WebIDE OAuth2 client ID (固定值，用于识别 WebIDE 应用)
    pub webide_client_id: String,
    /// WebIDE OAuth2 redirect URI path
    pub webide_redirect_uri_path: String,
    /// Maximum upload size in bytes (default: 1GB for Git operations)
    pub max_upload_size: usize,
}

impl Config {
    pub fn from_env() -> Self {
        // Generate unique instance identifier
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
        let pid = std::process::id();
        let instance_id = format!("{}:{}", hostname, pid);
        
        // 检查连接类型
        let connection_type = env::var("SERVER_CONNECTION_TYPE")
            .unwrap_or_else(|_| "tcp".to_string());
        let use_unix_socket = connection_type == "unix_socket";
        
        // 根据连接类型解析配置
        let (server_host, server_port, server_socket_path) = if use_unix_socket {
            // Unix Socket 模式：socket path 必须存在，host/port 使用默认值
            let socket_path = env::var("SERVER_SOCKET_PATH")
                .expect("SERVER_SOCKET_PATH must be set when SERVER_CONNECTION_TYPE=unix_socket");
            ("127.0.0.1".to_string(), 8081, Some(socket_path))
        } else {
            // TCP 模式：host/port 必须有效
            let host = env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
            let port = env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("Invalid SERVER_PORT");
            (host, port, None)
        };
        
        Self {
            server_host,
            server_port,
            server_socket_path,
            assets_path: env::var("ASSETS_PATH").unwrap_or_else(|_| "./assets".to_string()),
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-super-secret-jwt-key".to_string()),
            jwt_expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()
                .expect("Invalid JWT_EXPIRATION"),
            git_repos_path: env::var("GIT_REPOS_PATH")
                .unwrap_or_else(|_| "./repos".to_string()),
            shell_secret: env::var("GITFOX_SHELL_SECRET")
                .unwrap_or_else(|_| env::var("GITFOX_API_SECRET")
                    .unwrap_or_else(|_| "change-me-in-production".to_string())),
            base_url: env::var("GITFOX_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            internal_api_url: env::var("GITFOX_INTERNAL_API_URL")
                .unwrap_or_else(|_| {
                    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
                    format!("http://127.0.0.1:{}", port)
                }),
            ssh_enabled: env::var("SSH_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            ssh_host: env::var("SSH_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            ssh_port: env::var("SSH_PORT")
                .unwrap_or_else(|_| "2222".to_string())
                .parse()
                .expect("Invalid SSH_PORT"),
            ssh_public_host: env::var("SSH_PUBLIC_HOST")
                .unwrap_or_else(|_| env::var("SSH_HOST")
                    .unwrap_or_else(|_| "localhost".to_string())),
            ssh_public_port: env::var("SSH_PUBLIC_PORT")
                .unwrap_or_else(|_| env::var("SSH_PORT")
                    .unwrap_or_else(|_| "2222".to_string()))
                .parse()
                .expect("Invalid SSH_PUBLIC_PORT"),
            ssh_host_key_path: env::var("SSH_HOST_KEY_PATH")
                .unwrap_or_else(|_| "./data/ssh/host_key".to_string()),
            gitfox_shell_path: env::var("GITFOX_SHELL_PATH")
                .unwrap_or_else(|_| "./gitfox-shell/target/debug/gitfox-shell".to_string()),
            initial_admin_username: env::var("INITIAL_ADMIN_USERNAME").ok(),
            initial_admin_email: env::var("INITIAL_ADMIN_EMAIL").ok(),
            initial_admin_password: env::var("INITIAL_ADMIN_PASSWORD").ok(),
            oauth: OAuthConfig::from_env(),
            pat_default_expiration_days: env::var("PAT_DEFAULT_EXPIRATION_DAYS")
                .unwrap_or_else(|_| "365".to_string())
                .parse()
                .unwrap_or(365),
            pat_max_expiration_days: env::var("PAT_MAX_EXPIRATION_DAYS")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
            smtp: SmtpConfig::from_env(),
            webauthn_rp_name: env::var("WEBAUTHN_RP_NAME")
                .unwrap_or_else(|_| "GitFox".to_string()),
            webauthn_rp_id: env::var("WEBAUTHN_RP_ID")
                .unwrap_or_else(|_| {
                    // Extract domain from base_url
                    let base = env::var("GITFOX_BASE_URL")
                        .unwrap_or_else(|_| "http://localhost:8080".to_string());
                    base.split("://")
                        .nth(1)
                        .and_then(|s| s.split(':').next())
                        .unwrap_or("localhost")
                        .to_string()
                }),
            webauthn_origin: env::var("WEBAUTHN_ORIGIN")
                .unwrap_or_else(|_| env::var("GITFOX_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:8080".to_string())),
            instance_id,
            webide_client_id: env::var("WEBIDE_CLIENT_ID")
                .unwrap_or("gitfox-webide".to_string()),
            webide_redirect_uri_path: env::var("WEBIDE_REDIRECT_URI_PATH")
                .unwrap_or("/-/ide/oauth/callback".to_string()),
            max_upload_size: env::var("MAX_UPLOAD_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1024 * 1024 * 1024), // Default: 1GB
        }
    }
}

// Type alias for backward compatibility
pub type AppConfig = Config;
