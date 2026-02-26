//! Stub 程序生成器
//!
//! 从 stub/ 目录复制模板文件并替换路径占位符，生成可编译的 Rust 项目。
//! 使用 rust-embed 将所有资源嵌入到最终二进制。

use crate::build::CollectedAssets;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tracing::info;

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
    copy_dir_all(&assets.binaries_dir, &embedded_dir.join("bin"))?;
    copy_dir_all(&assets.migrations_dir, &embedded_dir.join("migrations"))?;

    // 生成 Cargo.toml (从模板复制)
    generate_cargo_toml(stub_dir, &template_dir)?;

    // 复制 main.rs (使用相对路径)
    copy_main_rs(&src_dir, &template_dir)?;

    info!("Stub project generated at: {}", stub_dir.display());
    Ok(())
}

/// 递归复制目录
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// 复制 Cargo.toml 模板
fn generate_cargo_toml(stub_dir: &Path, template_dir: &Path) -> Result<()> {
    let template_path = template_dir.join("Cargo.toml.template");
    let content = fs::read_to_string(&template_path)
        .with_context(|| format!("Failed to read {}", template_path.display()))?;

    fs::write(stub_dir.join("Cargo.toml"), content)?;
    Ok(())
}

/// 复制 main.rs (使用相对路径)
fn copy_main_rs(src_dir: &Path, template_dir: &Path) -> Result<()> {
    let template_path = template_dir.join("src/main.rs");
    let content = fs::read_to_string(&template_path)
        .with_context(|| format!("Failed to read {}", template_path.display()))?;

    fs::write(src_dir.join("main.rs"), content)?;
    Ok(())
}

