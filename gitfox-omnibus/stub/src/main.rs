//! GitFox - Self-contained DevOps Platform
//!
//! 这是 GitFox Omnibus 生成的超级二进制。
//! 包含所有组件，运行时自动解压并启动。
//!
//! 注意：这个文件中的 `$EMBED_*` 占位符会在 rust-embed 编译时被替换。
//! 开发时可以创建空目录来测试编译。

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use inquire::{Confirm, Text};
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

/// 前端静态文件 (Vue SPA) - 只有 index.html 等
#[derive(RustEmbed)]
#[folder = "embedded/frontend"]
#[prefix = ""]
struct FrontendAssets;

/// WebIDE 静态文件 (VS Code Web) - 只有 index.html 等
#[derive(RustEmbed)]
#[folder = "embedded/webide"]
#[prefix = ""]
struct WebideAssets;

/// 静态资源文件 (js/css 等)
/// 目录结构:
/// - main/         <- 主前端 js/css
/// - webide/main/  <- WebIDE js/css
/// - webide/vscode/ <- vscode 静态资源
/// - webide/extensions/ <- 扩展资源
#[derive(RustEmbed)]
#[folder = "embedded/static"]
#[prefix = ""]
struct StaticAssets;

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

/// 配置模板文件
#[derive(RustEmbed)]
#[folder = "embedded/templates"]
#[prefix = ""]
struct TemplateAssets;

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
    assets_dir: PathBuf,   // 静态资源根目录 (包含 main/, webide/, upload/)
    repos_dir: PathBuf,
}

impl ServicePaths {
    fn new(data_dir: &Path) -> Self {
        Self {
            bin_dir: data_dir.join("bin"),
            frontend_dir: data_dir.join("frontend"),
            webide_dir: data_dir.join("webide"),
            migrations_dir: data_dir.join("migrations"),
            assets_dir: data_dir.join("assets"),  // 静态资源根目录
            repos_dir: data_dir.join("repos"),
        }
    }

    fn ensure_dirs(&self) -> Result<()> {
        // 创建用户上传目录 (main/ 和 webide/* 由 extract_assets 创建)
        fs::create_dir_all(self.assets_dir.join("upload"))?;
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
        // 后端的 ASSETS_PATH 指向用户上传目录 (assets/upload)
        .env("ASSETS_PATH", config.assets_dir.join("upload"))
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
    
    // 读取 TOML 配置文件
    let config_content = fs::read_to_string(config_path)
        .context("Failed to read workhorse.toml")?;
    let config: toml::Value = toml::from_str(&config_content)
        .context("Failed to parse workhorse.toml")?;
    
    // 准备环境变量，使 TOML 配置优先于默认值
    let mut cmd = Command::new(&binary);
    cmd.env("RUST_LOG", if verbose { "debug" } else { "info" })
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    
    // 读取并设置配置项
    if let Some(listen_addr) = config.get("listen_addr").and_then(|v| v.as_str()) {
        cmd.env("WORKHORSE_LISTEN_ADDR", listen_addr);
    }
    if let Some(listen_port) = config.get("listen_port").and_then(|v| v.as_integer()) {
        cmd.env("WORKHORSE_LISTEN_PORT", listen_port.to_string());
    }
    if let Some(backend_url) = config.get("backend_url").and_then(|v| v.as_str()) {
        cmd.env("WORKHORSE_BACKEND_URL", backend_url);
    }
    if let Some(backend_socket) = config.get("backend_socket").and_then(|v| v.as_str()) {
        cmd.env("WORKHORSE_BACKEND_SOCKET", backend_socket);
    }
    if let Some(frontend_dist) = config.get("frontend_dist_path").and_then(|v| v.as_str()) {
        cmd.env("WORKHORSE_FRONTEND_DIST", frontend_dist);
    }
    if let Some(webide_dist) = config.get("webide_dist_path").and_then(|v| v.as_str()) {
        cmd.env("WORKHORSE_WEBIDE_DIST", webide_dist);
    }
    if let Some(assets_path) = config.get("assets_path").and_then(|v| v.as_str()) {
        cmd.env("WORKHORSE_ASSETS_PATH", assets_path);
    }
    if let Some(git_repos_path) = config.get("git_repos_path").and_then(|v| v.as_str()) {
        cmd.env("WORKHORSE_GIT_REPOS_PATH", git_repos_path);
    }
    
    let child = cmd.spawn()
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
    extract_embedded::<StaticAssets>(&data_dir.join("assets"), false)?;  // 静态资源 -> assets/
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
    
    println!("\nStatic Assets: {} files", StaticAssets::iter().count());
    list_first_n::<StaticAssets>(10);

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
    
    println!("\n🚀 GitFox Configuration Wizard\n");
    
    // 交互式收集配置
    let base_url = Text::new("GitFox public URL (用于 OAuth 回调等):")
        .with_default("http://localhost:8080")
        .with_help_message("例如: https://git.example.com or http://192.168.1.100:8080")
        .prompt()
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    
    let http_port: u16 = Text::new("HTTP 监听端口:")
        .with_default("8080")
        .with_help_message("Workhorse 对外服务的端口")
        .prompt()
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);
    
