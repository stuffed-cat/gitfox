//! 内置依赖构建模块
//!
//! 负责构建 PostgreSQL、Redis、Nginx 的 musl 静态链接版本。
//!
//! # 构建流程
//!
//! 1. 克隆镜像仓库（预配置好的 musl 编译版本）
//! 2. 应用必要的补丁
//! 3. 使用 musl-gcc 编译为静态二进制
//! 4. 收集二进制和必要的配置文件
//!
//! # 镜像仓库结构
//!
//! 每个镜像仓库包含：
//! - 源代码（可能已打补丁支持 musl）
//! - build.sh - 构建脚本
//! - config/ - 默认配置模板
//!
//! # 依赖
//!
//! 构建需要以下工具：
//! - musl-gcc, musl-g++ (musl-tools)
//! - make, autoconf, automake
//! - perl (PostgreSQL 需要)
//! - pkg-config

use anyhow::{Context, Result};
use git2::Repository;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use tracing::{info, warn};
use flate2::read::GzDecoder;
use tar::Archive;

/// 全局工具链编译器路径
static MUSL_GCC: OnceLock<String> = OnceLock::new();
static MUSL_GXX: OnceLock<String> = OnceLock::new();

/// 获取 musl-gcc 路径
fn get_musl_gcc() -> &'static str {
    MUSL_GCC.get().map(|s| s.as_str()).unwrap_or("musl-gcc")
}

/// 获取 musl-g++ 路径
fn get_musl_gxx() -> &'static str {
    MUSL_GXX.get().map(|s| s.as_str()).unwrap_or("musl-g++")
}

/// 镜像仓库 URL
const NGINX_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/nginx.git";
const POSTGRESQL_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/postgresql.git";
const REDIS_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/redis.git";

// PostgreSQL 依赖镜像
const ZLIB_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/zlib.git";
const NCURSES_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/ncurses.git";
const READLINE_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/readline.git";
const OPENSSL_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/openssl.git";
const ICU_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/icu.git";

// systemd（用于 Redis）
const SYSTEMD_MIRROR: &str = "https://gitfox.studio/gitfox/mirror/systemd.git";

/// 构建配置
#[derive(Debug, Clone)]
pub struct DepsConfig {
    /// 是否构建 PostgreSQL
    pub build_postgresql: bool,
    /// 是否构建 Redis
    pub build_redis: bool,
    /// 是否构建 Nginx
    pub build_nginx: bool,
    /// 工作目录（用于克隆和编译）
    pub work_dir: PathBuf,
    /// 输出目录（放置编译好的二进制）
    pub output_dir: PathBuf,
    /// 目标架构
    pub target: String,
    /// 是否使用缓存（如果已存在则跳过编译）
    pub use_cache: bool,
    /// 并行编译任务数（0 = auto）
    pub jobs: usize,
}

impl Default for DepsConfig {
    fn default() -> Self {
        Self {
            build_postgresql: false,
            build_redis: false,
            build_nginx: false,
            work_dir: PathBuf::from(".build/deps"),
            output_dir: PathBuf::from(".build/deps/output"),
            target: "x86_64-unknown-linux-musl".to_string(),
            use_cache: true,
            jobs: 0,
        }
    }
}

/// 构建结果
#[derive(Debug, Default)]
pub struct DepsOutput {
    /// PostgreSQL 二进制和库
    pub postgresql: Option<PostgresqlOutput>,
    /// Redis 二进制
    pub redis: Option<RedisOutput>,
    /// Nginx 二进制
    pub nginx: Option<NginxOutput>,
    /// 依赖库的 lib 目录（包含所有 .so 文件）
    pub deps_lib_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct PostgresqlOutput {
    /// postgres 主程序
    pub postgres: PathBuf,
    /// initdb
    pub initdb: PathBuf,
    /// pg_ctl
    pub pg_ctl: PathBuf,
    /// psql
    pub psql: PathBuf,
    /// pg_dump
    pub pg_dump: PathBuf,
    /// pg_restore
    pub pg_restore: PathBuf,
    /// lib 目录（共享库）
    pub lib_dir: PathBuf,
    /// share 目录（时区、编码等）
    pub share_dir: PathBuf,
    /// 默认配置文件
    pub default_config: PathBuf,
}

#[derive(Debug)]
pub struct RedisOutput {
    /// redis-server
    pub server: PathBuf,
    /// redis-cli
    pub cli: PathBuf,
    /// 默认配置
    pub default_config: PathBuf,
}

#[derive(Debug)]
pub struct NginxOutput {
    /// nginx 主程序
    pub nginx: PathBuf,
    /// 默认配置目录
    pub conf_dir: PathBuf,
    /// mime.types
    pub mime_types: PathBuf,
}

/// musl 工具链 URL (x86_64)
const MUSL_TOOLCHAIN_URL: &str = "https://musl.cc/x86_64-linux-musl-cross.tgz";

/// 下载并安装 musl 工具链
fn download_musl_toolchain(toolchain_dir: &Path) -> Result<PathBuf> {
    info!("musl toolchain not found in PATH, downloading from musl.cc...");
    
    // 创建工具链目录
    fs::create_dir_all(toolchain_dir)?;
    
    let tarball_path = toolchain_dir.join("musl-toolchain.tar.gz");
    
    // 检查是否已下载
    if !tarball_path.exists() {
        info!("Downloading musl toolchain from {}...", MUSL_TOOLCHAIN_URL);
        info!("This may take a few minutes (approx. 115 MB)...");
        
        let response = reqwest::blocking::get(MUSL_TOOLCHAIN_URL)
            .context("Failed to download musl toolchain")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to download: HTTP {}", response.status()));
        }
        
        let bytes = response.bytes().context("Failed to read response")?;
        fs::write(&tarball_path, &bytes).context("Failed to save tarball")?;
        info!("Download complete");
    } else {
        info!("Using cached toolchain tarball");
    }
    
    // 解压目录
    let extract_dir = toolchain_dir.join("x86_64-linux-musl-cross");
    
