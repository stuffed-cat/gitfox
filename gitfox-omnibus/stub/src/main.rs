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
use std::collections::HashMap;
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

/// 内置依赖二进制（仅在 bundled-deps feature 启用时可用）
#[cfg(feature = "bundled-deps")]
#[derive(RustEmbed)]
#[folder = "embedded/deps"]
#[prefix = ""]
struct BundledDepsAssets;

mod services;
mod unified_config;

use services::{BundledServices, BundledPostgresConfig, BundledRedisConfig};
use unified_config::{ConfigVars, GitFoxConfig, CONFIG_VERSION, generate_config_template, migrate_from_legacy};

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

    /// 从旧版本升级 (配置迁移)
    Upgrade {
        /// 自动应用所有变更 (不提示确认)
        #[arg(long)]
        yes: bool,
    },

    /// 重新生成所有组件配置文件 (从 gitfox.toml)
    Reconfigure,

    /// 验证和查看配置
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Clone, Subcommand)]
enum ConfigAction {
    /// 验证配置文件
    Check,
    /// 显示当前配置（隐藏敏感信息）
    Show,
    /// 生成各组件的配置文件
    Generate,
    /// 从 gitfox.env 迁移到 gitfox.toml
    Migrate,
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
        Commands::Upgrade { yes: _ } => {
            let gitfox_toml = cli.data_dir.join("gitfox.toml");
            let gitfox_env = cli.data_dir.join("gitfox.env");
            let workhorse_toml = cli.data_dir.join("workhorse.toml");
            
            // 如果 gitfox.toml 存在，检查是否需要版本升级
            if gitfox_toml.exists() {
                println!("📂 Found existing gitfox.toml, checking version...");
                match GitFoxConfig::load(&gitfox_toml) {
                    Ok(mut config) => {
                        if config.version == CONFIG_VERSION {
                            println!("✅ Configuration is already at latest version ({})", CONFIG_VERSION);
                            return Ok(());
                        }
                        
                        println!("🔄 Upgrading configuration from {} to {}...", config.version, CONFIG_VERSION);
                        
                        // 备份旧配置
                        let backup_path = cli.data_dir.join(format!("gitfox.toml.v{}.bak", config.version));
                        fs::copy(&gitfox_toml, &backup_path)?;
                        println!("   Backup saved to: {}", backup_path.display());
                        
                        // 保存升级后的配置（check_and_migrate 在 load 时已执行）
                        config.save(&gitfox_toml)?;
                        
                        println!("\n✅ Configuration upgraded to version {}!", CONFIG_VERSION);
                        println!("\n🔄 Regenerating component configuration files...");
                        reconfigure_from_toml(&cli.data_dir)?;
                        return Ok(());
                    }
                    Err(e) => {
                        println!("⚠️  Failed to load existing gitfox.toml: {}", e);
                        println!("   Will attempt migration from legacy files...");
                    }
                }
            }
            
            if !gitfox_env.exists() && !workhorse_toml.exists() {
                println!("❌ No configuration files found to migrate");
                println!("   Expected: gitfox.env and/or workhorse.toml in {}", cli.data_dir.display());
                return Ok(());
            }
            
            println!("🔄 Migrating configuration to gitfox.toml...\n");
            
            match migrate_from_legacy(&cli.data_dir) {
                Ok(result) => {
                    // 显示来源
                    println!("📂 Source files:");
                    for source in &result.sources {
                        println!("   - {}", source);
                    }
                    
                    // 保存新配置
                    result.config.save(&gitfox_toml)?;
                    
                    println!("\n✅ Migration completed!");
                    println!("   Migrated {} configuration fields", result.migrated_fields);
                    println!("   Output: {}", gitfox_toml.display());
                    
                    if !result.warnings.is_empty() {
                        println!("\n⚠️  Warnings:");
                        for warning in &result.warnings {
                            println!("   - {}", warning);
                        }
                    }
                    
                    // 自动执行 reconfigure 生成各组件配置
                    println!("\n🔄 Regenerating component configuration files...");
                    reconfigure_from_toml(&cli.data_dir)?;
                    
                    println!("\n📋 Next steps:");
                    println!("1. Review the generated gitfox.toml");
                    println!("2. Update DATABASE_URL and secrets if needed");
                    println!("3. Keep old config files as backup or remove them");
                    println!("4. Run 'gitfox start' to use the new config");
                }
                Err(e) => {
                    println!("❌ Migration failed: {}", e);
                }
            }
        }
        Commands::Reconfigure => {
            reconfigure_from_toml(&cli.data_dir)?;
        }
        Commands::Config { action } => {
            handle_config_command(&cli.data_dir, action)?;
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
    info!("Data directory: {}", cli.data_dir.display());

    // 加载 gitfox.toml 获取启用配置
    let gitfox_toml = cli.data_dir.join("gitfox.toml");
    if !gitfox_toml.exists() {
        eprintln!("❌ Missing configuration: gitfox.toml");
        eprintln!("   Expected at: {}", gitfox_toml.display());
        eprintln!("\nPlease run: gitfox init");
        return Err(anyhow::anyhow!("Configuration file missing: gitfox.toml"));
    }
    let config = GitFoxConfig::load(&gitfox_toml)?;
    let services = &config.services;

    // 检查启用组件所需的配置文件
    let mut required_configs: Vec<(&str, &str)> = Vec::new();
    if services.backend {
        required_configs.push(("gitfox.env", "devops backend"));
    }
    if services.gitlayer {
        required_configs.push(("gitlayer.env", "GitLayer"));
    }
    if services.shell && config.server.ssh.enabled {
        required_configs.push(("gitfox-shell.env", "SSH server"));
    }
    if services.workhorse {
        required_configs.push(("workhorse.toml", "HTTP proxy"));
    }

    for (file, desc) in &required_configs {
        let path = cli.data_dir.join(file);
        if !path.exists() {
            eprintln!("❌ Missing configuration: {} ({})", file, desc);
            eprintln!("   Expected at: {}", path.display());
            eprintln!("\nPlease run: gitfox reconfigure");
            return Err(anyhow::anyhow!("Configuration file missing: {}", file));
        }
    }

    // 解压资源到 data_dir
    extract_assets(&cli.data_dir)?;

    // 准备目录路径
    let paths = ServicePaths::new(&cli.data_dir);
    paths.ensure_dirs()?;

    // 初始化内置服务管理器
    let mut bundled_services = BundledServices::new(&cli.data_dir);

    // 启动内置 PostgreSQL（如果配置存在且启用）
    let pg_conf = cli.data_dir.join("postgresql.conf");
    if pg_conf.exists() && config.bundled.postgresql.enabled && BundledServices::has_bundled_deps() {
        info!("Starting bundled PostgreSQL...");
        bundled_services.start_postgresql_from_config(&pg_conf)?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    // 启动内置 Redis（如果配置存在且启用）
    let redis_conf = cli.data_dir.join("redis.conf");
    if redis_conf.exists() && config.bundled.redis.enabled && BundledServices::has_bundled_deps() {
        info!("Starting bundled Redis...");
        bundled_services.start_redis_from_config(&redis_conf)?;
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    // 根据配置启动组件
    let mut gitlayer: Option<Child> = None;
    let mut shell: Option<Child> = None;
    let mut backend: Option<Child> = None;
    let mut workhorse: Option<Child> = None;

    if services.gitlayer {
        info!("Starting GitLayer...");
        gitlayer = Some(start_gitlayer(&paths)?);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    if services.shell && config.server.ssh.enabled {
        info!("Starting gitfox-shell...");
        shell = Some(start_shell(&paths)?);
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    if services.backend {
        info!("Starting backend...");
        backend = Some(start_backend_from_env(&paths)?);
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    if services.workhorse {
        info!("Starting workhorse...");
        workhorse = Some(start_workhorse(&paths)?);
    }

    info!("GitFox is running!");
    info!("  Config dir: {}", cli.data_dir.display());
    info!("  Enabled services: backend={}, gitlayer={}, shell={}, workhorse={}",
          services.backend, services.gitlayer, 
          services.shell && config.server.ssh.enabled, services.workhorse);

    // 等待关闭信号
    let shutdown = Arc::new(AtomicBool::new(false));
    wait_for_shutdown(shutdown.clone()).await;

    // 优雅关闭服务（只关闭已启动的）
    info!("Shutting down...");
    if let Some(ref mut p) = workhorse { shutdown_process(p, "workhorse"); }
    if let Some(ref mut p) = backend { shutdown_process(p, "backend"); }
    if let Some(ref mut p) = gitlayer { shutdown_process(p, "gitlayer"); }
    if let Some(ref mut p) = shell { shutdown_process(p, "gitfox-shell"); }

    // 停止内置服务（如果有）
    bundled_services.stop_all();

    info!("GitFox stopped");
    Ok(())
}

// ============================================================================
// 服务路径
// ============================================================================

struct ServicePaths {
    data_dir: PathBuf,     // 数据根目录，配置文件和所有子目录的父目录
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
            data_dir: data_dir.to_path_buf(),
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
) -> Result<Child> {
    let binary = paths.bin_dir.join("devops");
    let env_file = paths.data_dir.join("gitfox.env");
    
    if !env_file.exists() {
        return Err(anyhow::anyhow!("gitfox.env not found at: {}", env_file.display()));
    }
    
    info!("Starting backend with config: {}", env_file.display());
    
    // 使用 bash source 加载配置文件，ctl 不控制任何配置
    let child = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "set -a; source '{}'; set +a; exec '{}'",
            env_file.display(),
            binary.display()
        ))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start backend")?;

    Ok(child)
}

/// 启动 GitLayer (Git 操作 RPC 服务)
fn start_gitlayer(paths: &ServicePaths) -> Result<Child> {
    let binary = paths.bin_dir.join("gitlayer");
    let env_file = paths.data_dir.join("gitlayer.env");
    
    if !env_file.exists() {
        return Err(anyhow::anyhow!("gitlayer.env not found at: {}", env_file.display()));
    }
    
    info!("Starting GitLayer with config: {}", env_file.display());
    
    let child = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "set -a; source '{}'; set +a; exec '{}'",
            env_file.display(),
            binary.display()
        ))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start gitlayer")?;

    Ok(child)
}

fn start_shell(paths: &ServicePaths) -> Result<Child> {
    let binary = paths.bin_dir.join("gitfox-shell");
    let env_file = paths.data_dir.join("gitfox-shell.env");
    
    if !env_file.exists() {
        return Err(anyhow::anyhow!("gitfox-shell.env not found at: {}", env_file.display()));
    }
    
    info!("Starting gitfox-shell with config: {}", env_file.display());
    
    let child = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "set -a; source '{}'; set +a; exec '{}'",
            env_file.display(),
            binary.display()
        ))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("Failed to start gitfox-shell")?;

