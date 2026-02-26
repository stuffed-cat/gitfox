use actix_files::NamedFile;
use actix_web::{
    error::ErrorNotFound,
    web, HttpRequest, Result,
};
use std::path::PathBuf;

/// SPA fallback handler - 对于未匹配的路由返回 index.html
pub async fn spa_fallback(req: HttpRequest, dist_path: web::Data<PathBuf>) -> Result<NamedFile> {
    let path = dist_path.join("index.html");
    if !path.exists() {
        tracing::error!("index.html not found at {:?}", path);
        return Err(ErrorNotFound("index.html not found"));
    }
    
    tracing::debug!("SPA fallback: serving index.html for {}", req.path());
    Ok(NamedFile::open(path)?)
}

/// WebIDE fallback handler - 对于未匹配的 WebIDE 路由返回 index.html
pub async fn webide_fallback(
    req: HttpRequest,
    webide_dist_path: web::Data<PathBuf>,
) -> Result<NamedFile> {
    let path = webide_dist_path.join("index.html");
    if !path.exists() {
        tracing::error!("WebIDE index.html not found at {:?}", path);
        return Err(ErrorNotFound("WebIDE index.html not found"));
    }

    tracing::debug!("WebIDE fallback: serving index.html for {}", req.path());
    Ok(NamedFile::open(path)?)
}

/// Assets 静态文件处理器
pub async fn serve_asset(
    req: HttpRequest,
    assets_path: web::Data<PathBuf>,
) -> Result<NamedFile> {
    // 从 /assets/xxx 提取出 xxx 部分
    let path = req.path().strip_prefix("/assets/").unwrap_or("");
    let file_path = assets_path.join(path);

    if !file_path.exists() {
        tracing::warn!("Asset not found: {:?}", file_path);
        return Err(ErrorNotFound("Asset not found"));
    }

    // 安全检查：确保路径在 assets 目录内
    let canonical_assets = assets_path
        .canonicalize()
        .map_err(|_| ErrorNotFound("Assets directory not found"))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| ErrorNotFound("Asset not found"))?;

    if !canonical_file.starts_with(&canonical_assets) {
        tracing::warn!("Attempted directory traversal: {:?}", file_path);
        return Err(ErrorNotFound("Invalid path"));
    }

    tracing::debug!("Serving asset: {:?}", file_path);
    Ok(NamedFile::open(file_path)?)
}