    if !extract_dir.exists() {
        info!("Extracting toolchain...");
        let tar_gz = fs::File::open(&tarball_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(toolchain_dir)?;
        info!("Toolchain extracted to {}", extract_dir.display());
    } else {
        info!("Using cached extracted toolchain");
    }
    
    let bin_dir = extract_dir.join("bin");
    if !bin_dir.exists() {
        return Err(anyhow::anyhow!("Toolchain bin directory not found after extraction"));
    }
    
    Ok(bin_dir)
}

/// 检查构建依赖
/// 返回 musl 工具链的 bin 目录路径（如果是下载的）
pub fn check_build_deps(work_dir: &Path) -> Result<Option<PathBuf>> {
    info!("Checking build dependencies...");

    let mut toolchain_bin: Option<PathBuf> = None;

    // 检查 musl-gcc 和 musl-g++
    let has_musl_gcc = which::which("musl-gcc").is_ok();
    let has_musl_gxx = which::which("musl-g++").is_ok();

    if has_musl_gcc && has_musl_gxx {
        info!("Found musl-gcc and musl-g++ in PATH");
    } else {
        warn!("musl-gcc or musl-g++ not found in system PATH");
        
        // 自动下载工具链
        let toolchain_dir = work_dir.join("musl-toolchain");
        let bin_dir = download_musl_toolchain(&toolchain_dir)?;
        
        // 验证工具链
        let gcc_path = bin_dir.join("x86_64-linux-musl-gcc");
        let gxx_path = bin_dir.join("x86_64-linux-musl-g++");
        
        if !gcc_path.exists() || !gxx_path.exists() {
            return Err(anyhow::anyhow!(
                "Downloaded toolchain is incomplete (missing gcc or g++)"
            ));
        }
        
        info!("musl toolchain ready: {}", bin_dir.display());
        toolchain_bin = Some(bin_dir);
    }

    // 检查 make
    which::which("make").context("make not found. Please install: apt install build-essential")?;

    // 检查 git
    which::which("git").context("git not found. Please install: apt install git")?;

    // 检查 perl (PostgreSQL 需要)
    which::which("perl").context("perl not found (required for PostgreSQL). Please install: apt install perl")?;

    info!("All build dependencies satisfied");
    Ok(toolchain_bin)
}

/// 构建所有启用的依赖
pub fn build_deps(config: &DepsConfig) -> Result<DepsOutput> {
    info!("Building bundled dependencies...");
    info!("Work directory: {}", config.work_dir.display());
    info!("Output directory: {}", config.output_dir.display());

    // 检查构建依赖，可能返回下载的工具链路径
    let toolchain_bin = check_build_deps(&config.work_dir)?;
    
    // 确定使用的编译器并设置全局变量
    if let Some(ref bin_dir) = toolchain_bin {
        let gcc = bin_dir.join("x86_64-linux-musl-gcc");
        let gxx = bin_dir.join("x86_64-linux-musl-g++");
        let gcc_str = gcc.display().to_string();
        let gxx_str = gxx.display().to_string();
        
        MUSL_GCC.set(gcc_str.clone()).map_err(|_| anyhow::anyhow!("Failed to set MUSL_GCC"))?;
        MUSL_GXX.set(gxx_str.clone()).map_err(|_| anyhow::anyhow!("Failed to set MUSL_GXX"))?;
        
        info!("Using CC: {}", gcc_str);
        info!("Using CXX: {}", gxx_str);
    } else {
        info!("Using system musl-gcc and musl-g++");
    }

    // 创建目录
    fs::create_dir_all(&config.work_dir)?;
    fs::create_dir_all(&config.output_dir)?;

    let mut output = DepsOutput::default();
    
    // 记录依赖库 lib 目录（包含 .so 文件）
    let deps_install_dir = config.work_dir.join("deps-install");
    output.deps_lib_dir = Some(deps_install_dir.join("lib"));

    // 计算并行任务数
    let jobs = if config.jobs == 0 {
        num_cpus()
    } else {
        config.jobs
    };
    info!("Using {} parallel jobs", jobs);

    // 构建 PostgreSQL
    if config.build_postgresql {
        info!("Building PostgreSQL...");
        output.postgresql = Some(build_postgresql(config, jobs)?);
    }

    // 构建 Redis
    if config.build_redis {
        info!("Building Redis...");
        output.redis = Some(build_redis(config, jobs)?);
    }

    // 构建 Nginx
    if config.build_nginx {
        info!("Building Nginx...");
        output.nginx = Some(build_nginx(config, jobs)?);
    }

    info!("Dependency build completed!");
    Ok(output)
}

/// 克隆或更新镜像仓库
fn clone_or_update(url: &str, dest: &Path) -> Result<()> {
    // 尝试打开现有仓库，如果失败则说明需要 clone
    if let Ok(repo) = Repository::open(dest) {
        info!("Updating existing repository: {}", dest.display());
        
        // 查找 origin remote
        let mut remote = repo.find_remote("origin")
            .with_context(|| "Failed to find 'origin' remote")?;
        
        // 创建进度条
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        
        // 配置 fetch options：启用多线程对象处理
        let mut fetch_opts = git2::FetchOptions::new();
        
        fetch_opts.remote_callbacks({
            let pb = pb.clone();
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.transfer_progress(move |stats| {
                if stats.received_objects() == stats.total_objects() {
                    // 开始解析 deltas
                    pb.set_length(stats.total_deltas() as u64);
                    pb.set_position(stats.indexed_deltas() as u64);
                    pb.set_message(format!("Resolving deltas"));
                } else {
                    // 接收对象阶段
                    pb.set_length(stats.total_objects() as u64);
                    pb.set_position(stats.received_objects() as u64);
                    pb.set_message(format!("Receiving objects"));
                }
                true
            });
            callbacks
        });
        
        // Fetch 远程分支（libgit2 会自动使用多线程处理对象）
        remote.fetch(&["refs/heads/*:refs/heads/*"], Some(&mut fetch_opts), None)
            .with_context(|| "Failed to fetch from origin")?;
        
        pb.finish_with_message("Fetch complete");
        
        // 获取当前分支
        let head = repo.head()
            .with_context(|| "Failed to get HEAD reference")?;
        
        if !head.is_branch() {
            warn!("HEAD is not a branch, skipping fast-forward");
            return Ok(());
        }
        
        let branch_name = head.shorthand()
            .ok_or_else(|| anyhow::anyhow!("Failed to get branch name"))?;
        
        // 查找对应的远程分支
        let remote_branch_name = format!("refs/remotes/origin/{}", branch_name);
        let remote_ref = repo.find_reference(&remote_branch_name)
            .with_context(|| format!("Failed to find remote branch: {}", remote_branch_name))?;
        
        let remote_commit = remote_ref.peel_to_commit()
            .with_context(|| "Failed to get remote commit")?;
        
        // Fast-forward merge (设置 HEAD 到远程 commit)
        let local_commit = head.peel_to_commit()
            .with_context(|| "Failed to get local commit")?;
        
        if repo.graph_descendant_of(remote_commit.id(), local_commit.id())? {
            warn!("Remote is behind local, skipping update");
        } else if repo.graph_descendant_of(local_commit.id(), remote_commit.id())? {
            // 可以 fast-forward
            repo.reference(
                head.name().ok_or_else(|| anyhow::anyhow!("Invalid HEAD reference"))?,
                remote_commit.id(),
                true, // force
                "Fast-forward merge"
            )?;
            
            // 更新工作目录
            repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
                .with_context(|| "Failed to checkout HEAD")?;
            
            info!("Successfully updated repository to {}", remote_commit.id());
        } else {
            warn!("Cannot fast-forward, branches have diverged. Keeping local version.");
        }
    } else {
        info!("Cloning: {} -> {}", url, dest.display());
        
        // 创建进度条
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        
        // 配置 fetch options：启用多线程对象处理
        let mut fetch_opts = git2::FetchOptions::new();
        
        fetch_opts.remote_callbacks({
            let pb = pb.clone();
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.transfer_progress(move |stats| {
                if stats.received_objects() == stats.total_objects() {
                    // 开始解析 deltas
                    pb.set_length(stats.total_deltas() as u64);
                    pb.set_position(stats.indexed_deltas() as u64);
                    pb.set_message(format!("Resolving deltas"));
                } else {
                    // 接收对象阶段
                    pb.set_length(stats.total_objects() as u64);
                    pb.set_position(stats.received_objects() as u64);
                    pb.set_message(format!("Receiving objects"));
                }
                true
            });
            callbacks
        });
        
        // 使用 RepoBuilder 配置 clone
        git2::build::RepoBuilder::new()
            .fetch_options(fetch_opts)
            .clone(url, dest)
            .with_context(|| format!("Failed to clone {} to {}", url, dest.display()))?;
        
        pb.finish_with_message("Clone complete");
    }
    Ok(())
}

/// 构建 PostgreSQL 静态依赖
fn build_postgresql_deps(config: &DepsConfig, jobs: usize) -> Result<PathBuf> {
    let deps_dir = config.output_dir.join("postgresql-deps");
    
    fs::create_dir_all(&deps_dir)?;
    let deps_include = deps_dir.join("include");
    let deps_lib = deps_dir.join("lib");
    fs::create_dir_all(&deps_include)?;
    fs::create_dir_all(&deps_lib)?;
    
    // 所有依赖同时生成 .a 和 .so
    // PostgreSQL 会静态链接 .a，我们只打包 .so 供运行时使用
    
    // 构建 zlib
    info!("Building zlib (static + shared)...");
    build_zlib(config, jobs, &deps_dir)?;
    
    // 构建 ncurses (readline 依赖)
    info!("Building ncurses (static + shared)...");
    build_ncurses(config, jobs, &deps_dir)?;
    
    // 构建 readline
    info!("Building readline (static + shared)...");
    build_readline(config, jobs, &deps_dir)?;
    
    // 构建 OpenSSL
    info!("Building OpenSSL (static + shared)...");
    build_openssl(config, jobs, &deps_dir)?;
    
    // 构建 ICU
    info!("Building ICU (static + shared)...");
    build_icu(config, jobs, &deps_dir)?;
    
    // 复制 ld-musl 动态链接器到 lib 目录
    // 这是运行时必需的，用于启动 musl 动态链接的二进制
    copy_musl_loader(&deps_lib)?;
    
    Ok(deps_dir)
}

/// 从 musl-toolchain 复制 ld-musl 动态链接器和运行时库
fn copy_musl_loader(lib_dir: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;
    
    let cxx_path = get_musl_gxx();
    if !cxx_path.contains("musl-toolchain") {
        // 使用系统 musl-gcc，尝试从系统路径复制
        let system_ld = Path::new("/lib/ld-musl-x86_64.so.1");
        if system_ld.exists() {
            let dest = lib_dir.join("ld-musl-x86_64.so.1");
            if !dest.exists() {
                fs::copy(system_ld, &dest)?;
                info!("Copied ld-musl from system path");
            }
        }
        return Ok(());
    }
    
    // 从下载的 musl-toolchain 复制
    if let Some(parent) = Path::new(cxx_path).parent() {
        if let Some(toolchain_root) = parent.parent() {
            let musl_lib = toolchain_root.join("x86_64-linux-musl/lib");
            
            // 先复制 libc.so（这是 musl 的核心）
            let libc_src = musl_lib.join("libc.so");
            let libc_dest = lib_dir.join("libc.so");
            if libc_src.exists() && !libc_dest.exists() {
                fs::copy(&libc_src, &libc_dest)?;
                info!("Copied libc.so to deps lib");
            }
            
            // 创建 ld-musl-x86_64.so.1 符号链接指向 libc.so
            // musl 的动态链接器就是 libc.so 本身
            let ld_musl_dest = lib_dir.join("ld-musl-x86_64.so.1");
            if !ld_musl_dest.exists() {
                symlink("libc.so", &ld_musl_dest)?;
                info!("Created ld-musl-x86_64.so.1 symlink -> libc.so");
            }
            
            // 复制 C++ 运行时库
            for entry in fs::read_dir(&musl_lib)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path();
                
                // 跳过符号链接（我们已经手动处理了 ld-musl）
                if path.is_symlink() {
                    continue;
                }
                
                // 复制 libstdc++, libgcc_s 等运行时必需的库
                if name.starts_with("libstdc++.so") || name.starts_with("libgcc_s.so") {
                    let dest = lib_dir.join(&name);
                    if !dest.exists() {
                        fs::copy(&path, &dest)?;
                        info!("Copied {} to deps lib", name);
                    }
                }
            }
            
            // 同时检查 gcc lib 目录下的 libstdc++ 和 libgcc_s
            let gcc_lib = toolchain_root.join("lib/gcc/x86_64-linux-musl/11.2.1");
            if gcc_lib.exists() {
                for entry in fs::read_dir(&gcc_lib)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_symlink() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("libstdc++.so") || name.starts_with("libgcc_s.so") {
                        let dest = lib_dir.join(&name);
                        if !dest.exists() {
                            fs::copy(&path, &dest)?;
                            info!("Copied {} to deps lib", name);
                        }
                    }
                }
            }
            
            // 还需要检查 x86_64-linux-musl/lib64 目录
            let musl_lib64 = toolchain_root.join("x86_64-linux-musl/lib64");
            if musl_lib64.exists() {
                for entry in fs::read_dir(&musl_lib64)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_symlink() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("libstdc++.so") || name.starts_with("libgcc_s.so") {
                        let dest = lib_dir.join(&name);
                        if !dest.exists() {
                            fs::copy(&path, &dest)?;
                            info!("Copied {} to deps lib", name);
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

/// 构建 zlib（同时生成 .a 和 .so）
fn build_zlib(config: &DepsConfig, jobs: usize, deps_dir: &Path) -> Result<()> {
    let src_dir = config.work_dir.join("zlib");
    clone_or_update(ZLIB_MIRROR, &src_dir)?;
    
    let build_script = src_dir.join("build-musl.sh");
    if build_script.exists() {
        let status = Command::new("bash")
            .arg(&build_script)
            .env("INSTALL_DIR", deps_dir)
            .env("JOBS", jobs.to_string())
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("zlib build script failed"));
        }
    } else {
        // zlib 不带 --static 会同时生成 .a 和 .so
        let status = Command::new("./configure")
            .arg(&format!("--prefix={}", deps_dir.display()))
            .env("CC", get_musl_gcc())
            .env("CFLAGS", "-Os -fPIC")
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("zlib configure failed"));
        }
        
        let status = Command::new("make")
            .args(["-j", &jobs.to_string()])
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("zlib make failed"));
        }
        