    println!("\n⚙️  后端服务配置\n");
    
    let use_unix_socket = Confirm::new("使用自动配置后端连接 (Unix Socket)?")
        .with_default(true)
        .with_help_message("推荐：组件间通过 Unix Socket 通信，性能更好更安全")
        .prompt()
        .unwrap_or(true);
    
    let (server_socket_path, server_host, server_port) = if use_unix_socket {
        let socket_path = "/tmp/gitfox-backend.sock".to_string();
        println!("   → Backend Socket: {}", socket_path);
        (socket_path, "127.0.0.1".to_string(), 8081)
    } else {
        println!("\n   手动配置后端连接 (TCP):\n");
        let host = Text::new("后端服务监听地址:")
            .with_default("127.0.0.1")
            .with_help_message("内部通信地址，通常使用 127.0.0.1")
            .prompt()
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        
        let port: u16 = Text::new("后端服务监听端口:")
            .with_default("8081")
            .with_help_message("后端 API 服务端口")
            .prompt()
            .unwrap_or_else(|_| "8081".to_string())
            .parse()
            .unwrap_or(8081);
        
        ("".to_string(), host, port)
    };
    
    println!("\n🔐 SSH 配置\n");
    
    let ssh_host = Text::new("SSH 监听地址:")
        .with_default("0.0.0.0")
        .with_help_message("通常使用 0.0.0.0 监听所有网卡")
        .prompt()
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    
    let ssh_port: u16 = Text::new("SSH 监听端口:")
        .with_default("2222")
        .with_help_message("默认 2222，避免与系统 SSH (22) 冲突")
        .prompt()
        .unwrap_or_else(|_| "2222".to_string())
        .parse()
        .unwrap_or(2222);
    
    let ssh_public_host = Text::new("SSH 公开访问地址:")
        .with_default("localhost")
        .with_help_message("用户通过此地址访问 Git SSH，例如：git.example.com 或 IP 地址")
        .prompt()
        .unwrap_or_else(|_| "localhost".to_string());
    
    let ssh_public_port: u16 = Text::new("SSH 公开访问端口:")
        .with_default(&ssh_port.to_string())
        .with_help_message("如果使用端口转发，这里填写外部端口")
        .prompt()
        .unwrap_or_else(|_| ssh_port.to_string())
        .parse()
        .unwrap_or(ssh_port);
    
    let enable_smtp = Confirm::new("启用 SMTP 邮件服务?")
        .with_default(false)
        .with_help_message("用于发送注册确认、密码重置等邮件")
        .prompt()
        .unwrap_or(false);
    