    Ok(child)
}

// ============================================================================
// Workhorse 配置与启动
// ============================================================================

fn start_workhorse(paths: &ServicePaths) -> Result<Child> {
    let binary = paths.bin_dir.join("gitfox-workhorse");
    let config_path = paths.data_dir.join("workhorse.toml");
    
    if !config_path.exists() {
        return Err(anyhow::anyhow!("workhorse.toml not found at: {}", config_path.display()));
    }
    
    info!("Starting workhorse with config: {}", config_path.display());
    
    // workhorse 内部会解析 TOML 并设置所有配置
    let child = Command::new(&binary)
        .env("WORKHORSE_CONFIG", &config_path)
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
    
    let gitfox_toml = data_dir.join("gitfox.toml");
    
    // 如果已存在，提示用户
    if gitfox_toml.exists() {
        println!("⚠️  gitfox.toml already exists at: {}", gitfox_toml.display());
        println!("   Run 'gitfox reconfigure' to regenerate component configs.");
        println!("   Or delete gitfox.toml to re-initialize.");
        return Ok(());
    }
    
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
    
    // 计算各目录路径
    let repos_dir = data_dir.join("repos");
    let assets_dir = data_dir.join("assets");
    let ssh_dir = data_dir.join("ssh");
    let shell_path = data_dir.join("bin").join("gitfox-shell");
    
    // 从模板生成配置（模板必须存在）
    let template = TemplateAssets::get("gitfox.toml.template")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .expect("gitfox.toml.template not found in embedded assets - build is corrupted");
    
    // 构建配置变量
    let vars = ConfigVars {
        database_url: "postgres://gitfox:password@localhost/gitfox".to_string(),
        redis_url: "redis://127.0.0.1:6379".to_string(),
        jwt_secret: secrets.jwt_secret.clone(),
        gitfox_shell_secret: secrets.shell_secret.clone(),
        gitfox_base_url: user_config.base_url.clone(),
        http_port: user_config.http_port,
        max_upload_size: 1073741824, // 1GB
        ssh_enabled: true,
        ssh_host: user_config.ssh_host.clone(),
        ssh_port: user_config.ssh_port,
        ssh_public_host: user_config.ssh_public_host.clone(),
        ssh_public_port: user_config.ssh_public_port,
        server_connection_type: if user_config.use_unix_socket { "unix_socket".to_string() } else { "tcp".to_string() },
        server_socket_path: user_config.server_socket_path.clone(),
        server_host: user_config.server_host.clone(),
        server_port: user_config.server_port,
        git_repos_path: repos_dir.display().to_string(),
        assets_path: assets_dir.display().to_string(),
        frontend_path: data_dir.join("frontend").display().to_string(),
        webide_path: data_dir.join("webide").display().to_string(),
        ssh_host_key_path: ssh_dir.join("host_key").display().to_string(),
        gitfox_shell_path: shell_path.display().to_string(),
        initial_admin_username: secrets.admin_username.clone(),
        initial_admin_email: secrets.admin_email.clone(),
        initial_admin_password: secrets.admin_password.clone(),
        smtp_enabled: user_config.smtp_config.is_some(),
        smtp_host: user_config.smtp_config.as_ref().map(|c| c.host.clone()).unwrap_or_default(),
        smtp_port: user_config.smtp_config.as_ref().map(|c| c.port).unwrap_or(587),
        smtp_username: user_config.smtp_config.as_ref().map(|c| c.username.clone()).unwrap_or_default(),
        smtp_password: user_config.smtp_config.as_ref().map(|c| c.password.clone()).unwrap_or_default(),
        smtp_from_email: user_config.smtp_config.as_ref().map(|c| c.from_email.clone()).unwrap_or_default(),
        smtp_from_name: user_config.smtp_config.as_ref().map(|c| c.from_name.clone()).unwrap_or_default(),
        smtp_use_tls: user_config.smtp_config.as_ref().map(|c| c.use_tls).unwrap_or(true),
        smtp_use_ssl: user_config.smtp_config.as_ref().map(|c| c.use_ssl).unwrap_or(false),
        webauthn_rp_id: user_config.webauthn_rp_id.clone(),
        webauthn_rp_origin: user_config.base_url.clone(),
        rust_log: "info".to_string(),
        ..Default::default()
    };
    
    let toml_content = generate_config_template(&template, &vars);
    fs::write(&gitfox_toml, toml_content)?;
    info!("Created: {}", gitfox_toml.display());
    
    // 打印配置信息
    println!("\n✅ gitfox.toml created!");
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
    
    // 从 gitfox.toml 生成各组件配置文件
    println!("\n🔄 Generating component configuration files...");
    reconfigure_from_toml(data_dir)?;
    
    println!("\nNext steps:");
    println!("1. Edit {} with your PostgreSQL and Redis settings", gitfox_toml.display());
    println!("2. Run 'gitfox reconfigure' after editing");
    println!("3. Run: gitfox start");
    
    Ok(())
}

