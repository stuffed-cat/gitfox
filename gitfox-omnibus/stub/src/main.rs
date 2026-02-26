//! GitFox - Self-contained DevOps Platform
//!
//! 这是 GitFox Omnibus 生成的超级二进制。
//! 包含所有组件，运行时自动解压并启动。
//!
//! 注意：这个文件中的 `$EMBED_*` 占位符会在 rust-embed 编译时被替换。
//! 开发时可以创建空目录来测试编译。

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use rand::Rng;
use rust_embed::RustEmbed;
use std::env;
use std::fs::{self, File, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal::unix::{signal as unix_signal, SignalKind};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// ============================================================================
// 嵌入的资源
// ============================================================================

/// 前端静态文件 (Vue SPA)
#[derive(RustEmbed)]
#[folder = "embedded/frontend"]
#[prefix = ""]
struct FrontendAssets;

/// WebIDE 静态文件 (VS Code Web)
#[derive(RustEmbed)]
#[folder = "embedded/webide"]
#[prefix = ""]
struct WebideAssets;

/// 编译好的二进制文件 (devops, workhorse, shell)
#[derive(RustEmbed)]
#[folder = "embedded/bin"]
#[prefix = ""]
struct BinaryAssets;

/// 数据库迁移文件
#[derive(RustEmbed)]
#[folder = "embedded/migrations"]
#[prefix = ""]
struct MigrationAssets;

// ============================================================================
// CLI
// ============================================================================

#[derive(Parser)]
#[command(name = "gitfox")]
#[command(version)]
#[command(about = "GitFox - Self-contained DevOps Platform")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// 数据目录 (存放解压的资源、配置文件、Git 仓库等)
    #[arg(long, env = "GITFOX_DATA_DIR", default_value = "/var/lib/gitfox")]
    data_dir: PathBuf,

    /// 启用调试日志
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone, Subcommand)]
enum Commands {
    /// 初始化配置 (生成配置文件模板)
    Init,

    /// 启动所有服务 (默认)
    Start,

    /// 只运行数据库迁移
    Migrate,

    /// 解压嵌入的资源到数据目录
    Extract,

    /// 列出嵌入的资源
    List,

    /// 显示版本信息
    Version,
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    init_logging(cli.verbose);

    let command = cli.command.clone().unwrap_or(Commands::Start);

    match command {
        Commands::Init => {
            init_config(&cli.data_dir)?;
        }
        Commands::Start => {
            start_services(&cli).await?;
        }
        Commands::Migrate => {
            run_migrations(&cli).await?;
        }
        Commands::Extract => {
            extract_assets(&cli.data_dir)?;
            info!("Assets extracted to: {}", cli.data_dir.display());
        }
        Commands::List => {
            list_assets();
        }
        Commands::Version => {
            println!("GitFox {}", env!("CARGO_PKG_VERSION"));
            println!("Built with GitFox Omnibus");
        }
    }

    Ok(())
}