    // 如果启用 SMTP，收集详细配置
    let smtp_config = if enable_smtp {
        println!("\n📧 SMTP 配置\n");
        
        let host = Text::new("SMTP 服务器地址:")
            .with_default("smtp.gmail.com")
            .with_help_message("例如: smtp.gmail.com, smtp.office365.com, smtp.qq.com")
            .prompt()
            .unwrap_or_else(|_| "smtp.gmail.com".to_string());
        
        let port: u16 = Text::new("SMTP 端口:")
            .with_default("587")
            .with_help_message("587 (TLS/STARTTLS) 或 465 (SSL)")
            .prompt()
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .unwrap_or(587);
        
        let username = Text::new("SMTP 用户名:")
            .with_help_message("通常是你的邮箱地址")
            .prompt()
            .unwrap_or_else(|_| "your-email@gmail.com".to_string());
        
        let password = Text::new("SMTP 密码:")
            .with_help_message("Gmail 需使用应用专用密码，QQ邮箱需使用授权码")
            .prompt()
            .unwrap_or_else(|_| "your-app-password".to_string());
        
        let from_email = Text::new("发件人邮箱:")
            .with_default(&username)
            .with_help_message("邮件的发件人地址")
            .prompt()
            .unwrap_or_else(|_| username.clone());
        
        let from_name = Text::new("发件人名称:")
            .with_default("GitFox")
            .prompt()
            .unwrap_or_else(|_| "GitFox".to_string());
        
        let use_tls = port == 587;
        let use_ssl = port == 465;
        
        Some(SmtpConfig {
            host,
            port,
            username,
            password,
            from_email,
            from_name,
            use_tls,
            use_ssl,
        })
    } else {
        None
    };
    
    // 从 base_url 提取域名作为 WebAuthn RP ID 的默认值
    let default_rp_id = extract_domain(&base_url);
    let webauthn_rp_id = Text::new("WebAuthn RP ID (域名):")
        .with_default(&default_rp_id)
        .with_help_message("用于 Passkey/WebAuthn 认证的域名（不含协议和端口）")
        .prompt()
        .unwrap_or(default_rp_id);
    
    // 生成随机密钥和密码
    let jwt_secret = generate_random_secret(64);
    let shell_secret = generate_random_secret(64);
    let admin_username = "admin".to_string();
    let admin_email = "admin@localhost".to_string();
    let admin_password = generate_random_password(16);
    
    let user_config = UserConfig {
        base_url,
        http_port,
        use_unix_socket,
        server_socket_path,
        server_host,
        server_port,
        ssh_host,
        ssh_port,
        ssh_public_host,
        ssh_public_port,
        smtp_config,
        webauthn_rp_id: webauthn_rp_id.clone(),
    };
    
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
        let env_template = generate_gitfox_env_template(data_dir, &secrets, &user_config);
        fs::write(&gitfox_env, env_template)?;
        info!("Created: {}", gitfox_env.display());
    }
    
    // 生成 workhorse.toml 配置（自动协调内部通信）
    if workhorse_toml.exists() {
        warn!("{} already exists, skipping", workhorse_toml.display());
    } else {
        let toml_template = generate_workhorse_toml_template(data_dir, &user_config);
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
    println!("\n📝 Configuration Summary:");
    println!("   Public URL: {}", user_config.base_url);
    println!("   HTTP Port:  {}", user_config.http_port);
    if user_config.use_unix_socket {
        println!("   Backend:    Unix Socket ({})", user_config.server_socket_path);
    } else {
        println!("   Backend:    TCP ({}:{})", user_config.server_host, user_config.server_port);
    }
    println!("   SSH:        {}:{} (public: {}:{})", 
        user_config.ssh_host, user_config.ssh_port,
        user_config.ssh_public_host, user_config.ssh_public_port);
    if let Some(ref smtp) = user_config.smtp_config {
        println!("   SMTP:       Enabled");
        println!("     Server:   {}:{}", smtp.host, smtp.port);
        println!("     From:     {} <{}>", smtp.from_name, smtp.from_email);
    } else {
        println!("   SMTP:       Disabled");
    }
    println!("   WebAuthn:   {}", user_config.webauthn_rp_id);
    println!("\nNext steps:");
    println!("1. Edit {} with your PostgreSQL and Redis settings", gitfox_env.display());
    if user_config.smtp_config.is_some() {
        println!("2. Review SMTP settings in {}", gitfox_env.display());
        println!("3. Run: gitfox start");
    } else {
        println!("2. Run: gitfox start");
    }
    
    Ok(())
}

struct GeneratedSecrets {
    jwt_secret: String,
    shell_secret: String,
    admin_username: String,
    admin_email: String,
    admin_password: String,
}

