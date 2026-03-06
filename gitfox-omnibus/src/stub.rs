//! Stub 程序生成器
//!
//! 从 stub/ 目录复制模板文件并替换路径占位符，生成可编译的 Rust 项目。
//! 使用 rust-embed 将所有资源嵌入到最终二进制。

use crate::build::CollectedAssets;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use tracing::{info, debug};

/// stub 模板目录相对于 omnibus crate 根目录的路径
const STUB_TEMPLATE_DIR: &str = "stub";

/// 生成 stub 项目
///
/// 从模板目录复制文件，并将资源复制到 embedded/ 子目录
pub fn generate_stub_project(
    stub_dir: &Path,
    assets: &CollectedAssets,
    omnibus_dir: &Path,
) -> Result<()> {
    info!("Generating stub project...");

    let template_dir = omnibus_dir.join(STUB_TEMPLATE_DIR);

    // 创建目录结构
    let src_dir = stub_dir.join("src");
    let embedded_dir = stub_dir.join("embedded");
    fs::create_dir_all(&src_dir)?;
    fs::create_dir_all(&embedded_dir)?;

    // 复制资源到 embedded/ 子目录（相对路径）
    copy_dir_all(&assets.frontend_dir, &embedded_dir.join("frontend"))?;
    copy_dir_all(&assets.webide_dir, &embedded_dir.join("webide"))?;
    copy_dir_all(&assets.static_dir, &embedded_dir.join("static"))?;  // 静态资源 (assets/)
    copy_dir_all(&assets.binaries_dir, &embedded_dir.join("bin"))?;
    copy_dir_all(&assets.migrations_dir, &embedded_dir.join("migrations"))?;
    copy_dir_all(&assets.templates_dir, &embedded_dir.join("templates"))?;
    
    // 复制内置依赖（如果有）
    if let Some(ref deps_dir) = assets.deps_dir {
        if deps_dir.exists() {
            info!("Copying bundled dependencies...");
            copy_dir_all(deps_dir, &embedded_dir.join("deps"))?;
        }
    }

    // 生成 Cargo.toml (从模板复制)
    generate_cargo_toml(stub_dir, &template_dir, assets.deps_dir.is_some())?;

    // 生成 build.rs (从模板复制)
    generate_build_rs(stub_dir, &template_dir)?;

    // 复制 main.rs (使用相对路径)
    copy_main_rs(&src_dir, &template_dir)?;

    info!("Stub project generated at: {}", stub_dir.display());
    Ok(())
}

/// 递归智能复制目录（支持符号链接）
/// 只在源文件比目标文件更新时才复制，实现增量更新
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else if ty.is_symlink() {
            // 复制符号链接
            let target = fs::read_link(entry.path())?;
            // 如果目标已存在，先删除
            if dst_path.exists() || dst_path.is_symlink() {
                fs::remove_file(&dst_path).ok();
            }
            std::os::unix::fs::symlink(&target, &dst_path)?;
        } else {
            // 智能复制：只在源文件更新时才复制
            if should_copy(&entry.path(), &dst_path) {
                fs::copy(entry.path(), &dst_path)?;
                debug!("Copied: {}", entry.path().display());
            }
        }
    }
    Ok(())
}

/// 判断是否需要复制文件
/// 如果目标不存在，或源文件修改时间更新，则需要复制
fn should_copy(src: &Path, dst: &Path) -> bool {
    if !dst.exists() {
        return true;
    }
    
    // 比较修改时间
    let src_mtime = get_mtime(src);
    let dst_mtime = get_mtime(dst);
    
    match (src_mtime, dst_mtime) {
        (Some(src_t), Some(dst_t)) => src_t > dst_t,
        _ => true, // 无法获取时间时，保守复制
    }
}

/// 获取文件修改时间
fn get_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
}

/// 复制 Cargo.toml 模板（只在内容变化时更新）
fn generate_cargo_toml(stub_dir: &Path, template_dir: &Path, has_deps: bool) -> Result<()> {
    let template_path = template_dir.join("Cargo.toml.template");
    let mut content = fs::read_to_string(&template_path)
        .with_context(|| format!("Failed to read {}", template_path.display()))?;

    // 如果有内置依赖，启用 bundled-deps feature 作为默认
    if has_deps {
        // 模板已有 bundled-deps = []，只需添加 default
        content = content.replace(
            "bundled-deps = []",
            "default = [\"bundled-deps\"]\nbundled-deps = []"
        );
    }

    let dest = stub_dir.join("Cargo.toml");
    
    // 只在文件不存在或内容不同时才写入
    let should_write = if dest.exists() {
        let existing = fs::read_to_string(&dest).unwrap_or_default();
        existing != content
    } else {
        true
    };
    
    if should_write {
        fs::write(&dest, content)?;
        info!("Updated: Cargo.toml");
    }
    
    Ok(())
}

/// 复制 build.rs 模板（只在内容变化时更新）
fn generate_build_rs(stub_dir: &Path, template_dir: &Path) -> Result<()> {
    let template_path = template_dir.join("build.rs.template");
    let content = fs::read_to_string(&template_path)
        .with_context(|| format!("Failed to read {}", template_path.display()))?;

    let dest = stub_dir.join("build.rs");
    
    // 只在文件不存在或内容不同时才写入
    let should_write = if dest.exists() {
        let existing = fs::read_to_string(&dest).unwrap_or_default();
        existing != content
    } else {
        true
    };
    
    if should_write {
        fs::write(&dest, content)?;
        info!("Updated: build.rs");
    }
    
    Ok(())
}

/// 复制所有 Rust 源文件（只在内容变化时复制，避免触发不必要的重新编译）
fn copy_main_rs(src_dir: &Path, template_dir: &Path) -> Result<()> {
    let template_src_dir = template_dir.join("src");
    
    // 复制 src/ 目录下所有 .rs 文件
    for entry in fs::read_dir(&template_src_dir)
        .with_context(|| format!("Failed to read directory {}", template_src_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map(|e| e == "rs").unwrap_or(false) {
            let filename = entry.file_name();
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            
            let dest = src_dir.join(&filename);
            
            // 只在文件不存在或内容不同时才写入
            let should_write = if dest.exists() {
                let existing = fs::read_to_string(&dest).unwrap_or_default();
                existing != content
            } else {
                true
            };
            
            if should_write {
                fs::write(&dest, content)?;
                info!("Updated: src/{}", filename.to_string_lossy());
            }
        }
    }
    
    Ok(())
}