        let status = Command::new("make")
            .arg("install")
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("zlib install failed"));
        }
    }
    Ok(())
}

/// 构建 ncurses（同时生成 .a 和 .so）
fn build_ncurses(config: &DepsConfig, jobs: usize, deps_dir: &Path) -> Result<()> {
    let src_dir = config.work_dir.join("ncurses");
    clone_or_update(NCURSES_MIRROR, &src_dir)?;
    
    let build_script = src_dir.join("build-musl.sh");
    if build_script.exists() {
        let status = Command::new("bash")
            .arg(&build_script)
            .env("INSTALL_DIR", deps_dir)
            .env("JOBS", jobs.to_string())
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("ncurses build script failed"));
        }
    } else {
        // --with-shared 同时生成 .a 和 .so
        let status = Command::new("./configure")
            .arg(&format!("--prefix={}", deps_dir.display()))
            .arg("--with-shared")
            .arg("--without-debug")
            .arg("--without-cxx-binding")
            .arg("--without-ada")
            .arg("--enable-widec")
            .arg(&format!("--with-default-terminfo-dir={}/share/terminfo", deps_dir.display()))
            .arg(&format!("--with-terminfo-dirs={}/share/terminfo", deps_dir.display()))
            .env("CC", get_musl_gcc())
            .env("CFLAGS", "-Os -fPIC")
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("ncurses configure failed"));
        }
        
        let status = Command::new("make")
            .args(["-j", &jobs.to_string()])
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("ncurses make failed"));
        }
        
        let status = Command::new("make")
            .arg("install")
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("ncurses install failed"));
        }
    }
    Ok(())
}