struct UserConfig {
    base_url: String,
    http_port: u16,
    // Server 配置
    use_unix_socket: bool,
    server_socket_path: String,
    server_host: String,
    server_port: u16,
    // SSH 配置
    ssh_host: String,
    ssh_port: u16,
    ssh_public_host: String,
    ssh_public_port: u16,
    smtp_config: Option<SmtpConfig>,
    webauthn_rp_id: String,
}

struct SmtpConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
    from_email: String,
    from_name: String,
    use_tls: bool,
    use_ssl: bool,
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

/// 从 URL 中提取域名
fn extract_domain(url: &str) -> String {
    url.trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(':')
        .next()
        .unwrap_or("localhost")
        .to_string()
}

/// 生成 SMTP 配置部分
fn generate_smtp_config_section(smtp_config: &Option<SmtpConfig>) -> String {
    if let Some(smtp) = smtp_config {
        format!(r#"# SMTP 服务器
SMTP_HOST={}
SMTP_PORT={}
SMTP_USERNAME={}
SMTP_PASSWORD={}

# 发件人信息
SMTP_FROM_EMAIL={}
SMTP_FROM_NAME={}

# TLS/SSL 配置
SMTP_USE_TLS={}   # STARTTLS (端口 587)
SMTP_USE_SSL={}   # SSL (端口 465)"#,
            smtp.host,
            smtp.port,
            smtp.username,
            smtp.password,
            smtp.from_email,
            smtp.from_name,
            if smtp.use_tls { "true" } else { "false" },
            if smtp.use_ssl { "true" } else { "false" }
        )
    } else {
        r#"# SMTP 服务器（未配置，如需启用请取消注释并填写）
# SMTP_HOST=smtp.gmail.com
# SMTP_PORT=587
# SMTP_USERNAME=your-email@gmail.com
# SMTP_PASSWORD=your-app-password

# 发件人信息
# SMTP_FROM_EMAIL=noreply@gitfox.local
# SMTP_FROM_NAME=GitFox

# TLS/SSL 配置
# SMTP_USE_TLS=true   # STARTTLS (端口 587)
# SMTP_USE_SSL=false  # SSL (端口 465)"#.to_string()
    }
}

