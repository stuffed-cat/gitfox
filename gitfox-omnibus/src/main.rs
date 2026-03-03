//! GitFox Omnibus - Packaging tool for creating self-contained GitFox binary
//!
//! # 用法
//!
//! ```bash
//! # 构建超级二进制
//! gitfox-omnibus build --output ./gitfox
//!
//! # 指定 musl 目标
//! gitfox-omnibus build --target x86_64-unknown-linux-musl --output ./gitfox
//!
//! # 跳过前端构建（使用已有的 dist）
//! gitfox-omnibus build --skip-frontend --output ./gitfox
//!
//! # 构建包含内置依赖的完整版本 (PostgreSQL, Redis, Nginx)
//! gitfox-omnibus build --bundled-deps --output ./gitfox
//! ```
//!
//! # 生成的超级二进制
//!
//! 最终生成的二进制内嵌：
//! - devops (后端 API)
//! - gitfox-workhorse (反向代理)
//! - gitfox-shell (SSH 访问)
//! - frontend/dist (Vue SPA)
//! - webide/dist (VS Code Web)
//! - migrations/*.sql
//! - （可选）内置依赖：PostgreSQL, Redis, Nginx
//!
//! 运行时会解压到 data_dir 然后启动各组件。

mod build;
mod deps;
mod stub;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "gitfox-omnibus")]
#[command(author = "GitFox Team")]
#[command(version)]
#[command(about = "GitFox Omnibus - Create self-contained GitFox binary")]
struct Cli {
    /// 启用调试日志
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 构建超级二进制
    Build {
        /// 输出文件路径
        #[arg(short, long, default_value = "./gitfox")]
        output: PathBuf,

        /// Rust 编译目标 (用于静态链接)
        #[arg(short, long, default_value = "x86_64-unknown-linux-musl")]
        target: String,

        /// 工作区根目录 (包含 frontend, webide, src 等)
        #[arg(short, long)]
        workspace: Option<PathBuf>,

        /// 跳过前端构建
        #[arg(long)]
        skip_frontend: bool,

        /// 跳过 WebIDE 构建
        #[arg(long)]
        skip_webide: bool,

        /// 跳过 Rust 二进制编译
        #[arg(long)]
        skip_rust: bool,

        /// 使用 release 模式 (默认)
        #[arg(long, default_value = "true")]
        release: bool,

        /// 保留临时文件 (调试用)
        #[arg(long)]
        keep_temp: bool,

        /// 构建并嵌入内置依赖 (PostgreSQL, Redis, Nginx)
        /// 启用后可在 gitfox.toml 中配置使用内置或外置服务
        #[arg(long)]
        bundled_deps: bool,

        /// 只构建指定的内置依赖 (逗号分隔: postgresql,redis,nginx)
        /// 默认构建全部。仅在 --bundled-deps 时有效
        #[arg(long, value_delimiter = ',')]
        deps: Vec<String>,

        /// 跳过内置依赖构建（使用已缓存的）
        #[arg(long)]
        skip_deps_build: bool,
    },