/// 构建 readline（同时生成 .a 和 .so）
fn build_readline(config: &DepsConfig, jobs: usize, deps_dir: &Path) -> Result<()> {
    let src_dir = config.work_dir.join("readline");
    clone_or_update(READLINE_MIRROR, &src_dir)?;
    
    let build_script = src_dir.join("build-musl.sh");
    if build_script.exists() {
        let status = Command::new("bash")
            .arg(&build_script)
            .env("INSTALL_DIR", deps_dir)
            .env("JOBS", jobs.to_string())
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("readline build script failed"));
        }
    } else {
        // 同时生成 .a 和 .so
        let status = Command::new("./configure")
            .arg(&format!("--prefix={}", deps_dir.display()))
            .arg("--enable-shared")
            .arg("--enable-static")
            .env("CC", get_musl_gcc())
            .env("CFLAGS", format!("-Os -fPIC -I{}", deps_dir.join("include").display()))
            .env("LDFLAGS", format!("-L{}", deps_dir.join("lib").display()))
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("readline configure failed"));
        }
        
        let status = Command::new("make")
            .args(["-j", &jobs.to_string()])
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("readline make failed"));
        }
        
        let status = Command::new("make")
            .arg("install")
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("readline install failed"));
        }
    }
    Ok(())
}

/// 构建 OpenSSL（同时生成 .a 和 .so）
fn build_openssl(config: &DepsConfig, jobs: usize, deps_dir: &Path) -> Result<()> {
    let src_dir = config.work_dir.join("openssl");
    clone_or_update(OPENSSL_MIRROR, &src_dir)?;
    
    let build_script = src_dir.join("build-musl.sh");
    if build_script.exists() {
        let status = Command::new("bash")
            .arg(&build_script)
            .env("INSTALL_DIR", deps_dir)
            .env("JOBS", jobs.to_string())
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("OpenSSL build script failed"));
        }
    } else {
        // shared 同时生成 .a 和 .so
        let status = Command::new("./Configure")
            .arg("linux-x86_64")
            .arg(&format!("--prefix={}", deps_dir.display()))
            .arg("shared")
            .env("CC", get_musl_gcc())
            .env("CFLAGS", "-Os -fPIC")
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("OpenSSL configure failed"));
        }
        
        let status = Command::new("make")
            .args(["-j", &jobs.to_string()])
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("OpenSSL make failed"));
        }
        
        let status = Command::new("make")
            .arg("install_sw")
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("OpenSSL install failed"));
        }
    }
    Ok(())
}

