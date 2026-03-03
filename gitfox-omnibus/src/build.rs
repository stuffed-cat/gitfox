//! Build command implementation
//!
//! 执行完整的打包流程：
//! 1. 编译前端 (Vue)
//! 2. 编译 WebIDE
//! 3. 编译 Rust 二进制 (musl 静态链接)
//! 4. 收集 migrations
//! 5. （可选）编译内置依赖 (PostgreSQL, Redis, Nginx)
//! 6. 生成 stub 程序源码
//! 7. 编译 stub 程序 -> 最终超级二进制

use crate::deps;
use crate::stub;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

/// 构建配置
pub struct BuildConfig {
    pub workspace_root: PathBuf,
    pub output_path: PathBuf,
    pub target: String,
    pub skip_frontend: bool,
    pub skip_webide: bool,
    pub skip_rust: bool,
    pub release: bool,
    pub keep_temp: bool,
    /// 是否构建内置依赖
    pub bundled_deps: bool,
    /// 构建 PostgreSQL
    pub build_postgresql: bool,
    /// 构建 Redis
    pub build_redis: bool,
    /// 构建 Nginx
    pub build_nginx: bool,
    /// 跳过依赖构建（使用缓存）
    pub skip_deps_build: bool,
}

/// 收集的资源
pub struct CollectedAssets {
    /// 前端目录 (只有 index.html 等)
    pub frontend_dir: PathBuf,
    /// WebIDE 目录 (只有 index.html 等)
    pub webide_dir: PathBuf,
    /// 静态资源目录 (assets/main, assets/webide/*)
    pub static_dir: PathBuf,
    /// 二进制目录
    pub binaries_dir: PathBuf,
    /// Migrations 目录
    pub migrations_dir: PathBuf,
    /// 配置模板目录
    pub templates_dir: PathBuf,
    /// 内置依赖目录（可选）
    pub deps_dir: Option<PathBuf>,
}