    /// 列出会被打包的组件
    List {
        /// 工作区根目录
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    /// 验证工作区结构
    Verify {
        /// 工作区根目录
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    /// 单独构建内置依赖（不构建完整二进制）
    BuildDeps {
        /// 工作区根目录
        #[arg(short, long)]
        workspace: Option<PathBuf>,

        /// 输出目录
        #[arg(short, long, default_value = ".build/deps/output")]
        output: PathBuf,

        /// 构建 PostgreSQL
        #[arg(long)]
        postgresql: bool,

        /// 构建 Redis
        #[arg(long)]
        redis: bool,

        /// 构建 Nginx
        #[arg(long)]
        nginx: bool,

        /// 构建全部依赖
        #[arg(long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("gitfox_omnibus={}", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    match cli.command {
        Commands::Build {
            output,
            target,
            workspace,
            skip_frontend,
            skip_webide,
            skip_rust,
            release,
            keep_temp,
            bundled_deps,
            deps,
            skip_deps_build,
        } => {
            let workspace = workspace.unwrap_or_else(|| find_workspace_root());

            // 解析要构建的依赖
            let (build_pg, build_redis, build_nginx) = if bundled_deps {
                if deps.is_empty() {
                    // 默认构建全部
                    (true, true, true)
                } else {
                    (
                        deps.iter().any(|d| d == "postgresql" || d == "pg"),
                        deps.iter().any(|d| d == "redis"),
                        deps.iter().any(|d| d == "nginx"),
                    )
                }
            } else {
                (false, false, false)
            };

            let config = build::BuildConfig {
                workspace_root: workspace,
                output_path: output,
                target,
                skip_frontend,
                skip_webide,
                skip_rust,
                release,
                keep_temp,
                bundled_deps,
                build_postgresql: build_pg,
                build_redis,
                build_nginx,
                skip_deps_build,
            };

            build::run_build(config).await?;
        }

        Commands::List { workspace } => {
            let workspace = workspace.unwrap_or_else(|| find_workspace_root());
            list_components(&workspace)?;
        }

        Commands::Verify { workspace } => {
            let workspace = workspace.unwrap_or_else(|| find_workspace_root());
            verify_workspace(&workspace)?;
        }

        Commands::BuildDeps {
            workspace,
            output,
            postgresql,
            redis,
            nginx,
            all,
        } => {
            let workspace = workspace.unwrap_or_else(|| find_workspace_root());
            let omnibus_dir = workspace.join("gitfox-omnibus");
            let work_dir = omnibus_dir.join(".build/deps");

            let config = deps::DepsConfig {
                build_postgresql: all || postgresql,
                build_redis: all || redis,
                build_nginx: all || nginx,
                work_dir,
                output_dir: output,
                target: "x86_64-unknown-linux-musl".to_string(),
                use_cache: true,
                jobs: 0,
            };

            if !config.build_postgresql && !config.build_redis && !config.build_nginx {
                eprintln!("Please specify at least one dependency to build:");
                eprintln!("  --postgresql, --redis, --nginx, or --all");
                return Ok(());
            }

            let result = deps::build_deps(&config)?;
            
            println!("\nBuild completed:");
            if result.postgresql.is_some() {
                println!("  ✓ PostgreSQL");
            }
            if result.redis.is_some() {
                println!("  ✓ Redis");
            }
            if result.nginx.is_some() {
                println!("  ✓ Nginx");
            }
        }
    }

    Ok(())
}

/// 查找工作区根目录
fn find_workspace_root() -> PathBuf {
    // 从当前目录向上查找包含 Cargo.toml 和 frontend 的目录
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    loop {
        if current.join("Cargo.toml").exists()
            && current.join("frontend").exists()
            && current.join("src").exists()
        {
            return current;
        }

        if !current.pop() {
            // 找不到，返回当前目录
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

/// 列出工作区组件
fn list_components(workspace: &PathBuf) -> Result<()> {
    println!("Workspace: {}", workspace.display());
    println!();
    println!("Components to be packaged:");
    println!();

    // Rust binaries
    println!("  Rust Binaries:");
    println!("    - devops (main backend)");
    println!("    - gitfox-workhorse (reverse proxy)");
    println!("    - gitfox-shell (SSH access)");
    println!("    - gitfox-shell-authorized-keys-check");
    println!();

    // Frontend
    let frontend = workspace.join("frontend");
    if frontend.exists() {
        let dist = frontend.join("dist");
        if dist.exists() {
            let count = walkdir::WalkDir::new(&dist)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count();
            println!("  Frontend: {} files in dist/", count);
        } else {
            println!("  Frontend: (not built yet)");
        }
    } else {
        println!("  Frontend: NOT FOUND");
    }

    // WebIDE
    let webide = workspace.join("webide");
    if webide.exists() {
        let dist = webide.join("dist");
        if dist.exists() {
            let count = walkdir::WalkDir::new(&dist)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count();
            println!("  WebIDE: {} files in dist/", count);
        } else {
            println!("  WebIDE: (not built yet)");
        }
    } else {
        println!("  WebIDE: NOT FOUND");
    }

    // Migrations
    let migrations = workspace.join("migrations");
    if migrations.exists() {
        let count = std::fs::read_dir(&migrations)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "sql")
            })
            .count();
        println!("  Migrations: {} SQL files", count);
    } else {
        println!("  Migrations: NOT FOUND");
    }

    Ok(())
}

/// 验证工作区
fn verify_workspace(workspace: &PathBuf) -> Result<()> {
    println!("Verifying workspace: {}\n", workspace.display());

    let mut errors = Vec::new();

    // 检查必需的目录和文件
    let checks = [
        ("Cargo.toml", true),
        ("src/main.rs", true),
        ("frontend/package.json", true),
        ("gitfox-workhorse/Cargo.toml", true),
        ("gitfox-shell/Cargo.toml", true),
        ("migrations", false), // 目录
    ];

    for (path, is_file) in checks {
        let full_path = workspace.join(path);
        let exists = if is_file {
            full_path.is_file()
        } else {
            full_path.is_dir()
        };

        if exists {
            println!("  ✓ {}", path);
        } else {
            println!("  ✗ {} (MISSING)", path);
            errors.push(path);
        }
    }

    // 检查可选组件
    let webide = workspace.join("webide/package.json");
    if webide.exists() {
        println!("  ✓ webide/package.json");
    } else {
        println!("  - webide/package.json (optional, not found)");
    }

    println!();

    if errors.is_empty() {
        println!("Workspace verification passed!");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Workspace verification failed: missing {}",
            errors.join(", ")
        ))
    }
}