// ============================================================================
// 配置重建 (从 gitfox.toml 生成各组件配置)
// ============================================================================

/// 从 gitfox.toml 重新生成所有组件配置文件
fn reconfigure_from_toml(data_dir: &Path) -> Result<()> {
    let gitfox_toml = data_dir.join("gitfox.toml");
    
    if !gitfox_toml.exists() {
        println!("❌ gitfox.toml not found at: {}", gitfox_toml.display());
        println!("\n   Run 'gitfox init' to create configuration first.");
        return Err(anyhow::anyhow!("gitfox.toml not found"));
    }
    
    println!("📂 Loading configuration from: {}", gitfox_toml.display());
    let config = GitFoxConfig::load(&gitfox_toml)?;
    
    // 验证配置
    if let Ok(warnings) = config.validate() {
        if !warnings.is_empty() {
            println!("\n⚠️  Configuration warnings:");
            for warning in warnings {
                println!("   - {}", warning);
            }
        }
    }
    
    println!("\n🔄 Generating component configuration files...\n");
    
    // 1. gitfox.env (主应用 devops backend)
    let gitfox_env_path = data_dir.join("gitfox.env");
    write_env_file(&gitfox_env_path, &config.to_backend_env())?;
    println!("✅ Generated: {}", gitfox_env_path.display());
    
    // 2. gitlayer.env
    let gitlayer_env_path = data_dir.join("gitlayer.env");
    write_env_file(&gitlayer_env_path, &config.to_gitlayer_env())?;
    println!("✅ Generated: {}", gitlayer_env_path.display());
    
    // 3. gitfox-shell.env
    let shell_env_path = data_dir.join("gitfox-shell.env");
    write_env_file(&shell_env_path, &config.to_shell_env())?;
    println!("✅ Generated: {}", shell_env_path.display());
    
    // 4. workhorse.toml
    let workhorse_toml_path = data_dir.join("workhorse.toml");
    fs::write(&workhorse_toml_path, config.to_workhorse_toml())?;
    println!("✅ Generated: {}", workhorse_toml_path.display());
    
    println!("\n✨ All configuration files generated successfully!");
    println!("\n📋 Generated files:");
    println!("   - gitfox.env         (devops backend)");
    println!("   - gitlayer.env       (Git operations service)");
    println!("   - gitfox-shell.env   (SSH server)");
    println!("   - workhorse.toml     (HTTP proxy)");
    println!("\n💡 These files are auto-generated from gitfox.toml");
    println!("   To update configuration:");
    println!("   1. Edit gitfox.toml");
    println!("   2. Run: gitfox reconfigure");
    
    Ok(())
}