/// 执行构建
pub async fn run_build(config: BuildConfig) -> Result<()> {
    info!("Starting GitFox Omnibus build...");
    info!("Workspace: {}", config.workspace_root.display());
    info!("Target: {}", config.target);
    info!("Output: {}", config.output_path.display());

    // 创建构建目录 (在工作区内，避免占用 /tmp)
    let omnibus_dir = config.workspace_root.join("gitfox-omnibus");
    let build_dir = omnibus_dir.join(".build");
    
    // 只清理 embedded 资源目录（资源可能变化），保留 stub/target 以实现增量编译
    let stub_embedded = build_dir.join("stub").join("embedded");
    if stub_embedded.exists() {
        info!("Cleaning embedded resources...");
        fs::remove_dir_all(&stub_embedded)?;
    }
    
    info!("Build directory: {}", build_dir.display());

    // 创建子目录 (仅用于收集资源，不在这里编译)
    let assets_dir = build_dir.join("assets");
    let frontend_dir = assets_dir.join("frontend");
    let webide_dir = assets_dir.join("webide");
    let static_dir = assets_dir.join("static");  // 静态资源 (assets/main, assets/webide/*)
    let binaries_dir = assets_dir.join("binaries");
    let migrations_dir = assets_dir.join("migrations");
    let stub_dir = build_dir.join("stub");

    fs::create_dir_all(&frontend_dir)?;
    fs::create_dir_all(&webide_dir)?;
    fs::create_dir_all(&static_dir.join("main"))?;          // assets/main (前端 js/css)
    fs::create_dir_all(&static_dir.join("webide/main"))?;   // assets/webide/main (WebIDE js/css)
    fs::create_dir_all(&static_dir.join("webide/vscode"))?; // assets/webide/vscode
    fs::create_dir_all(&static_dir.join("webide/extensions"))?; // assets/webide/extensions
    fs::create_dir_all(&binaries_dir)?;
    fs::create_dir_all(&migrations_dir)?;
    fs::create_dir_all(&stub_dir)?;

    // Step 1: 构建前端 (在原位置编译)
    if !config.skip_frontend {
        build_frontend_split(&config.workspace_root, &frontend_dir, &static_dir.join("main"))?;
    } else {
        // 复制已有的 dist，分离 assets 目录
        let src = config.workspace_root.join("frontend/dist");
        if src.exists() {
            split_frontend_dist(&src, &frontend_dir, &static_dir.join("main"))?;
            info!("Copied and split existing frontend dist");
        } else {
            warn!("Frontend dist not found, skipping");
        }
    }

    // Step 2: 构建 WebIDE
    if !config.skip_webide {
        build_webide_split(&config.workspace_root, &webide_dir, &static_dir.join("webide"))?;
    } else {
        let src = config.workspace_root.join("webide/dist");
        if src.exists() {
            split_webide_dist(&src, &webide_dir, &static_dir.join("webide"))?;
            info!("Copied and split existing webide dist");
        } else {
            warn!("WebIDE dist not found, skipping");
        }
    }

    // Step 3: 编译 Rust 二进制
    if !config.skip_rust {
        build_rust_binaries(&config.workspace_root, &binaries_dir, &config.target, config.release)?;
    } else {
        warn!("Skipping Rust binary compilation");
    }

    // Step 4: 复制 migrations
    copy_migrations(&config.workspace_root, &migrations_dir)?;
    
    // Step 4.5: 复制配置模板文件
    let templates_dir = assets_dir.join("templates");
    fs::create_dir_all(&templates_dir)?;
    copy_templates(&config.workspace_root, &templates_dir)?;

    // Step 5: 构建内置依赖（可选）
    let deps_dir = if config.bundled_deps {
        let deps_work_dir = build_dir.join("deps-work");
        let deps_output_dir = assets_dir.join("deps");
        fs::create_dir_all(&deps_output_dir)?;

        if !config.skip_deps_build {
            info!("Building bundled dependencies...");
            let deps_config = deps::DepsConfig {
                build_postgresql: config.build_postgresql,
                build_redis: config.build_redis,
                build_nginx: config.build_nginx,
                work_dir: deps_work_dir,
                output_dir: deps_output_dir.clone(),
                target: config.target.clone(),
                use_cache: true,
                jobs: 0,
            };

            let deps_output = deps::build_deps(&deps_config)?;
            deps::copy_deps_to_output(&deps_output, &deps_output_dir)?;
        } else {
            info!("Skipping dependency build, using cached binaries");
        }

        Some(deps_output_dir)
    } else {
        None
    };

    // Step 6: 生成 stub 程序源码
    let assets = CollectedAssets {
        frontend_dir,
        webide_dir,
        static_dir,
        binaries_dir,
        migrations_dir,
        templates_dir,
        deps_dir,
    };

    // omnibus 目录 (包含 stub/ 模板)
    stub::generate_stub_project(&stub_dir, &assets, &omnibus_dir)?;

    // Step 7: 编译 stub 程序 (在 stub_dir 编译，会产生 target/)
    compile_stub(&stub_dir, &config.output_path, &config.target, config.release)?;

    // 计算输出文件的 hash
    let hash = compute_file_hash(&config.output_path)?;
    let size = fs::metadata(&config.output_path)?.len();

    info!("Build completed successfully!");
    info!("Output: {}", config.output_path.display());
    info!("Size: {} bytes ({:.2} MB)", size, size as f64 / 1024.0 / 1024.0);
    info!("SHA256: {}", hash);

    // 清理或保留构建目录
    if config.keep_temp {
        info!("Build files kept at: {}", build_dir.display());
    } else {
        info!("Cleaning up build directory...");
        fs::remove_dir_all(&build_dir)?;
    }

    Ok(())
}

/// 构建前端并分离 assets 目录
/// - index.html 等放到 frontend_dir
/// - assets/* 放到 static_assets_dir
fn build_frontend_split(workspace: &Path, frontend_dir: &Path, static_assets_dir: &Path) -> Result<()> {
    let frontend_src = workspace.join("frontend");

    info!("Building frontend...");

    // npm ci
    run_command("npm", &["ci", "--prefer-offline"], &frontend_src)
        .context("Failed to install frontend dependencies")?;

    // npm run build
    run_command("npm", &["run", "build"], &frontend_src)
        .context("Failed to build frontend")?;

    // 分离 dist 目录
    let dist = frontend_src.join("dist");
    if !dist.exists() {
        return Err(anyhow::anyhow!("Frontend build did not produce dist/"));
    }

    split_frontend_dist(&dist, frontend_dir, static_assets_dir)?;
    info!("Frontend built and split successfully");

    Ok(())
}

/// 分离前端 dist 目录
/// - dist/assets/* -> static_assets_dir/*
/// - dist/* (其他) -> frontend_dir/*
fn split_frontend_dist(dist: &Path, frontend_dir: &Path, static_assets_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dist)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        
        if name == "assets" && path.is_dir() {
            // assets 目录移到 static_assets_dir
            copy_dir_recursive(&path, static_assets_dir)?;
        } else {
            // 其他文件/目录保留在 frontend_dir
            let dest = frontend_dir.join(&name);
            if path.is_dir() {
                copy_dir_recursive(&path, &dest)?;
            } else {
                fs::copy(&path, &dest)?;
            }
        }
    }
    Ok(())
}