fn init_logging(verbose: bool) {
    let log_level = if verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("gitfox={},actix_web=info", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

// ============================================================================
// 服务启动
// ============================================================================

async fn start_services(cli: &Cli) -> Result<()> {
    info!("GitFox starting...");

    // 加载配置文件
    let env_file = cli.data_dir.join("gitfox.env");
    let workhorse_config = cli.data_dir.join("workhorse.toml");
    
    if !env_file.exists() {
        eprintln!("❌ Configuration file not found: {}", env_file.display());
        eprintln!("\nPlease run: gitfox init");
        eprintln!("Then edit {} with your settings", env_file.display());
        return Err(anyhow::anyhow!("Configuration file missing"));
    }
    
    info!("Loading configuration from: {}", env_file.display());
    load_env_file(&env_file)?;

    // 验证必需的配置
    let database_url = env::var("DATABASE_URL")
        .context("DATABASE_URL not set in gitfox.env")?;
    
    let redis_url = env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    
    // HTTP 端口：优先使用环境变量，其次读取 workhorse.toml，最后默认 8080
    let http_port: u16 = env::var("GITFOX_PORT")
        .ok()
        .or_else(|| {
            if workhorse_config.exists() {
                fs::read_to_string(&workhorse_config)
                    .ok()
                    .and_then(|content| toml::from_str::<toml::Value>(&content).ok())
                    .and_then(|config| config.get("listen_port")?.as_integer().map(|v| v.to_string()))
            } else {
                None
            }
        })
        .unwrap_or_else(|| "8080".to_string())
        .parse()
        .context("Invalid GITFOX_PORT")?;

    // 解压资源到 data_dir
    extract_assets(&cli.data_dir)?;

    // 准备目录路径
    let paths = ServicePaths::new(&cli.data_dir);
    paths.ensure_dirs()?;

    // 后端内部端口 (从配置读取或默认)
    let backend_port: u16 = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()
        .unwrap_or(8081);

    // 启动后端
    info!("Starting backend on internal port {}...", backend_port);
    let mut backend = start_backend_from_env(&paths, database_url, redis_url, cli.verbose)?;

    // 等待后端启动
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 启动 Workhorse
    info!("Starting workhorse on port {}...", http_port);
    let mut workhorse = if workhorse_config.exists() {
        start_workhorse_from_config(&paths, &workhorse_config, cli.verbose)?
    } else {
        // 使用默认配置
        warn!("workhorse.toml not found, using defaults");
        start_workhorse(&WorkhorseConfig {
            binary: paths.bin_dir.join("gitfox-workhorse"),
            listen_port: http_port,
            backend_port,
            frontend_dir: paths.frontend_dir.clone(),
            webide_dir: paths.webide_dir.clone(),
            assets_dir: paths.assets_dir.clone(),
            repos_dir: paths.repos_dir.clone(),
            verbose: cli.verbose,
        })?
    };

    let ssh_port = env::var("SSH_PORT")
        .or_else(|_| env::var("SSH_PUBLIC_PORT"))
        .unwrap_or_else(|_| "22".to_string());

    info!("GitFox is running!");
    info!("  HTTP:  http://0.0.0.0:{}", http_port);
    info!("  SSH:   ssh -p {} git@localhost", ssh_port);

    // 等待关闭信号
    let shutdown = Arc::new(AtomicBool::new(false));
    wait_for_shutdown(shutdown.clone()).await;

    // 优雅关闭服务
    info!("Shutting down...");
    shutdown_process(&mut workhorse, "workhorse");
    shutdown_process(&mut backend, "backend");

    info!("GitFox stopped");
    Ok(())
}

// ============================================================================
// 服务路径
// ============================================================================

struct ServicePaths {
    bin_dir: PathBuf,
    frontend_dir: PathBuf,
    webide_dir: PathBuf,
    migrations_dir: PathBuf,
    assets_dir: PathBuf,
    repos_dir: PathBuf,
}

impl ServicePaths {
    fn new(data_dir: &Path) -> Self {
        Self {
            bin_dir: data_dir.join("bin"),
            frontend_dir: data_dir.join("frontend"),
            webide_dir: data_dir.join("webide"),
            migrations_dir: data_dir.join("migrations"),
            assets_dir: data_dir.join("assets"),
            repos_dir: data_dir.join("repos"),
        }
    }

    fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.assets_dir)?;
        fs::create_dir_all(&self.repos_dir)?;
        Ok(())
    }
}

// ============================================================================
// 后端配置与启动
// ============================================================================

struct BackendConfig {
    binary: PathBuf,
    database_url: String,
    redis_url: String,
    jwt_secret: Option<String>,
    port: u16,
    ssh_port: u16,
    repos_dir: PathBuf,
    assets_dir: PathBuf,
    migrations_dir: PathBuf,
    verbose: bool,
}

fn start_backend(config: &BackendConfig) -> Result<Child> {
    let jwt = config
        .jwt_secret
        .as_deref()
        .unwrap_or("gitfox-default-secret-change-me");

    let child = Command::new(&config.binary)
        .env("DATABASE_URL", &config.database_url)
        .env("REDIS_URL", &config.redis_url)
        .env("JWT_SECRET", jwt)
        .env("HTTP_PORT", config.port.to_string())
        .env("SSH_PORT", config.ssh_port.to_string())
        .env("GIT_REPOS_PATH", &config.repos_dir)
        .env("ASSETS_PATH", &config.assets_dir)
        .env("MIGRATIONS_PATH", &config.migrations_dir)
        .env("RUST_LOG", if config.verbose { "debug" } else { "info" })
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start backend")?;

    Ok(child)
}

