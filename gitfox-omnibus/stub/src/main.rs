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

use services::{BundledServices, BundledPostgresConfig};
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
                // 使用 load_raw 获取原始版本（不执行迁移）
                match unified_config::GitFoxConfig::load_raw(&gitfox_toml) {
                    Ok(mut config) => {
                        let original_version = config.version.clone();
                        
                        if original_version == CONFIG_VERSION {
                            println!("✅ Configuration is already at latest version ({})", CONFIG_VERSION);
                            return Ok(());
                        }
                        
                        println!("🔄 Upgrading configuration from {} to {}...", original_version, CONFIG_VERSION);
                        
                        // 备份旧配置（使用原始版本号）
                        let backup_path = cli.data_dir.join(format!("gitfox.toml.v{}.bak", original_version));
                        fs::copy(&gitfox_toml, &backup_path)?;
                        println!("   Backup saved to: {}", backup_path.display());
                        
                        // 使用 load_with_migration 执行迁移
                        let result = unified_config::GitFoxConfig::load_with_migration(&gitfox_toml)?;
                        
                        // 保存升级后的配置（只更新版本号和迁移修改的字段）
                        result.config.save_migration(&gitfox_toml)?;
                        
                        println!("\n✅ Configuration upgraded from {} to {}!", original_version, CONFIG_VERSION);
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
    
    // 加载配置（检查是否需要迁移）
    let load_result = unified_config::GitFoxConfig::load_with_migration(&gitfox_toml)?;
    let mut config = load_result.config;
    
    // 如果发生了迁移，保存更新后的配置（避免每次启动都重复迁移）
    // 使用 save_migration 只更新版本号和迁移修改的字段，不覆盖用户的其他配置
    if load_result.migrated {
        info!(
            "Configuration migrated from {} to {}, saving...",
            load_result.original_version, config.version
        );
        config.save_migration(&gitfox_toml)?;
    }
    
    // 转换相对路径为绝对路径
    config.resolve_paths(&cli.data_dir);
    
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
        // 从统一配置构造 services 层配置
        let pg_service_config = BundledPostgresConfig {
            enabled: config.bundled.postgresql.enabled,
            port: config.bundled.postgresql.port,
            host: config.bundled.postgresql.host.clone(),
            database: config.bundled.postgresql.database.clone(),
            username: config.bundled.postgresql.username.clone(),
            password: config.bundled.postgresql.password.clone(),
            max_connections: config.bundled.postgresql.max_connections,
            shared_buffers_mb: config.bundled.postgresql.shared_buffers,
            work_mem_mb: config.bundled.postgresql.work_mem,
            data_dir: cli.data_dir.join("postgresql/data"),
        };
        bundled_services.start_postgresql(&pg_service_config)?;
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    // 启动内置 Redis（如果配置存在且启用）
    let redis_conf = cli.data_dir.join("redis.conf");
    if redis_conf.exists() && config.bundled.redis.enabled && BundledServices::has_bundled_deps() {
        info!("Starting bundled Redis...");
        // 从统一配置构造 services 层配置
        let redis_service_config = services::BundledRedisConfig {
            enabled: config.bundled.redis.enabled,
            port: config.bundled.redis.port,
            host: config.bundled.redis.host.clone(),
            maxmemory_mb: config.bundled.redis.maxmemory,
            maxmemory_policy: config.bundled.redis.maxmemory_policy.clone(),
            persistence: config.bundled.redis.persistence,
            persistence_mode: config.bundled.redis.persistence_mode.clone(),
            data_dir: cli.data_dir.join("redis/data"),
        };
        bundled_services.start_redis(&redis_service_config)?;
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

    // 询问服务启用配置
    println!("\n🔧 服务启用配置\n");
    println!("   GitFox 允许只运行部分组件，适合分布式部署\n");
    
    let use_all_in_one = Confirm::new("使用默认 All-in-One 模式 (启用所有组件)?")
        .with_default(true)
        .with_help_message("推荐：单机部署时启用所有组件")
        .prompt()
        .unwrap_or(true);
    
    let (services_backend, services_gitlayer, services_shell, services_workhorse) = if use_all_in_one {
        (true, true, true, true)
    } else {
        println!("\n   选择要启用的组件:\n");
        
        let backend = Confirm::new("启用 Backend (API + gRPC Auth)?")
            .with_default(true)
            .with_help_message("核心后端服务")
            .prompt()
            .unwrap_or(true);
        
        let gitlayer = Confirm::new("启用 GitLayer (Git 操作 gRPC)?")
            .with_default(true)
            .with_help_message("Git 仓库操作服务")
            .prompt()
            .unwrap_or(true);
        
        let shell = Confirm::new("启用 Shell (SSH 服务器)?")
            .with_default(true)
            .with_help_message("Git SSH 访问")
            .prompt()
            .unwrap_or(true);
        
        let workhorse = Confirm::new("启用 Workhorse (HTTP 反向代理)?")
            .with_default(true)
            .with_help_message("HTTP 入口、静态文件、LFS")
            .prompt()
            .unwrap_or(true);
        
        (backend, gitlayer, shell, workhorse)
    };

    // 询问内置服务配置（bundled deps）
    println!("\n📦 内置服务配置\n");
    println!("   GitFox 可以打包内置 PostgreSQL、Redis、Nginx");
    println!("   如果您有自己的基础设施，可以禁用内置服务\n");
    
    let use_bundled = Confirm::new("使用默认集成套件 (内置 PostgreSQL/Redis)?")
        .with_default(false)
        .with_help_message("推荐：使用外部数据库。内置服务适合快速体验")
        .prompt()
        .unwrap_or(false);
    
    let (bundled_postgresql, bundled_redis, bundled_nginx) = if use_bundled {
        println!("\n   选择要启用的内置服务:\n");
        
        let postgresql = Confirm::new("启用内置 PostgreSQL?")
            .with_default(true)
            .with_help_message("内置 PostgreSQL 数据库")
            .prompt()
            .unwrap_or(true);
        
        let redis = Confirm::new("启用内置 Redis?")
            .with_default(true)
            .with_help_message("内置 Redis 缓存服务")
            .prompt()
            .unwrap_or(true);
        
        let nginx = Confirm::new("启用内置 Nginx?")
            .with_default(false)
            .with_help_message("内置 Nginx 反代（用于 HTTPS/负载均衡）")
            .prompt()
            .unwrap_or(false);
        
        (postgresql, redis, nginx)
    } else {
        (false, false, false)
    };
    
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
        services_backend,
        services_gitlayer,
        services_shell,
        services_workhorse,
        registry_domain: String::new(),
        registry_storage_path: data_dir.join("registry").display().to_string(),
        bundled_enabled: use_bundled,
        bundled_postgresql_enabled: bundled_postgresql,
        bundled_redis_enabled: bundled_redis,
        bundled_nginx_enabled: bundled_nginx,
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
    
    // 加载配置（检查是否需要迁移）
    let load_result = unified_config::GitFoxConfig::load_with_migration(&gitfox_toml)?;
    let config = load_result.config.clone();
    
    // 如果发生了迁移，保存更新后的配置（只更新版本号和迁移修改的字段）
    if load_result.migrated {
        println!("   ⚠️  Configuration migrated from {} to {}", 
            load_result.original_version, config.version);
        load_result.config.save_migration(&gitfox_toml)?;
        println!("   ✅ Saved migrated configuration");
    }
    
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
    
    // 1. gitfox.env (主应用 devops backend) - 使用模板
    let gitfox_env_path = data_dir.join("gitfox.env");
    // 模板从构建时复制的源位置加载 (.env.example → gitfox.env.template)
    let backend_template = TemplateAssets::get("gitfox.env.template")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .expect("gitfox.env.template not found in embedded templates");
    fs::write(&gitfox_env_path, config.to_backend_env_template(&backend_template))?;
    println!("✅ Generated: {}", gitfox_env_path.display());
    
    // 2. gitlayer.env - 使用模板
    let gitlayer_env_path = data_dir.join("gitlayer.env");
    // 模板从构建时复制的源位置加载 (gitlayer/.env.example → gitlayer.env.template)
    let gitlayer_template = TemplateAssets::get("gitlayer.env.template")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .expect("gitlayer.env.template not found in embedded templates");
    fs::write(&gitlayer_env_path, config.to_gitlayer_env_template(&gitlayer_template))?;
    println!("✅ Generated: {}", gitlayer_env_path.display());
    
    // 3. gitfox-shell.env - 使用模板
    let shell_env_path = data_dir.join("gitfox-shell.env");
    // 模板从构建时复制的源位置加载 (gitfox-shell/.env.example → shell.env.template)
    let shell_template = TemplateAssets::get("shell.env.template")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .expect("shell.env.template not found in embedded templates");
    fs::write(&shell_env_path, config.to_shell_env_template(&shell_template))?;
    println!("✅ Generated: {}", shell_env_path.display());
    
    // 4. workhorse.toml - 使用模板
    let workhorse_toml_path = data_dir.join("workhorse.toml");
    // 模板从构建时复制的源位置加载 (gitfox-workhorse/config.example.toml → workhorse.toml.template)
    let workhorse_template = TemplateAssets::get("workhorse.toml.template")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .expect("workhorse.toml.template not found in embedded templates");
    fs::write(&workhorse_toml_path, config.to_workhorse_toml(&workhorse_template))?;
    println!("✅ Generated: {}", workhorse_toml_path.display());
    
    // 5. postgresql.conf（如果启用内置 PostgreSQL）
    if config.bundled.postgresql.enabled {
        let pg_conf_path = data_dir.join("postgresql.conf");
        let pg_data_dir = data_dir.join("postgresql").join("data");
        let pg_conf_content = generate_postgresql_conf(&config, &pg_data_dir);
        fs::write(&pg_conf_path, pg_conf_content)?;
        println!("✅ Generated: {} (bundled PostgreSQL)", pg_conf_path.display());
    }
    
    // 6. redis.conf（如果启用内置 Redis）
    if config.bundled.redis.enabled {
        let redis_conf_path = data_dir.join("redis.conf");
        let redis_data_dir = data_dir.join("redis").join("data");
        let redis_conf_content = generate_redis_conf(&config, &redis_data_dir);
        fs::write(&redis_conf_path, redis_conf_content)?;
        println!("✅ Generated: {} (bundled Redis)", redis_conf_path.display());
    }

    // 7. nginx.conf（如果启用内置 Nginx）
    if config.bundled.nginx.enabled {
        let nginx_conf_path = data_dir.join("nginx.conf");
        let nginx_pid_path = data_dir.join("nginx").join("nginx.pid");
        let nginx_conf_content = generate_nginx_conf(&config, &nginx_pid_path);
        fs::write(&nginx_conf_path, nginx_conf_content)?;
        println!("✅ Generated: {} (bundled Nginx)", nginx_conf_path.display());
    }
    
    println!("\n✨ All configuration files generated successfully!");
    println!("\n📋 Generated files:");
    println!("   - gitfox.env         (devops backend)");
    println!("   - gitlayer.env       (Git operations service)");
    println!("   - gitfox-shell.env   (SSH server)");
    println!("   - workhorse.toml     (HTTP proxy)");
    if config.bundled.postgresql.enabled {
        println!("   - postgresql.conf    (bundled PostgreSQL)");
    }
    if config.bundled.redis.enabled {
        println!("   - redis.conf         (bundled Redis)");
    }
    if config.bundled.nginx.enabled {
        println!("   - nginx.conf         (bundled Nginx)");
    }
    println!("\n💡 These files are auto-generated from gitfox.toml");
    println!("   To update configuration:");
    println!("   1. Edit gitfox.toml");
    println!("   2. Run: gitfox reconfigure");
    
    Ok(())
}

/// 生成 PostgreSQL 配置文件内容
fn generate_postgresql_conf(config: &GitFoxConfig, data_dir: &Path) -> String {
    let pg = &config.bundled.postgresql;
    let effective_cache = pg.shared_buffers * 3; // 大约 3 倍 shared_buffers
    
    let template = TemplateAssets::get("postgresql.conf.template")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .unwrap_or_else(|| {
            // 如果模板不存在，使用硬编码的默认配置
            include_str!("../embedded/templates/postgresql.conf.template").to_string()
        });
    
    template
        .replace("{{POSTGRESQL_HOST}}", &pg.host)
        .replace("{{POSTGRESQL_PORT}}", &pg.port.to_string())
        .replace("{{POSTGRESQL_MAX_CONNECTIONS}}", &pg.max_connections.to_string())
        .replace("{{POSTGRESQL_SHARED_BUFFERS}}", &pg.shared_buffers.to_string())
        .replace("{{POSTGRESQL_WORK_MEM}}", &pg.work_mem.to_string())
        .replace("{{POSTGRESQL_EFFECTIVE_CACHE}}", &effective_cache.to_string())
        .replace("{{POSTGRESQL_DATA_DIR}}", &data_dir.display().to_string())
}

/// 生成 Redis 配置文件内容
fn generate_redis_conf(config: &GitFoxConfig, data_dir: &Path) -> String {
    let redis = &config.bundled.redis;
    
    // 根据持久化配置生成相应的配置段
    let persistence_config = if redis.persistence {
        match redis.persistence_mode.as_str() {
            "aof" => "appendonly yes\nappendfsync everysec".to_string(),
            "rdb+aof" => "save 900 1\nsave 300 10\nsave 60 10000\nappendonly yes\nappendfsync everysec".to_string(),
            _ => "save 900 1\nsave 300 10\nsave 60 10000".to_string(), // rdb
        }
    } else {
        "save \"\"".to_string()
    };
    
    let template = TemplateAssets::get("redis.conf.template")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .unwrap_or_else(|| {
            include_str!("../embedded/templates/redis.conf.template").to_string()
        });
    
    template
        .replace("{{REDIS_HOST}}", &redis.host)
        .replace("{{REDIS_PORT}}", &redis.port.to_string())
        .replace("{{REDIS_MAXMEMORY}}", &redis.maxmemory.to_string())
        .replace("{{REDIS_MAXMEMORY_POLICY}}", &redis.maxmemory_policy)
        .replace("{{REDIS_DATA_DIR}}", &data_dir.display().to_string())
        .replace("{{REDIS_PERSISTENCE_CONFIG}}", &persistence_config)
}

/// 生成 Nginx 配置文件内容
fn generate_nginx_conf(config: &GitFoxConfig, pid_path: &Path) -> String {
    let nginx = &config.bundled.nginx;
    
    // worker_processes: 0 = auto
    let worker_processes = if nginx.worker_processes == 0 {
        "auto".to_string()
    } else {
        nginx.worker_processes.to_string()
    };
    
    // gzip 配置
    let gzip_config = if nginx.gzip_enabled {
        r#"    gzip on;
    gzip_vary on;
    gzip_proxied any;
    gzip_comp_level 6;
    gzip_min_length 1000;
    gzip_types text/plain text/css text/xml application/json application/javascript application/xml+rss application/atom+xml image/svg+xml;"#.to_string()
    } else {
        "    gzip off;".to_string()
    };
    
    // SSL 重定向配置
    let ssl_redirect = if nginx.ssl_enabled {
        "        # HTTPS 重定向\n        return 301 https://$host$request_uri;".to_string()
    } else {
        String::new()
    };
    
    // 构建 upstream servers 列表（支持 workhorse 集群负载均衡）
    let upstream_servers = nginx.upstream_servers
        .iter()
        .map(|s| format!("        server {};", s))
        .collect::<Vec<_>>()
        .join("\n");
    
    // HTTPS 服务器配置（从模板加载）
    let https_server = if nginx.ssl_enabled {
        let https_template = TemplateAssets::get("nginx-https-server.template")
            .map(|f| String::from_utf8_lossy(&f.data).to_string())
            .unwrap_or_else(|| {
                include_str!("../embedded/templates/nginx-https-server.template").to_string()
            });
        
        https_template
            .replace("{{NGINX_HOST}}", &nginx.host)
            .replace("{{NGINX_HTTPS_PORT}}", &nginx.https_port.to_string())
            .replace("{{NGINX_SSL_CERTIFICATE}}", &nginx.ssl_certificate)
            .replace("{{NGINX_SSL_CERTIFICATE_KEY}}", &nginx.ssl_certificate_key)
            .replace("{{NGINX_STATIC_CACHE_TIME}}", &nginx.static_cache_time)
    } else {
        String::new()
    };
    
    let template = TemplateAssets::get("nginx.conf.template")
        .map(|f| String::from_utf8_lossy(&f.data).to_string())
        .unwrap_or_else(|| {
            include_str!("../embedded/templates/nginx.conf.template").to_string()
        });
    
    template
        .replace("{{NGINX_WORKER_PROCESSES}}", &worker_processes)
        .replace("{{NGINX_WORKER_CONNECTIONS}}", &nginx.worker_connections.to_string())
        .replace("{{NGINX_PID_PATH}}", &pid_path.display().to_string())
        .replace("{{NGINX_CLIENT_MAX_BODY_SIZE}}", &nginx.client_max_body_size)
        .replace("{{NGINX_GZIP_CONFIG}}", &gzip_config)
        .replace("{{NGINX_UPSTREAM_SERVERS}}", &upstream_servers)
        .replace("{{NGINX_HOST}}", &nginx.host)
        .replace("{{NGINX_HTTP_PORT}}", &nginx.http_port.to_string())
        .replace("{{NGINX_SSL_REDIRECT}}", &ssl_redirect)
        .replace("{{NGINX_STATIC_CACHE_TIME}}", &nginx.static_cache_time)
        .replace("{{NGINX_HTTPS_SERVER}}", &https_server)
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
            // 模板从构建时复制的源位置加载 (gitfox-workhorse/config.example.toml → workhorse.toml.template)
            let workhorse_template = TemplateAssets::get("workhorse.toml.template")
                .map(|f| String::from_utf8_lossy(&f.data).to_string())
                .expect("workhorse.toml.template not found in embedded templates");
            let workhorse_content = config.to_workhorse_toml(&workhorse_template);
            fs::write(&workhorse_toml_path, &workhorse_content)?;
            println!("✅ Generated: {}", workhorse_toml_path.display());
            
            // 显示各组件的配置变量
            println!("\n📋 Backend configuration variables:");
            let backend_vars = config.to_backend_vars();
            println!("   DATABASE_URL = ...");
            println!("   SERVER_HOST = {}", backend_vars.server_host);
            println!("   SERVER_PORT = {}", backend_vars.server_port);
            println!("   GRPC_ADDRESS = {}", backend_vars.grpc_address);
            println!("   GITLAYER_ADDRESS = {}", backend_vars.gitlayer_address);
            
            println!("\n📋 GitLayer configuration variables:");
            let gitlayer_vars = config.to_gitlayer_vars();
            println!("   GITLAYER_LISTEN_ADDR = {}", gitlayer_vars.gitlayer_listen_addr);
            println!("   GIT_REPOS_PATH = {}", gitlayer_vars.git_repos_path);
            println!("   RUST_LOG = {}", gitlayer_vars.rust_log);
            
            println!("\n📋 Shell configuration variables:");
            let shell_vars = config.to_shell_vars();
            println!("   SSH_LISTEN_ADDR = {}", shell_vars.ssh_listen_addr);
            println!("   SSH_HOST_KEY_PATH = {}", shell_vars.ssh_host_key_path);
            println!("   AUTH_GRPC_ADDRESS = {}", shell_vars.auth_grpc_address);
            println!("   GITLAYER_ADDRESS = {}", shell_vars.gitlayer_address);
            println!("   GITFOX_SHELL_SECRET = ***");
        }
        
        ConfigAction::Migrate => {
            let gitfox_env = data_dir.join("gitfox.env");
            let workhorse_toml = data_dir.join("workhorse.toml");
            
            // 如果 gitfox.toml 存在，检查是否需要版本升级
            if gitfox_toml.exists() {
                println!("📂 Found existing gitfox.toml, checking version...");
                
                // 使用 load_raw 获取原始版本（不执行迁移）
                match unified_config::GitFoxConfig::load_raw(&gitfox_toml) {
                    Ok(raw_config) => {
                        let original_version = raw_config.version.clone();
                        
                        if original_version == CONFIG_VERSION {
                            println!("✅ Configuration is already at latest version ({})", CONFIG_VERSION);
                            return Ok(());
                        }
                        
                        println!("🔄 Upgrading configuration from {} to {}...", original_version, CONFIG_VERSION);
                        
                        // 备份旧配置（使用原始版本号）
                        let backup_path = data_dir.join(format!("gitfox.toml.v{}.bak", original_version));
                        fs::copy(&gitfox_toml, &backup_path)?;
                        println!("   Backup saved to: {}", backup_path.display());
                        
                        // 使用 load_with_migration 执行迁移
                        let result = unified_config::GitFoxConfig::load_with_migration(&gitfox_toml)?;
                        
                        // 保存升级后的配置（只更新版本号和迁移修改的字段）
                        result.config.save_migration(&gitfox_toml)?;
                        
                        println!("\n✅ Configuration upgraded from {} to {}!", original_version, CONFIG_VERSION);
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