/// 构建 WebIDE 并分离 assets 目录
/// - index.html 等放到 webide_dir
/// - assets/* 放到 static_dir/main/*
/// - vscode/* 放到 static_dir/vscode/*
/// - extensions/* 放到 static_dir/extensions/*
fn build_webide_split(workspace: &Path, webide_dir: &Path, static_dir: &Path) -> Result<()> {
    let webide_src = workspace.join("webide");

    if !webide_src.exists() {
        warn!("WebIDE directory not found, skipping");
        return Ok(());
    }

    info!("Building WebIDE...");

    // npm ci
    run_command("npm", &["ci", "--prefer-offline"], &webide_src)
        .context("Failed to install WebIDE dependencies")?;

    // npm run build
    run_command("npm", &["run", "build"], &webide_src)
        .context("Failed to build WebIDE")?;

    // 分离 dist 目录
    let dist = webide_src.join("dist");
    if dist.exists() {
        split_webide_dist(&dist, webide_dir, static_dir)?;
        info!("WebIDE built and split successfully");
    } else {
        warn!("WebIDE build did not produce dist/");
    }

    Ok(())
}

/// 分离 WebIDE dist 目录
/// - dist/assets/* -> static_dir/main/*
/// - dist/vscode/* -> static_dir/vscode/*
/// - dist/extensions/* -> static_dir/extensions/*
/// - dist/* (其他) -> webide_dir/*
fn split_webide_dist(dist: &Path, webide_dir: &Path, static_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(dist)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        
        if path.is_dir() {
            match name_str.as_ref() {
                "assets" => {
                    // assets 目录移到 static_dir/main/
                    copy_dir_recursive(&path, &static_dir.join("main"))?;
                }
                "vscode" => {
                    // vscode 目录移到 static_dir/vscode/
                    copy_dir_recursive(&path, &static_dir.join("vscode"))?;
                }
                "extensions" => {
                    // extensions 目录移到 static_dir/extensions/
                    copy_dir_recursive(&path, &static_dir.join("extensions"))?;
                }
                _ => {
                    // 其他目录保留在 webide_dir
                    copy_dir_recursive(&path, &webide_dir.join(&name))?;
                }
            }
        } else {
            // 文件保留在 webide_dir
            fs::copy(&path, webide_dir.join(&name))?;
        }
    }
    Ok(())
}

/// 编译 Rust 二进制
fn build_rust_binaries(workspace: &Path, output_dir: &Path, target: &str, release: bool) -> Result<()> {
    info!("Building Rust binaries with target: {}", target);

    // 确保目标已安装
    ensure_rust_target(target)?;

    let profile_dir = if release { "release" } else { "debug" };

    // 编译主后端 (devops)
    info!("Building devops...");
    compile_rust_binary(workspace, "devops", target, release)?;
    copy_binary(workspace, "devops", target, profile_dir, output_dir)?;

    // 编译 workhorse
    info!("Building gitfox-workhorse...");
    let workhorse_dir = workspace.join("gitfox-workhorse");
    compile_rust_binary(&workhorse_dir, "gitfox-workhorse", target, release)?;
    copy_binary(&workhorse_dir, "gitfox-workhorse", target, profile_dir, output_dir)?;

    // 编译 shell
    info!("Building gitfox-shell...");
    let shell_dir = workspace.join("gitfox-shell");
    compile_rust_binary(&shell_dir, "gitfox-shell", target, release)?;
    copy_binary(&shell_dir, "gitfox-shell", target, profile_dir, output_dir)?;

    // 编译 authorized-keys-check
    info!("Building gitfox-shell-authorized-keys-check...");
    compile_rust_binary_with_name(&shell_dir, "gitfox-shell-authorized-keys-check", target, release)?;
    copy_binary(&shell_dir, "gitfox-shell-authorized-keys-check", target, profile_dir, output_dir)?;

    // 编译 gitlayer (Git 操作 RPC 服务)
    info!("Building gitlayer...");
    let gitlayer_dir = workspace.join("gitlayer");
    compile_rust_binary(&gitlayer_dir, "gitlayer", target, release)?;
    copy_binary(&gitlayer_dir, "gitlayer", target, profile_dir, output_dir)?;

    info!("All Rust binaries built successfully");
    Ok(())
}

/// 确保 Rust 目标可用
fn ensure_rust_target(target: &str) -> Result<()> {
    if target.contains("musl") {
        let output = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()?;

        let installed = String::from_utf8_lossy(&output.stdout);
        if !installed.contains(target) {
            info!("Installing Rust target: {}", target);
            run_command("rustup", &["target", "add", target], &std::env::current_dir()?)?;
        }
    }
    Ok(())
}