/// 构建 ICU（同时生成 .a 和 .so）
fn build_icu(config: &DepsConfig, jobs: usize, deps_dir: &Path) -> Result<()> {
    let src_dir = config.work_dir.join("icu");
    clone_or_update(ICU_MIRROR, &src_dir)?;
    
    let build_script = src_dir.join("build-musl.sh");
    if build_script.exists() {
        let status = Command::new("bash")
            .arg(&build_script)
            .env("INSTALL_DIR", deps_dir)
            .env("JOBS", jobs.to_string())
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("ICU build script failed"));
        }
    } else {
        // ICU 源码在 icu4c/source 子目录
        let source_dir = src_dir.join("icu4c/source");
        let actual_dir = if source_dir.exists() { 
            source_dir 
        } else {
            // fallback: 可能是 tarball 解压后的 source 目录
            let alt_dir = src_dir.join("source");
            if alt_dir.exists() { alt_dir } else { src_dir.clone() }
        };
        
        // 同时生成 .a 和 .so
        let status = Command::new("./configure")
            .arg(&format!("--prefix={}", deps_dir.display()))
            .arg("--enable-shared")
            .arg("--enable-static")
            .arg("--disable-samples")
            .arg("--disable-tests")
            .env("CC", get_musl_gcc())
            .env("CXX", get_musl_gxx())
            .env("CFLAGS", "-Os -fPIC")
            .env("CXXFLAGS", "-Os -fPIC")
            .current_dir(&actual_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("ICU configure failed"));
        }
        
        let status = Command::new("make")
            .args(["-j", &jobs.to_string()])
            .current_dir(&actual_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("ICU make failed"));
        }
        
        let status = Command::new("make")
            .arg("install")
            .current_dir(&actual_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("ICU install failed"));
        }
    }
    Ok(())
}

/// 构建 PostgreSQL
fn build_postgresql(config: &DepsConfig, jobs: usize) -> Result<PostgresqlOutput> {
    let src_dir = config.work_dir.join("postgresql");
    let build_dir = src_dir.join("build");
    let install_dir = config.output_dir.join("postgresql");

    // 克隆源码
    clone_or_update(POSTGRESQL_MIRROR, &src_dir)?;

    // 检查是否有预制的构建脚本
    let build_script = src_dir.join("build-musl.sh");
    if build_script.exists() {
        info!("Using pre-configured build script");
        let status = Command::new("bash")
            .arg(&build_script)
            .env("INSTALL_DIR", &install_dir)
            .env("JOBS", jobs.to_string())
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("PostgreSQL build script failed"));
        }
    } else {
        // 先构建依赖
        info!("Building PostgreSQL static dependencies...");
        let deps_dir = build_postgresql_deps(config, jobs)?;
        
        // 手动构建
        fs::create_dir_all(&build_dir)?;
        fs::create_dir_all(&install_dir)?;

        // 配置 (musl 动态链接 + 启用所有功能)
        // 使用 musl libc 但动态链接，避免静态链接的循环依赖问题
        // .so 文件会打包进超级二进制，运行时释放到 datadir
        info!("Configuring PostgreSQL with musl dynamic linking...");
        let configure = src_dir.join("configure");
        
        // OpenSSL 可能安装在 lib64，添加到库搜索路径
        let lib_paths = format!("{}/lib:{}/lib64", deps_dir.display(), deps_dir.display());
        let include_paths = format!("{}/include:{}/include/ncursesw", 
            deps_dir.display(), deps_dir.display());
        
        // 构建 LDFLAGS：
        // - 不用 -static，使用动态链接
        // - 设置 RPATH 为 $ORIGIN/../lib 让二进制在相对路径找 .so
        let mut ldflags = format!(
            "-L{}/lib -L{}/lib64 -Wl,-rpath,'$ORIGIN/../lib'",
            deps_dir.display(), deps_dir.display()
        );
        
        // 如果使用下载的 musl-toolchain，添加其 C++ 标准库路径
        let cxx_path = get_musl_gxx();
        if cxx_path.contains("musl-toolchain") {
            if let Some(parent) = Path::new(cxx_path).parent() {
                if let Some(toolchain_root) = parent.parent() {
                    let toolchain_lib = toolchain_root.join("x86_64-linux-musl/lib");
                    let gcc_lib = toolchain_root.join("lib/gcc/x86_64-linux-musl/11.2.1");
                    
                    if toolchain_lib.exists() {
                        ldflags.push_str(&format!(" -L{}", toolchain_lib.display()));
                        info!("Added C++ stdlib path: {}", toolchain_lib.display());
                    }
                    if gcc_lib.exists() {
                        ldflags.push_str(&format!(" -L{}", gcc_lib.display()));
                        info!("Added gcc lib path: {}", gcc_lib.display());
                    }
                }
            }
        }
        
        // PostgreSQL configure 不支持 --enable-shared/--disable-static
        // 它默认会同时生成 .a 和 .so
        // 需要 --host 因为 musl 编译的程序在 glibc 系统上无法直接运行
        let status = Command::new(&configure)
            .args([
                &format!("--prefix={}", install_dir.display()),
                &format!("--with-includes={}", include_paths),
                &format!("--with-libraries={}", lib_paths),
                "--host=x86_64-linux-musl",     // 交叉编译目标
                "--build=x86_64-linux-gnu",     // 构建系统
                "--with-readline",
                "--with-zlib",
                "--with-openssl",
                "--with-icu",
            ])
            .env("CC", get_musl_gcc())
            .env("CXX", get_musl_gxx())
            .env("AR", "ar")
            .env("CFLAGS", format!("-Os -I{}/include -I{}/include/ncursesw", 
                deps_dir.display(), deps_dir.display()))
            .env("CXXFLAGS", format!("-Os -I{}/include -I{}/include/ncursesw", 
                deps_dir.display(), deps_dir.display()))
            .env("LDFLAGS", &ldflags)
            .env("LIBS", "-lreadline -lncursesw -lstdc++ -lpthread -lm")
            .env("CPPFLAGS", format!("-I{}/include -I{}/include/ncursesw", 
                deps_dir.display(), deps_dir.display()))
            .env("PKG_CONFIG_PATH", format!("{}/lib/pkgconfig:{}/lib64/pkgconfig", 
                deps_dir.display(), deps_dir.display()))
            .current_dir(&build_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("PostgreSQL configure failed"));
        }

        // 编译
        info!("Compiling PostgreSQL...");
        let cxx = get_musl_gxx();
        
        let status = Command::new("make")
            .args(["-j", &jobs.to_string()])
            .arg(format!("LD={}", cxx))  // 使用 C++ 链接器（ICU 需要）
            .current_dir(&build_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("PostgreSQL make failed"));
        }

        // 安装
        info!("Installing PostgreSQL...");
        let status = Command::new("make")
            .arg("install")
            .current_dir(&build_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("PostgreSQL install failed"));
        }
        
        // 复制 musl libc 动态库到 lib 目录
        // PostgreSQL 可执行文件需要 musl libc.so
        info!("Copying musl runtime to PostgreSQL lib directory...");
        let pg_lib_dir = install_dir.join("lib");
        if cxx_path.contains("musl-toolchain") {
            use std::os::unix::fs::symlink;
            
            if let Some(parent) = Path::new(cxx_path).parent() {
                if let Some(toolchain_root) = parent.parent() {
                    let musl_lib = toolchain_root.join("x86_64-linux-musl/lib");
                    
                    // 先复制 libc.so（这是 musl 的核心）
                    let libc_src = musl_lib.join("libc.so");
                    let libc_dest = pg_lib_dir.join("libc.so");
                    if libc_src.exists() && !libc_dest.exists() {
                        fs::copy(&libc_src, &libc_dest)?;
                        info!("Copied libc.so to PostgreSQL lib");
                    }
                    
                    // 创建 ld-musl-x86_64.so.1 符号链接
                    let ld_musl_dest = pg_lib_dir.join("ld-musl-x86_64.so.1");
                    if !ld_musl_dest.exists() {
                        symlink("libc.so", &ld_musl_dest)?;
                        info!("Created ld-musl-x86_64.so.1 symlink");
                    }
                    
                    // 复制其他运行时库（跳过符号链接）
                    for entry in fs::read_dir(&musl_lib)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.is_symlink() {
                            continue;
                        }
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with("libstdc++.so") || name.starts_with("libgcc_s.so") {
                            let dest = pg_lib_dir.join(&name);
                            if !dest.exists() {
                                fs::copy(&path, &dest)?;
                                info!("Copied {} to PostgreSQL lib", name);
                            }
                        }
                    }
                }
            }
        }
    }

    // 创建默认配置
    create_postgresql_config(&install_dir)?;

    info!("PostgreSQL build completed");
    Ok(postgresql_output(&install_dir))
}