fn generate_gitfox_env_template(data_dir: &Path, secrets: &GeneratedSecrets, user_config: &UserConfig) -> String {
    // 从嵌入的模板文件加载
    let template_file = TemplateAssets::get("gitfox.env.template")
        .expect("gitfox.env.template not found in embedded assets");
    let mut template = String::from_utf8_lossy(&template_file.data).to_string();
    
    // 计算路径
    let repos_dir = data_dir.join("repos");
    let assets_dir = data_dir.join("assets");
    let ssh_dir = data_dir.join("ssh");
    let shell_path = data_dir.join("bin").join("gitfox-shell");
    
    // 替换所有模板变量
    template = template.replace("{{JWT_SECRET}}", &secrets.jwt_secret);
    template = template.replace("{{GIT_REPOS_PATH}}", &repos_dir.display().to_string());
    template = template.replace("{{ASSETS_PATH}}", &assets_dir.display().to_string());
    
    // Server 配置
    if user_config.use_unix_socket {
        template = template.replace("{{SERVER_CONNECTION_TYPE}}", "unix_socket");
        template = template.replace("{{SERVER_SOCKET_PATH}}", &user_config.server_socket_path);
        template = template.replace("{{SERVER_HOST}}", "127.0.0.1");
        template = template.replace("{{SERVER_PORT}}", "8081");
    } else {
        template = template.replace("{{SERVER_CONNECTION_TYPE}}", "tcp");
        template = template.replace("{{SERVER_SOCKET_PATH}}", "");
        template = template.replace("{{SERVER_HOST}}", &user_config.server_host);
        template = template.replace("{{SERVER_PORT}}", &user_config.server_port.to_string());
    }
    
    // SSH 配置
    template = template.replace("{{SSH_HOST}}", &user_config.ssh_host);
    template = template.replace("{{SSH_PORT}}", &user_config.ssh_port.to_string());
    template = template.replace("{{SSH_HOST_KEY_PATH}}", &ssh_dir.join("host_key").display().to_string());
    template = template.replace("{{SSH_PUBLIC_HOST}}", &user_config.ssh_public_host);
    template = template.replace("{{SSH_PUBLIC_PORT}}", &user_config.ssh_public_port.to_string());
    template = template.replace("{{GITFOX_BASE_URL}}", &user_config.base_url);
    template = template.replace("{{GITFOX_SHELL_PATH}}", &shell_path.display().to_string());
    template = template.replace("{{GITFOX_SHELL_SECRET}}", &secrets.shell_secret);
    template = template.replace("{{INITIAL_ADMIN_USERNAME}}", &secrets.admin_username);
    template = template.replace("{{INITIAL_ADMIN_EMAIL}}", &secrets.admin_email);
    template = template.replace("{{INITIAL_ADMIN_PASSWORD}}", &secrets.admin_password);
    template = template.replace("{{SMTP_ENABLED}}", if user_config.smtp_config.is_some() { "true" } else { "false" });
    
    // SMTP 配置 - 逐字段替换
    if let Some(smtp) = &user_config.smtp_config {
        template = template.replace("{{SMTP_HOST}}", &smtp.host);
        template = template.replace("{{SMTP_PORT}}", &smtp.port.to_string());
        template = template.replace("{{SMTP_USERNAME}}", &smtp.username);
        template = template.replace("{{SMTP_PASSWORD}}", &smtp.password);
        template = template.replace("{{SMTP_FROM_EMAIL}}", &smtp.from_email);
        template = template.replace("{{SMTP_FROM_NAME}}", &smtp.from_name);
        template = template.replace("{{SMTP_USE_TLS}}", if smtp.use_tls { "true" } else { "false" });
        template = template.replace("{{SMTP_USE_SSL}}", if smtp.use_ssl { "true" } else { "false" });
    } else {
        // 未配置 SMTP 时使用默认示例值
        template = template.replace("{{SMTP_HOST}}", "smtp.example.com");
        template = template.replace("{{SMTP_PORT}}", "587");
        template = template.replace("{{SMTP_USERNAME}}", "your-email@example.com");
        template = template.replace("{{SMTP_PASSWORD}}", "your-password");
        template = template.replace("{{SMTP_FROM_EMAIL}}", "noreply@example.com");
        template = template.replace("{{SMTP_FROM_NAME}}", "GitFox");
        template = template.replace("{{SMTP_USE_TLS}}", "true");
        template = template.replace("{{SMTP_USE_SSL}}", "false");
    }
    
    template = template.replace("{{WEBAUTHN_RP_ID}}", &user_config.webauthn_rp_id);
    
    template
}

fn generate_workhorse_toml_template(data_dir: &Path, user_config: &UserConfig) -> String {
    // 从嵌入的模板文件加载
    let template_file = TemplateAssets::get("workhorse.toml.template")
        .expect("workhorse.toml.template not found in embedded assets");
    let mut template = String::from_utf8_lossy(&template_file.data).to_string();
    
    // 计算路径
    let frontend_dir = data_dir.join("frontend");
    let webide_dir = data_dir.join("webide");
    let assets_dir = data_dir.join("assets");
    let repos_dir = data_dir.join("repos");
    
    // 替换所有模板变量
    template = template.replace("{{LISTEN_PORT}}", &user_config.http_port.to_string());
    
    // 后端连接配置
    if user_config.use_unix_socket {
        template = template.replace("{{BACKEND_SOCKET}}", &user_config.server_socket_path);
        template = template.replace("{{BACKEND_URL}}", "");
    } else {
        template = template.replace("{{BACKEND_SOCKET}}", "");
        template = template.replace("{{BACKEND_URL}}", &format!("http://{}:{}", user_config.server_host, user_config.server_port));
    }
    
    template = template.replace("{{FRONTEND_DIST_PATH}}", &frontend_dir.display().to_string());
    template = template.replace("{{WEBIDE_DIST_PATH}}", &webide_dir.display().to_string());
    template = template.replace("{{ASSETS_PATH}}", &assets_dir.display().to_string());
    template = template.replace("{{GIT_REPOS_PATH}}", &repos_dir.display().to_string());
    
    template
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

