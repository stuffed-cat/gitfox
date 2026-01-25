use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiration: i64,
    pub git_repos_path: String,
    /// Secret token for GitFox Shell internal API authentication
    pub shell_secret: String,
    /// Base URL for the GitFox instance (used for LFS, webhooks, etc.)
    pub base_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("Invalid SERVER_PORT"),
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
        }
    }
}

// Type alias for backward compatibility
pub type AppConfig = Config;