/// 编译单个 Rust 二进制
fn compile_rust_binary(manifest_dir: &Path, bin_name: &str, target: &str, release: bool) -> Result<()> {
    let mut args = vec!["build", "--bin", bin_name, "--target", target];
    if release {
        args.push("--release");
    }

    let status = Command::new("cargo")
        .args(&args)
        .current_dir(manifest_dir)
        .env("RUSTFLAGS", "-C target-feature=+crt-static")
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to compile {}", bin_name));
    }

    Ok(())
}

/// 编译指定名称的二进制
fn compile_rust_binary_with_name(manifest_dir: &Path, bin_name: &str, target: &str, release: bool) -> Result<()> {
    compile_rust_binary(manifest_dir, bin_name, target, release)
}

/// 复制编译好的二进制
fn copy_binary(manifest_dir: &Path, bin_name: &str, target: &str, profile: &str, output_dir: &Path) -> Result<()> {
    let src = manifest_dir.join("target").join(target).join(profile).join(bin_name);
    let dst = output_dir.join(bin_name);

    if src.exists() {
        fs::copy(&src, &dst)?;
    } else {
        return Err(anyhow::anyhow!("Binary not found: {}", src.display()));
    }

    Ok(())
}

/// 复制配置模板文件
fn copy_templates(workspace: &Path, output_dir: &Path) -> Result<()> {
    info!("Copying configuration templates...");
    
    // 复制 .env.example (旧格式，向后兼容)
    let env_example = workspace.join(".env.example");
    if env_example.exists() {
        fs::copy(&env_example, output_dir.join("gitfox.env.template"))?;
        info!("Copied .env.example → gitfox.env.template");
    } else {
        warn!(".env.example not found");
    }
    
    // 复制 gitfox.toml.example (新统一配置格式) - 从 gitfox-omnibus 目录
    let omnibus_dir = workspace.join("gitfox-omnibus");
    let toml_example = omnibus_dir.join("gitfox.toml.example");
    if toml_example.exists() {
        fs::copy(&toml_example, output_dir.join("gitfox.toml.template"))?;
        info!("Copied gitfox-omnibus/gitfox.toml.example → gitfox.toml.template");
    } else {
        return Err(anyhow::anyhow!("gitfox-omnibus/gitfox.toml.example not found - this file is required"));
    }
    
    // 复制 config.example.toml
    let workhorse_config = workspace.join("gitfox-workhorse").join("config.example.toml");
    if workhorse_config.exists() {
        fs::copy(&workhorse_config, output_dir.join("workhorse.toml.template"))?;
        info!("Copied config.example.toml → workhorse.toml.template");
    } else {
        warn!("gitfox-workhorse/config.example.toml not found");
    }
    
    Ok(())
}

/// 复制 migrations
fn copy_migrations(workspace: &Path, output_dir: &Path) -> Result<()> {
    let migrations_src = workspace.join("migrations");

    if !migrations_src.exists() {
        warn!("Migrations directory not found");
        return Ok(());
    }

    info!("Copying migrations...");

    let mut count = 0;
    for entry in fs::read_dir(&migrations_src)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "sql") {
            let dest = output_dir.join(entry.file_name());
            fs::copy(&path, &dest)?;
            count += 1;
        }
    }

    info!("Copied {} migration files", count);
    Ok(())
}

/// 编译 stub 程序
fn compile_stub(stub_dir: &Path, output_path: &Path, target: &str, release: bool) -> Result<()> {
    info!("Compiling stub program...");

    let mut args = vec!["build", "--target", target];
    if release {
        args.push("--release");
    }

    let status = Command::new("cargo")
        .args(&args)
        .current_dir(stub_dir)
        .env("RUSTFLAGS", "-C target-feature=+crt-static")
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to compile stub program"));
    }

    // 复制最终二进制
    let profile = if release { "release" } else { "debug" };
    let stub_binary = stub_dir.join("target").join(target).join(profile).join("gitfox");

    if !stub_binary.exists() {
        return Err(anyhow::anyhow!("Stub binary not found: {}", stub_binary.display()));
    }

    // 确保输出目录存在
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(&stub_binary, output_path)?;

    // 设置可执行权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(output_path, perms)?;
    }

    info!("Stub program compiled successfully");
    Ok(())
}

/// 运行命令
fn run_command(cmd: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("Failed to execute: {} {:?}", cmd, args))?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "Command failed: {} {:?} (exit code: {:?})",
            cmd,
            args,
            status.code()
        ));
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

/// 计算文件 hash
fn compute_file_hash(path: &Path) -> Result<String> {
    let data = fs::read(path)?;
    let hash = Sha256::digest(&data);
    Ok(hex::encode(hash))
}