/// 将环境变量 HashMap 写入 .env 文件
fn write_env_file(path: &Path, env: &HashMap<String, String>) -> Result<()> {
    let mut content = String::new();
    content.push_str("# Auto-generated from gitfox.toml by 'gitfox reconfigure'\n");
    content.push_str("# DO NOT EDIT MANUALLY - Changes will be overwritten\n");
    content.push_str("# Edit gitfox.toml instead and run 'gitfox reconfigure'\n\n");
    
    // 按字母顺序排序 key
    let mut keys: Vec<_> = env.keys().collect();
    keys.sort();
    
    for key in keys {
        let value = &env[key];
        // 如果值包含空格或特殊字符，用引号包裹
        if value.contains(' ') || value.contains('$') || value.contains('#') {
            content.push_str(&format!("{}=\"{}\"\n", key, value));
        } else {
            content.push_str(&format!("{}={}\n", key, value));
        }
    }
    
    fs::write(path, content)
        .with_context(|| format!("Failed to write env file: {}", path.display()))?;
    
    Ok(())
}

// ============================================================================
// 配置命令处理
// ============================================================================

fn handle_config_command(data_dir: &Path, action: ConfigAction) -> Result<()> {
    let gitfox_toml = data_dir.join("gitfox.toml");
    
    match action {
        ConfigAction::Check => {
            println!("🔍 Checking configuration...\n");
            
            if !gitfox_toml.exists() {
                println!("❌ gitfox.toml not found at: {}", gitfox_toml.display());
                println!("\n   Run 'gitfox init' to create configuration files.");
                return Ok(());
            }
            
            match GitFoxConfig::load(&gitfox_toml) {
                Ok(config) => {
                    match config.validate() {
                        Ok(warnings) => {
                            println!("✅ Configuration is valid!");
                            if !warnings.is_empty() {
                                println!("\n⚠️  Warnings:");
                                for warning in warnings {
                                    println!("   - {}", warning);
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Configuration error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to load config: {}", e);
                }
            }
        }
        
        ConfigAction::Show => {
            if !gitfox_toml.exists() {
                println!("❌ gitfox.toml not found at: {}", gitfox_toml.display());
                return Ok(());
            }
            
            let config = GitFoxConfig::load(&gitfox_toml)?;
            
            println!("📋 GitFox Configuration\n");
            println!("Version: {}", config.version);
            println!("\n[database]");
            println!("  url = \"postgres://***:***@***/***\" (hidden)");
            println!("\n[redis]");
            println!("  url = \"{}\"", config.redis.url);
            println!("\n[secrets]");
            println!("  jwt = \"***\" (hidden)");
            println!("  internal = \"***\" (hidden)");
            println!("\n[server]");
            println!("  base_url = \"{}\"", config.server.base_url);
            println!("  http_port = {}", config.server.http_port);
            println!("\n[server.ssh]");
            println!("  enabled = {}", config.server.ssh.enabled);
            println!("  host = \"{}\"", config.server.ssh.host);
            println!("  port = {}", config.server.ssh.port);
            println!("  public_host = \"{}\"", config.server.ssh.public_host);
            println!("  public_port = {}", config.server.ssh.public_port);
            println!("\n[internal]");
            if let Some(ref socket) = config.internal.backend_socket {
                println!("  backend_socket = \"{}\"", socket);
            } else {
                println!("  backend_port = {}", config.internal.backend_port);
            }
            println!("  gitlayer_port = {}", config.internal.gitlayer_port);
            println!("  auth_grpc_port = {}", config.internal.auth_grpc_port);
            println!("\n[paths]");
            println!("  repos = \"{}\"", config.paths.repos);
            println!("  frontend = \"{}\"", config.paths.frontend);
            println!("  webide = \"{}\"", config.paths.webide);
            println!("\n[smtp]");
            println!("  enabled = {}", config.smtp.enabled);
            if config.smtp.enabled {
                println!("  host = \"{}\"", config.smtp.host);
                println!("  port = {}", config.smtp.port);
            }
            println!("\n[logging]");
            println!("  level = \"{}\"", config.logging.level);
        }
        
        ConfigAction::Generate => {
            if !gitfox_toml.exists() {
                println!("❌ gitfox.toml not found at: {}", gitfox_toml.display());
                println!("   Run 'gitfox init' first.");
                return Ok(());
            }
            
            let config = GitFoxConfig::load(&gitfox_toml)?;
            
            // 生成 workhorse.toml
            let workhorse_toml_path = data_dir.join("workhorse.toml");
            let workhorse_content = config.to_workhorse_toml();
            fs::write(&workhorse_toml_path, &workhorse_content)?;
            println!("✅ Generated: {}", workhorse_toml_path.display());
            
            // 显示各组件的环境变量
            println!("\n📋 Backend environment variables:");
            let backend_env = config.to_backend_env();
            for (key, _) in backend_env.iter().take(5) {
                println!("   {} = ...", key);
            }
            println!("   ... and {} more", backend_env.len().saturating_sub(5));
            
            println!("\n📋 GitLayer environment variables:");
            for (key, value) in config.to_gitlayer_env() {
                println!("   {} = {}", key, value);
            }
            
            println!("\n📋 Shell environment variables:");
            for (key, value) in config.to_shell_env() {
                if key.contains("SECRET") {
                    println!("   {} = ***", key);
                } else {
                    println!("   {} = {}", key, value);
                }
            }
        }
        
        ConfigAction::Migrate => {
            let gitfox_env = data_dir.join("gitfox.env");
            let workhorse_toml = data_dir.join("workhorse.toml");
            
            // 如果 gitfox.toml 存在，检查是否需要版本升级
            if gitfox_toml.exists() {
                println!("📂 Found existing gitfox.toml, checking version...");
                match GitFoxConfig::load(&gitfox_toml) {
                    Ok(mut config) => {
                        if config.version == CONFIG_VERSION {
                            println!("✅ Configuration is already at latest version ({})", CONFIG_VERSION);
                            return Ok(());
                        }
                        
                        println!("🔄 Upgrading configuration from {} to {}...", config.version, CONFIG_VERSION);
                        
                        // 备份旧配置
                        let backup_path = data_dir.join(format!("gitfox.toml.v{}.bak", config.version));
                        fs::copy(&gitfox_toml, &backup_path)?;
                        println!("   Backup saved to: {}", backup_path.display());
                        
                        // 保存升级后的配置
                        config.save(&gitfox_toml)?;
                        
                        println!("\n✅ Configuration upgraded to version {}!", CONFIG_VERSION);
                        println!("\n🔄 Regenerating component configuration files...");
                        reconfigure_from_toml(data_dir)?;
                        return Ok(());
                    }
                    Err(e) => {
                        println!("⚠️  Failed to load existing gitfox.toml: {}", e);
                        println!("   Will attempt migration from legacy files...");
                    }
                }
            }
            
            if !gitfox_env.exists() && !workhorse_toml.exists() {
                println!("❌ No configuration files found to migrate");
                println!("   Expected: gitfox.env and/or workhorse.toml in {}", data_dir.display());
                return Ok(());
            }
            
            println!("🔄 Migrating to gitfox.toml...\n");
            
            match migrate_from_legacy(data_dir) {
                Ok(result) => {
                    // 显示来源
                    println!("📂 Source files:");
                    for source in &result.sources {
                        println!("   - {}", source);
                    }
                    
                    // 保存新配置
                    result.config.save(&gitfox_toml)?;
                    
                    println!("\n✅ Migration completed!");
                    println!("   Migrated {} fields", result.migrated_fields);
                    println!("   Output: {}", gitfox_toml.display());
                    
                    if !result.warnings.is_empty() {
                        println!("\n⚠️  Warnings:");
                        for warning in &result.warnings {
                            println!("   - {}", warning);
                        }
                    }
                    
                    // 自动执行 reconfigure 生成各组件配置
                    println!("\n🔄 Regenerating component configuration files...");
                    reconfigure_from_toml(data_dir)?;
                    
                    println!("\n📋 Next steps:");
                    println!("1. Review the generated gitfox.toml");
                    println!("2. Update DATABASE_URL and secrets if needed");
                    println!("3. Keep old config files as backup or remove them");
                    println!("4. Run 'gitfox config check' to validate");
                }
                Err(e) => {
                    println!("❌ Migration failed: {}", e);
                }
            }
        }
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

/// 从 .env 文件读取配置并设置到 Command 环境变量
/// 与 load_env_file 类似，但设置到子进程命令而不是当前进程
fn load_env_to_command(path: &Path, cmd: &mut Command) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    
    for line in content.lines() {
        let line = line.trim();
        
        // 跳过注释和空行
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // 解析 KEY=VALUE
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            
            // 展开环境变量（如 ${HOME}）
            let expanded = shellexpand::full(value)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| value.to_string());
            
            cmd.env(key, expanded);
        }
    }
    
    Ok(())
}