/// 从环境变量启动后端（已通过 load_env_file 设置）
fn start_backend_from_env(
    paths: &ServicePaths,
    database_url: String,
    redis_url: String,
    verbose: bool,
) -> Result<Child> {
    let binary = paths.bin_dir.join("devops");
    
    // 打印准备启动的环境变量（调试用）
    if verbose {
        info!("Backend environment:");
        if let Ok(v) = env::var("SERVER_PORT") {
            info!("  SERVER_PORT={}", v);
        }
        if let Ok(v) = env::var("SSH_PORT") {
            info!("  SSH_PORT={}", v);
        }
    }
    
    // 后端直接从环境变量读取所有配置，我们只需要启动并继承环境
    let child = Command::new(&binary)
        .env("DATABASE_URL", database_url)
        .env("REDIS_URL", redis_url)
        .env("GIT_REPOS_PATH", &paths.repos_dir)
        .env("RUST_LOG", if verbose { "debug" } else { "info" })
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start backend")?;

    Ok(child)
}

// ============================================================================
// Workhorse 配置与启动
// ============================================================================

struct WorkhorseConfig {
    binary: PathBuf,
    listen_port: u16,
    backend_port: u16,
    frontend_dir: PathBuf,
    webide_dir: PathBuf,
    assets_dir: PathBuf,
    repos_dir: PathBuf,
    verbose: bool,
}

fn start_workhorse(config: &WorkhorseConfig) -> Result<Child> {
    let child = Command::new(&config.binary)
        .env("WORKHORSE_LISTEN_ADDR", "0.0.0.0")
        .env("WORKHORSE_LISTEN_PORT", config.listen_port.to_string())
        .env(
            "WORKHORSE_BACKEND_URL",
            format!("http://127.0.0.1:{}", config.backend_port),
        )
        .env("WORKHORSE_FRONTEND_DIST", &config.frontend_dir)
        .env("WORKHORSE_WEBIDE_DIST", &config.webide_dir)
        .env("WORKHORSE_ASSETS_PATH", &config.assets_dir)
        .env("WORKHORSE_GIT_REPOS_PATH", &config.repos_dir)
        .env("RUST_LOG", if config.verbose { "debug" } else { "info" })
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start workhorse")?;

    Ok(child)
}

/// 从 TOML 配置文件启动 Workhorse
fn start_workhorse_from_config(
    paths: &ServicePaths,
    config_path: &Path,
    verbose: bool,
) -> Result<Child> {
    let binary = paths.bin_dir.join("gitfox-workhorse");
    
    // gitfox-workhorse 支持 --config 参数直接读取配置文件
    let child = Command::new(&binary)
        .arg("--config")
        .arg(config_path)
        .env("RUST_LOG", if verbose { "debug" } else { "info" })
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start workhorse")?;

    Ok(child)
}

// ============================================================================
// 进程管理
// ============================================================================

fn shutdown_process(child: &mut Child, name: &str) {
    // 发送 SIGTERM
    let pid = Pid::from_raw(child.id() as i32);
    if let Err(e) = signal::kill(pid, Signal::SIGTERM) {
        warn!("Failed to send SIGTERM to {}: {}", name, e);
    }

    // 等待退出
    std::thread::sleep(std::time::Duration::from_secs(3));

    // 如果还在运行，强制杀死
    if let Ok(None) = child.try_wait() {
        warn!("Force killing {}", name);
        let _ = child.kill();
    }

    let _ = child.wait();
}

async fn wait_for_shutdown(shutdown: Arc<AtomicBool>) {
    let mut sigterm = unix_signal(SignalKind::terminate()).expect("Failed to register SIGTERM");
    let mut sigint = unix_signal(SignalKind::interrupt()).expect("Failed to register SIGINT");

    tokio::select! {
        _ = sigterm.recv() => {
            info!("Received SIGTERM");
        }
        _ = sigint.recv() => {
            info!("Received SIGINT");
        }
    }

    shutdown.store(true, Ordering::SeqCst);
}

// ============================================================================
// 资源解压
// ============================================================================

fn extract_assets(data_dir: &Path) -> Result<()> {
    info!("Extracting embedded assets to: {}", data_dir.display());

    fs::create_dir_all(data_dir)?;

    // 解压各类资源
    extract_embedded::<BinaryAssets>(&data_dir.join("bin"), true)?;
    extract_embedded::<FrontendAssets>(&data_dir.join("frontend"), false)?;
    extract_embedded::<WebideAssets>(&data_dir.join("webide"), false)?;
    extract_embedded::<MigrationAssets>(&data_dir.join("migrations"), false)?;

    info!("Assets extracted successfully");
    Ok(())
}