fn postgresql_output(install_dir: &Path) -> PostgresqlOutput {
    let bin = install_dir.join("bin");
    PostgresqlOutput {
        postgres: bin.join("postgres"),
        initdb: bin.join("initdb"),
        pg_ctl: bin.join("pg_ctl"),
        psql: bin.join("psql"),
        pg_dump: bin.join("pg_dump"),
        pg_restore: bin.join("pg_restore"),
        lib_dir: install_dir.join("lib"),
        share_dir: install_dir.join("share"),
        default_config: install_dir.join("config/postgresql.conf"),
    }
}

fn create_postgresql_config(install_dir: &Path) -> Result<()> {
    let config_dir = install_dir.join("config");
    fs::create_dir_all(&config_dir)?;

    // 创建优化的默认配置
    let config = r#"# GitFox Bundled PostgreSQL Configuration
# 此配置针对 GitFox 场景优化

# 连接设置
listen_addresses = '127.0.0.1'
port = 5432
max_connections = 100

# 内存设置
shared_buffers = 256MB
work_mem = 4MB
maintenance_work_mem = 64MB
effective_cache_size = 512MB

# WAL 设置
wal_level = replica
max_wal_size = 1GB
min_wal_size = 80MB

# 查询规划
random_page_cost = 1.1
effective_io_concurrency = 200

# 日志设置
log_destination = 'stderr'
logging_collector = on
log_directory = 'log'
log_filename = 'postgresql-%Y-%m-%d.log'
log_rotation_age = 1d
log_rotation_size = 100MB
log_min_duration_statement = 1000

# Locale 设置
lc_messages = 'C'
lc_monetary = 'C'
lc_numeric = 'C'
lc_time = 'C'
"#;

    fs::write(config_dir.join("postgresql.conf"), config)?;

    // pg_hba.conf
    let hba = r#"# GitFox Bundled PostgreSQL HBA Configuration
# TYPE  DATABASE        USER            ADDRESS                 METHOD

# 本地 Unix socket 连接
local   all             all                                     trust

# 本地 IPv4 连接
host    all             all             127.0.0.1/32            md5

# 本地 IPv6 连接
host    all             all             ::1/128                 md5
"#;

    fs::write(config_dir.join("pg_hba.conf"), hba)?;

    Ok(())
}

/// 克隆 systemd 仓库（Redis 只需要头文件）
fn clone_systemd_headers(config: &DepsConfig) -> Result<PathBuf> {
    let systemd_dir = config.work_dir.join("systemd");
    
    if !systemd_dir.exists() {
        info!("Cloning systemd repository for headers...");
        clone_or_update(SYSTEMD_MIRROR, &systemd_dir)?;
    } else {
        info!("systemd headers already available");
    }
    
    Ok(systemd_dir)
}

/// 构建 Redis
fn build_redis(config: &DepsConfig, jobs: usize) -> Result<RedisOutput> {
    let src_dir = config.work_dir.join("redis");
    let install_dir = config.output_dir.join("redis");

    // 克隆源码
    clone_or_update(REDIS_MIRROR, &src_dir)?;
    
    // 克隆 systemd 获取头文件
    let systemd_dir = clone_systemd_headers(config)?;

    fs::create_dir_all(&install_dir)?;
    fs::create_dir_all(install_dir.join("bin"))?;

    // 检查是否有预制的构建脚本
    let build_script = src_dir.join("build-musl.sh");
    if build_script.exists() {
        info!("Using pre-configured build script");
        let status = Command::new("bash")
            .arg(&build_script)
            .env("INSTALL_DIR", &install_dir)
            .env("JOBS", jobs.to_string())
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Redis build script failed"));
        }
    } else {
        // 复用 PostgreSQL 的依赖（OpenSSL, zlib 等）
        info!("Building shared dependencies (OpenSSL, zlib, etc.)...");
        let deps_dir = build_postgresql_deps(config, jobs)?;
        
        // Redis 使用简单的 Makefile，参考 PostgreSQL 的方式使用动态链接
        info!("Compiling Redis with musl (all features enabled)...");
        let cflags = format!("-O2 -fno-lto -Wno-error=deprecated-declarations -I{} -I{}/include", 
            systemd_dir.display(), deps_dir.display());
        
        // 像 PostgreSQL 一样：不用 -static，使用动态链接 + RPATH
        // 这样主二进制和测试模块都能正确编译
        let mut ldflags = format!("-L{}/lib -L{}/lib64 -Wl,-rpath,'$ORIGIN/../lib' -lssl -lcrypto -lz", 
            deps_dir.display(), deps_dir.display());
        
        // 添加 musl toolchain 的 C++ 库路径（如果使用下载的 toolchain）
        let cxx_path = get_musl_gxx();
        if cxx_path.contains("musl-toolchain") {
            if let Some(parent) = Path::new(cxx_path).parent() {
                if let Some(toolchain_root) = parent.parent() {
                    let toolchain_lib = toolchain_root.join("x86_64-linux-musl/lib");
                    let gcc_lib = toolchain_root.join("lib/gcc/x86_64-linux-musl/11.2.1");
                    
                    if toolchain_lib.exists() {
                        ldflags.push_str(&format!(" -L{}", toolchain_lib.display()));
                    }
                    if gcc_lib.exists() {
                        ldflags.push_str(&format!(" -L{}", gcc_lib.display()));
                    }
                }
            }
        }
        
        // 编译所有目标（包括测试模块）
        let status = Command::new("make")
            .args([
                "-j", &jobs.to_string(),
                &format!("CC={}", get_musl_gcc()),
                &format!("CXX={}", get_musl_gxx()),  // C++ 编译器也要用 musl
                &format!("CFLAGS={}", cflags),  // 添加 systemd 和依赖头文件路径，忽略 deprecated 错误
                &format!("LDFLAGS={}", ldflags),  // 动态链接 + RPATH
                "MALLOC=libc",  // musl 不兼容 jemalloc
                "BUILD_TLS=yes",  // 启用 TLS（使用 OpenSSL）
            ])
            .current_dir(&src_dir.join("src"))
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Redis make failed"));
        }

        // 复制二进制
        for bin in ["redis-server", "redis-cli", "redis-benchmark", "redis-check-aof", "redis-check-rdb"] {
            let src = src_dir.join("src").join(bin);
            if src.exists() {
                fs::copy(&src, install_dir.join("bin").join(bin))?;
            }
        }
    }

    // 创建默认配置
    create_redis_config(&install_dir)?;

    info!("Redis build completed");
    Ok(redis_output(&install_dir))
}

fn redis_output(install_dir: &Path) -> RedisOutput {
    let bin = install_dir.join("bin");
    RedisOutput {
        server: bin.join("redis-server"),
        cli: bin.join("redis-cli"),
        default_config: install_dir.join("config/redis.conf"),
    }
}

fn create_redis_config(install_dir: &Path) -> Result<()> {
    let config_dir = install_dir.join("config");
    fs::create_dir_all(&config_dir)?;

    let config = r#"# GitFox Bundled Redis Configuration

# 网络
bind 127.0.0.1
port 6379
tcp-backlog 511
timeout 0
tcp-keepalive 300

# 通用
daemonize no
pidfile /var/run/redis/redis-server.pid
loglevel notice

# 内存
maxmemory 256mb
maxmemory-policy volatile-lru

# 持久化 (RDB)
save 900 1
save 300 10
save 60 10000
stop-writes-on-bgsave-error yes
rdbcompression yes
rdbchecksum yes
dbfilename dump.rdb

# 复制
replica-serve-stale-data yes
replica-read-only yes

# 安全
# 内置模式下不需要密码，因为只监听本地
# requirepass your-strong-password

# 限制
maxclients 1000

# 惰性释放
lazyfree-lazy-eviction no
lazyfree-lazy-expire no
lazyfree-lazy-server-del no
replica-lazy-flush no
"#;

    fs::write(config_dir.join("redis.conf"), config)?;
    Ok(())
}

/// 构建 Nginx
fn build_nginx(config: &DepsConfig, jobs: usize) -> Result<NginxOutput> {
    let src_dir = config.work_dir.join("nginx");
    let install_dir = config.output_dir.join("nginx");

    // 克隆源码
    clone_or_update(NGINX_MIRROR, &src_dir)?;

    fs::create_dir_all(&install_dir)?;

    // 检查是否有预制的构建脚本
    let build_script = src_dir.join("build-musl.sh");
    if build_script.exists() {
        info!("Using pre-configured build script");
        let status = Command::new("bash")
            .arg(&build_script)
            .env("INSTALL_DIR", &install_dir)
            .env("JOBS", jobs.to_string())
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Nginx build script failed"));
        }
    } else {
        // 手动构建
        // Nginx 需要 PCRE 和 zlib，我们假设镜像仓库已包含这些
        info!("Configuring Nginx...");
        let status = Command::new("./configure")
            .args([
                &format!("--prefix={}", install_dir.display()),
                "--with-cc=musl-gcc",
                "--with-cc-opt=-static -Os",
                "--with-ld-opt=-static",
                "--without-http_rewrite_module",  // 需要 PCRE
                "--without-http_gzip_module",     // 需要 zlib
                "--without-http_ssl_module",      // 需要 OpenSSL
                "--without-stream_ssl_module",
                "--without-mail_ssl_module",
                "--http-log-path=/var/log/nginx/access.log",
                "--error-log-path=/var/log/nginx/error.log",
                "--pid-path=/var/run/nginx.pid",
            ])
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Nginx configure failed"));
        }

        info!("Compiling Nginx...");
        let status = Command::new("make")
            .args(["-j", &jobs.to_string()])
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Nginx make failed"));
        }

        info!("Installing Nginx...");
        let status = Command::new("make")
            .arg("install")
            .current_dir(&src_dir)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Nginx install failed"));
        }
    }

    // 创建 GitFox 专用配置
    create_nginx_config(&install_dir)?;

    info!("Nginx build completed");
    Ok(nginx_output(&install_dir))
}

fn nginx_output(install_dir: &Path) -> NginxOutput {
    NginxOutput {
        nginx: install_dir.join("sbin/nginx"),
        conf_dir: install_dir.join("conf"),
        mime_types: install_dir.join("conf/mime.types"),
    }
}