fn extract_embedded<E: RustEmbed>(output_dir: &Path, executable: bool) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    for file_name in E::iter() {
        let file_name = file_name.as_ref();
        if let Some(content) = E::get(file_name) {
            let dest_path = output_dir.join(file_name);

            // 创建父目录
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // 写入文件
            let mut file = File::create(&dest_path)?;
            file.write_all(&content.data)?;

            // 设置权限
            if executable {
                fs::set_permissions(&dest_path, Permissions::from_mode(0o755))?;
            }
        }
    }

    Ok(())
}

// ============================================================================
// 迁移
// ============================================================================

async fn run_migrations(cli: &Cli) -> Result<()> {
    // 加载配置文件
    let env_file = cli.data_dir.join("gitfox.env");
    if env_file.exists() {
        load_env_file(&env_file)?;
    }
    
    let database_url = env::var("DATABASE_URL")
        .context("DATABASE_URL not set. Please run 'gitfox init' first")?;

    info!("Running migrations...");
    info!("Database: {}", database_url.split('@').last().unwrap_or("<hidden>"));

    // 解压迁移文件
    let migrations_dir = cli.data_dir.join("migrations");
    extract_embedded::<MigrationAssets>(&migrations_dir, false)?;

    // 列出要执行的迁移
    let mut migrations: Vec<_> = MigrationAssets::iter().collect();
    migrations.sort();

    for name in &migrations {
        info!("Migration: {}", name);
    }

    info!("Note: Migrations are run automatically by the backend on startup");
    info!("Migration files extracted to: {}", migrations_dir.display());

    Ok(())
}

// ============================================================================
// 资源列表
// ============================================================================

fn list_assets() {
    println!("=== Embedded Assets ===\n");

    println!("Binaries:");
    for name in BinaryAssets::iter() {
        if let Some(content) = BinaryAssets::get(name.as_ref()) {
            println!("  {} ({} bytes)", name, content.data.len());
        }
    }

    println!("\nFrontend: {} files", FrontendAssets::iter().count());
    list_first_n::<FrontendAssets>(5);

    println!("\nWebIDE: {} files", WebideAssets::iter().count());
    list_first_n::<WebideAssets>(5);

    println!("\nMigrations:");
    let mut migrations: Vec<_> = MigrationAssets::iter().collect();
    migrations.sort();
    for name in &migrations {
        println!("  {}", name);
    }
}

fn list_first_n<E: RustEmbed>(n: usize) {
    for name in E::iter().take(n) {
        println!("  {}", name);
    }
    let count = E::iter().count();
    if count > n {
        println!("  ... and {} more", count - n);
    }
}

// ============================================================================
// 配置初始化
// ============================================================================

fn init_config(data_dir: &Path) -> Result<()> {
    fs::create_dir_all(data_dir)?;
    
    let gitfox_env = data_dir.join("gitfox.env");
    let workhorse_toml = data_dir.join("workhorse.toml");
    
    // 生成随机密钥和密码
    let jwt_secret = generate_random_secret(64);
    let shell_secret = generate_random_secret(64);
    let admin_username = "admin".to_string();
    let admin_email = "admin@localhost".to_string();
    let admin_password = generate_random_password(16);
    
    let secrets = GeneratedSecrets {
        jwt_secret,
        shell_secret,
        admin_username: admin_username.clone(),
        admin_email: admin_email.clone(),
        admin_password: admin_password.clone(),
    };
    
    // 生成 gitfox.env 配置
    if gitfox_env.exists() {
        warn!("{} already exists, skipping", gitfox_env.display());
    } else {
        let env_template = generate_gitfox_env_template(data_dir, &secrets);
        fs::write(&gitfox_env, env_template)?;
        info!("Created: {}", gitfox_env.display());
    }
    
    // 生成 workhorse.toml 配置（自动协调内部通信）
    if workhorse_toml.exists() {
        warn!("{} already exists, skipping", workhorse_toml.display());
    } else {
        let toml_template = generate_workhorse_toml_template(data_dir);
        fs::write(&workhorse_toml, toml_template)?;
        info!("Created: {}", workhorse_toml.display());
    }
    
    // 打印配置信息
    println!("\n✅ Configuration files created!");
    println!("\n{}", "=".repeat(60));
    println!("  Initial Admin User (auto-generated)");
    println!("{}", "=".repeat(60));
    println!("  Username: {}", admin_username);
    println!("  Email:    {}", admin_email);
    println!("  Password: {}", admin_password);
    println!("{}", "=".repeat(60));
    println!("\n⚠️  Please save this password! It will be used on first startup.");
    println!("\nNext steps:");
    println!("1. Edit {} with your PostgreSQL and Redis settings", gitfox_env.display());
    println!("2. Configure SMTP, OAuth if needed (optional)");
    println!("3. Run: gitfox start");
    
    Ok(())
}