fn create_nginx_config(install_dir: &Path) -> Result<()> {
    let conf_dir = install_dir.join("conf");
    fs::create_dir_all(&conf_dir)?;

    // 主配置
    let nginx_conf = r#"# GitFox Bundled Nginx Configuration
# 此配置作为 GitFox 前端代理

worker_processes auto;
error_log stderr;
pid /var/run/nginx.pid;

events {
    worker_connections 1024;
    use epoll;
    multi_accept on;
}

http {
    include mime.types;
    default_type application/octet-stream;

    # 日志格式
    log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                    '$status $body_bytes_sent "$http_referer" '
                    '"$http_user_agent" "$http_x_forwarded_for"';

    access_log /var/log/nginx/access.log main;

    # 性能优化
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    types_hash_max_size 2048;

    # Gzip（如果编译时启用）
    # gzip on;
    # gzip_vary on;
    # gzip_proxied any;
    # gzip_comp_level 6;
    # gzip_types text/plain text/css text/xml application/json application/javascript application/xml;

    # 客户端限制
    client_max_body_size 1g;
    client_body_buffer_size 128k;

    # 代理缓冲
    proxy_buffering on;
    proxy_buffer_size 4k;
    proxy_buffers 8 32k;
    proxy_busy_buffers_size 64k;

    # 超时设置
    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;

    # 代理头设置
    proxy_set_header Host $http_host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;

    # 包含站点配置
    include conf.d/*.conf;

    # 默认服务器
    server {
        listen 80 default_server;
        listen [::]:80 default_server;
        server_name _;

        # 代理到 Workhorse
        location / {
            proxy_pass http://127.0.0.1:8080;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
        }

        # 静态资源缓存
        location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
            proxy_pass http://127.0.0.1:8080;
            proxy_cache_valid 200 1h;
            add_header Cache-Control "public, max-age=3600";
        }

        # Git HTTP 长连接
        location ~ \.git {
            proxy_pass http://127.0.0.1:8080;
            proxy_http_version 1.1;
            proxy_read_timeout 300s;
            proxy_connect_timeout 75s;
        }
    }
}
"#;

    fs::write(conf_dir.join("nginx.conf"), nginx_conf)?;

    // mime.types
    let mime_types = r#"types {
    text/html                             html htm shtml;
    text/css                              css;
    text/xml                              xml;
    image/gif                             gif;
    image/jpeg                            jpeg jpg;
    application/javascript                js;
    application/json                      json;
    application/atom+xml                  atom;
    application/rss+xml                   rss;
    text/plain                            txt;
    image/png                             png;
    image/svg+xml                         svg svgz;
    image/webp                            webp;
    image/x-icon                          ico;
    font/woff                             woff;
    font/woff2                            woff2;
    application/font-woff                 woff;
    application/font-woff2                woff2;
    application/vnd.ms-fontobject         eot;
    font/ttf                              ttf;
    font/otf                              otf;
    application/octet-stream              bin exe dll;
    application/zip                       zip;
    application/gzip                      gz;
    application/x-tar                     tar;
    application/pdf                       pdf;
}
"#;

    fs::write(conf_dir.join("mime.types"), mime_types)?;

    // 创建 conf.d 目录
    fs::create_dir_all(conf_dir.join("conf.d"))?;

    Ok(())
}

/// 获取 CPU 核心数
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

/// 复制依赖二进制到指定目录
pub fn copy_deps_to_output(output: &DepsOutput, target_dir: &Path) -> Result<()> {
    let deps_dir = target_dir.join("deps");
    fs::create_dir_all(&deps_dir)?;

    if let Some(ref pg) = output.postgresql {
        let pg_dir = deps_dir.join("postgresql");
        fs::create_dir_all(pg_dir.join("bin"))?;
        
        // 复制二进制
        for (name, path) in [
            ("postgres", &pg.postgres),
            ("initdb", &pg.initdb),
            ("pg_ctl", &pg.pg_ctl),
            ("psql", &pg.psql),
            ("pg_dump", &pg.pg_dump),
            ("pg_restore", &pg.pg_restore),
        ] {
            if path.exists() {
                fs::copy(path, pg_dir.join("bin").join(name))?;
            }
        }
        
        // 复制 lib 和 share
        if pg.lib_dir.exists() {
            copy_dir_recursive(&pg.lib_dir, &pg_dir.join("lib"))?;
        }
        if pg.share_dir.exists() {
            copy_dir_recursive(&pg.share_dir, &pg_dir.join("share"))?;
        }
        
        // 复制依赖库的 .so 文件到 PostgreSQL lib 目录
        // 这样运行时 ld-musl 可以通过 LD_LIBRARY_PATH 找到所有动态库
        if let Some(ref deps_lib) = output.deps_lib_dir {
            if deps_lib.exists() {
                let pg_lib = pg_dir.join("lib");
                for entry in fs::read_dir(deps_lib)? {
                    let entry = entry?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    // 只复制 .so 文件（包括 .so.N 和 .so.N.N.N 等）
                    if name.contains(".so") {
                        let dest = pg_lib.join(&name);
                        if !dest.exists() {
                            fs::copy(entry.path(), &dest)?;
                            info!("Copied {} to PostgreSQL lib", name);
                        }
                    }
                }
            }
        }
        
        // 复制配置
        let config_parent = pg.default_config.parent().unwrap();
        if config_parent.exists() {
            copy_dir_recursive(config_parent, &pg_dir.join("config"))?;
        }
        
        info!("PostgreSQL binaries copied to: {}", pg_dir.display());
    }

    if let Some(ref redis) = output.redis {
        let redis_dir = deps_dir.join("redis");
        fs::create_dir_all(redis_dir.join("bin"))?;
        
        // 复制二进制
        if redis.server.exists() {
            fs::copy(&redis.server, redis_dir.join("bin/redis-server"))?;
        }
        if redis.cli.exists() {
            fs::copy(&redis.cli, redis_dir.join("bin/redis-cli"))?;
        }
        
        // 复制配置
        let config_parent = redis.default_config.parent().unwrap();
        if config_parent.exists() {
            copy_dir_recursive(config_parent, &redis_dir.join("config"))?;
        }
        
        info!("Redis binaries copied to: {}", redis_dir.display());
    }

    if let Some(ref nginx) = output.nginx {
        let nginx_dir = deps_dir.join("nginx");
        fs::create_dir_all(nginx_dir.join("sbin"))?;
        
        // 复制二进制
        if nginx.nginx.exists() {
            fs::copy(&nginx.nginx, nginx_dir.join("sbin/nginx"))?;
        }
        
        // 复制配置
        if nginx.conf_dir.exists() {
            copy_dir_recursive(&nginx.conf_dir, &nginx_dir.join("conf"))?;
        }
        
        info!("Nginx binaries copied to: {}", nginx_dir.display());
    }

    Ok(())
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let src_path = entry.path();
        let relative = src_path.strip_prefix(src)?;
        let dst_path = dst.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&dst_path)?;
        } else {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src_path, &dst_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DepsConfig::default();
        assert!(!config.build_postgresql);
        assert!(!config.build_redis);
        assert!(!config.build_nginx);
    }
}