struct GeneratedSecrets {
    jwt_secret: String,
    shell_secret: String,
    admin_username: String,
    admin_email: String,
    admin_password: String,
}

/// 生成随机密钥（十六进制）
fn generate_random_secret(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| format!("{:02x}", rng.gen::<u8>()))
        .collect()
}

/// 生成随机密码（字母数字）
fn generate_random_password(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

fn generate_gitfox_env_template(data_dir: &Path, secrets: &GeneratedSecrets) -> String {
    let repos_dir = data_dir.join("repos");
    let assets_dir = data_dir.join("assets");
    let ssh_dir = data_dir.join("ssh");
    let shell_path = data_dir.join("bin").join("gitfox-shell");
    
    format!(r#"# GitFox 主配置文件
# 
# 此文件由 'gitfox init' 自动生成
# 请修改数据库和 Redis 连接信息后运行: gitfox start

# ============================================================================
# 核心配置 (必需)
# ============================================================================

# PostgreSQL 数据库连接
# 格式: postgres://username:password@hostname:port/database
DATABASE_URL=postgres://gitfox:password@localhost:5432/gitfox

# Redis 连接
REDIS_URL=redis://127.0.0.1:6379

# JWT 密钥 (自动生成，请勿修改)
JWT_SECRET={}

# ============================================================================
# 内部组件配置 (自动管理，无需修改)
# ============================================================================

# 后端 API 内部监听端口 (Workhorse 会自动代理)
SERVER_HOST=127.0.0.1
SERVER_PORT=8081

# Git 仓库存储路径
GIT_REPOS_PATH={}

# 用户上传文件路径
ASSETS_PATH={}

# ============================================================================
# SSH 配置
# ============================================================================

# 启用 SSH 服务器
SSH_ENABLED=true

# SSH 监听地址
SSH_HOST=0.0.0.0
SSH_PORT=22

# SSH 公开访问地址（用于 git clone 显示）
SSH_PUBLIC_HOST=localhost
SSH_PUBLIC_PORT=22

# SSH 主机密钥路径
SSH_HOST_KEY_PATH={}/host_key

# ============================================================================
# 外部访问配置
# ============================================================================

# GitFox 实例的公开 URL（用于 OAuth 回调等）
GITFOX_BASE_URL=http://localhost:8080

# GitFox Shell 配置 (SSH Git 操作)
GITFOX_SHELL_PATH={}
GITFOX_SHELL_SECRET={}

# ============================================================================
# 初始管理员 (自动生成)
# ============================================================================

# 首次启动时会自动创建此管理员账号
# 创建后请立即登录并修改密码！
INITIAL_ADMIN_USERNAME={}
INITIAL_ADMIN_EMAIL={}
INITIAL_ADMIN_PASSWORD={}

# ============================================================================
# SMTP 邮件配置 (可选)
# ============================================================================

# 启用邮件发送
SMTP_ENABLED=false

# SMTP 服务器
# SMTP_HOST=smtp.gmail.com
# SMTP_PORT=587
# SMTP_USERNAME=your-email@gmail.com
# SMTP_PASSWORD=your-app-password

# 发件人信息
# SMTP_FROM_EMAIL=noreply@gitfox.local
# SMTP_FROM_NAME=GitFox

# TLS/SSL 配置
# SMTP_USE_TLS=true   # STARTTLS (端口 587)
# SMTP_USE_SSL=false  # SSL (端口 465)

# ============================================================================
# OAuth 配置 (可选)
# ============================================================================

# GitHub OAuth
# OAUTH_GITHUB_CLIENT_ID=your-github-client-id
# OAUTH_GITHUB_CLIENT_SECRET=your-github-client-secret

# GitLab OAuth
# OAUTH_GITLAB_CLIENT_ID=your-gitlab-client-id
# OAUTH_GITLAB_CLIENT_SECRET=your-gitlab-client-secret
# OAUTH_GITLAB_URL=https://gitlab.com  # 自建实例可修改

# Google OAuth
# OAUTH_GOOGLE_CLIENT_ID=your-google-client-id
# OAUTH_GOOGLE_CLIENT_SECRET=your-google-client-secret

# ============================================================================
# WebAuthn / Passkey 配置
# ============================================================================

# WebAuthn Relying Party Name (显示名称)
WEBAUTHN_RP_NAME=GitFox

# WebAuthn RP ID (域名，不含协议和端口)
WEBAUTHN_RP_ID=localhost

# WebAuthn Origin (完整 URL)
WEBAUTHN_ORIGIN=http://localhost:8080

# ============================================================================
# Personal Access Token 配置
# ============================================================================

# PAT 默认过期天数 (0 = 永不过期)
PAT_DEFAULT_EXPIRATION_DAYS=365

# PAT 最大过期天数 (0 = 无限制)
PAT_MAX_EXPIRATION_DAYS=0

# ============================================================================
# WebIDE 配置
# ============================================================================

# WebIDE OAuth2 客户端 ID (固定值)
WEBIDE_CLIENT_ID=gitfox-webide

# WebIDE OAuth2 回调路径
WEBIDE_REDIRECT_URI_PATH=/-/ide/oauth/callback

# ============================================================================
# 高级配置
# ============================================================================

# JWT 过期时间 (秒，默认 24 小时)
JWT_EXPIRATION=86400

# 日志级别
RUST_LOG=info
"#, 
    secrets.jwt_secret,
    repos_dir.display(),
    assets_dir.display(),
    ssh_dir.display(),
    shell_path.display(),
    secrets.shell_secret,
    secrets.admin_username,
    secrets.admin_email,
    secrets.admin_password
    )
}

fn generate_workhorse_toml_template(data_dir: &Path) -> String {
    let frontend_dir = data_dir.join("frontend");
    let webide_dir = data_dir.join("webide");
    let assets_dir = data_dir.join("assets");
    let repos_dir = data_dir.join("repos");
    
    format!(r#"# GitFox Workhorse 配置文件
#
# 此文件由 'gitfox init' 自动生成
# Workhorse 是 HTTP 反向代理，负责：
# - 服务静态文件 (前端、WebIDE)
# - 代理 API 请求到后端
# - 处理大文件上传
# - WebSocket 连接

# ============================================================================
# 外部访问配置
# ============================================================================

# HTTP 监听地址（对外服务）
listen_addr = "0.0.0.0"
listen_port = 8080

# ============================================================================
# 内部组件配置 (自动管理，无需修改)
# ============================================================================

# 后端 API 服务器地址（内部通信，自动协调）
backend_url = "http://127.0.0.1:8081"

# ============================================================================
# 静态文件路径
# ============================================================================

# 前端 Vue SPA 文件
frontend_dist_path = "{}"

# WebIDE 文件
webide_dist_path = "{}"

# 用户上传文件
assets_path = "{}"

# Git 仓库路径
git_repos_path = "{}"

# ============================================================================
# 性能配置
# ============================================================================

# 最大上传文件大小 (字节，默认 100MB)
max_upload_size = 104857600

# WebSocket 超时时间 (秒，默认 1 小时)
websocket_timeout = 3600

# 静态文件缓存控制头
static_cache_control = "public, max-age=31536000, immutable"

# ============================================================================
# 功能开关
# ============================================================================

# 启用请求日志
enable_request_logging = true

# 启用 CORS
enable_cors = true
"#,
    frontend_dir.display(),
    webide_dir.display(),
    assets_dir.display(),
    repos_dir.display()
    )
}

/// 从 .env 文件读取配置到环境变量
fn load_env_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let line = line.trim();
        
        // 跳过注释和空行
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // 解析 KEY=VALUE
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            
            // 展开环境变量（如 ${HOME}）
            let expanded = shellexpand::full(value)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| value.to_string());
            
            // 只设置未被设置的环境变量（命令行参数优先）
            if env::var(key).is_err() {
                env::set_var(key, expanded);
            }
        }
    }
    
    Ok(())
}

