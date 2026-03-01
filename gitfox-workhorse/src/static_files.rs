use actix_files::NamedFile;
use actix_web::{
    error::ErrorNotFound,
    web, HttpRequest, Result,
};
use std::path::PathBuf;

/// SPA index.html 处理器 - 为前端路由返回 index.html
pub async fn serve_spa_index(req: HttpRequest, dist_path: web::Data<PathBuf>) -> Result<NamedFile> {
    let path = dist_path.join("index.html");
    if !path.exists() {
        tracing::error!("index.html not found at {:?}", path);
        return Err(ErrorNotFound("index.html not found"));
    }
    
    tracing::debug!("Serving SPA index.html for: {}", req.path());
    Ok(NamedFile::open(path)?)
}

/// WebIDE index.html 处理器 - 为 WebIDE 路由返回 index.html
pub async fn serve_webide_index(
    req: HttpRequest,
    webide_dist_path: web::Data<PathBuf>,
) -> Result<NamedFile> {
    let path = webide_dist_path.join("index.html");
    if !path.exists() {
        tracing::error!("WebIDE index.html not found at {:?}", path);
        return Err(ErrorNotFound("WebIDE index.html not found"));
    }

    tracing::debug!("Serving WebIDE index.html for: {}", req.path());
    Ok(NamedFile::open(path)?)
}

/// 主前端静态资源 - /assets/* -> assets_path/main/*
/// (排除 /assets/upload/*)
pub async fn serve_main_assets(
    req: HttpRequest,
    assets_path: web::Data<PathBuf>,
) -> Result<NamedFile> {
    let path = req.path().strip_prefix("/assets/").unwrap_or("");
    let file_path = assets_path.join("main").join(path);

    if !file_path.exists() {
        tracing::warn!("Main asset not found: {:?}", file_path);
        return Err(ErrorNotFound("File not found"));
    }

    // 安全检查
    let canonical_base = assets_path
        .join("main")
        .canonicalize()
        .map_err(|_| ErrorNotFound("main directory not found"))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| ErrorNotFound("File not found"))?;

    if !canonical_file.starts_with(&canonical_base) {
        tracing::warn!("Attempted directory traversal: {:?}", file_path);
        return Err(ErrorNotFound("Invalid path"));
    }

    tracing::debug!("Serving main asset: {:?}", file_path);
    Ok(NamedFile::open(file_path)?)
}

/// WebIDE main 静态文件 - 路径重写
/// /-/ide/assets/* (web) -> assets_path/webide/main/* (fs)
pub async fn serve_webide_main(
    req: HttpRequest,
    assets_path: web::Data<PathBuf>,
) -> Result<NamedFile> {
    let path = req.path().strip_prefix("/-/ide/assets/").unwrap_or("");
    let file_path = assets_path.join("webide").join("main").join(path);

    if !file_path.exists() {
        tracing::warn!("WebIDE main file not found: {:?}", file_path);
        return Err(ErrorNotFound("File not found"));
    }

    // 安全检查
    let canonical_base = assets_path
        .join("webide")
        .join("main")
        .canonicalize()
        .map_err(|_| ErrorNotFound("webide/main directory not found"))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| ErrorNotFound("File not found"))?;

    if !canonical_file.starts_with(&canonical_base) {
        tracing::warn!("Attempted directory traversal: {:?}", file_path);
        return Err(ErrorNotFound("Invalid path"));
    }

    tracing::debug!("Serving webide main file: {:?}", file_path);
    Ok(NamedFile::open(file_path)?)
}

/// WebIDE vscode 静态文件 - 路径重写
/// /-/ide/vscode/* (web) -> assets_path/webide/vscode/* (fs)
pub async fn serve_webide_vscode(
    req: HttpRequest,
    assets_path: web::Data<PathBuf>,
) -> Result<NamedFile> {
    let path = req.path().strip_prefix("/-/ide/vscode/").unwrap_or("");
    let file_path = assets_path.join("webide").join("vscode").join(path);

    if !file_path.exists() {
        tracing::warn!("WebIDE vscode file not found: {:?}", file_path);
        return Err(ErrorNotFound("File not found"));
    }

    // 安全检查
    let canonical_base = assets_path
        .join("webide")
        .join("vscode")
        .canonicalize()
        .map_err(|_| ErrorNotFound("webide/vscode directory not found"))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| ErrorNotFound("File not found"))?;

    if !canonical_file.starts_with(&canonical_base) {
        tracing::warn!("Attempted directory traversal: {:?}", file_path);
        return Err(ErrorNotFound("Invalid path"));
    }

    tracing::debug!("Serving webide vscode file: {:?}", file_path);
    Ok(NamedFile::open(file_path)?)
}

/// WebIDE extensions 静态文件 - 路径重写
/// /-/ide/extensions/* (web) -> assets_path/webide/extensions/* (fs)
pub async fn serve_webide_extensions(
    req: HttpRequest,
    assets_path: web::Data<PathBuf>,
) -> Result<NamedFile> {
    let path = req.path().strip_prefix("/-/ide/extensions/").unwrap_or("");
    let file_path = assets_path.join("webide").join("extensions").join(path);

    if !file_path.exists() {
        tracing::warn!("WebIDE extension file not found: {:?}", file_path);
        return Err(ErrorNotFound("File not found"));
    }

    // 安全检查
    let canonical_base = assets_path
        .join("webide")
        .join("extensions")
        .canonicalize()
        .map_err(|_| ErrorNotFound("webide/extensions directory not found"))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| ErrorNotFound("File not found"))?;

    if !canonical_file.starts_with(&canonical_base) {
        tracing::warn!("Attempted directory traversal: {:?}", file_path);
        return Err(ErrorNotFound("Invalid path"));
    }

    tracing::debug!("Serving webide extension file: {:?}", file_path);
    Ok(NamedFile::open(file_path)?)
}

/// 用户上传文件 - /assets/upload/* -> assets_path/upload/*
pub async fn serve_upload(
    req: HttpRequest,
    assets_path: web::Data<PathBuf>,
) -> Result<NamedFile> {
    let path = req.path().strip_prefix("/assets/upload/").unwrap_or("");
    let file_path = assets_path.join("upload").join(path);

    if !file_path.exists() {
        tracing::warn!("Upload asset not found: {:?}", file_path);
        return Err(ErrorNotFound("Asset not found"));
    }

    // 安全检查
    let canonical_assets = assets_path
        .join("upload")
        .canonicalize()
        .map_err(|_| ErrorNotFound("upload directory not found"))?;
    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| ErrorNotFound("Asset not found"))?;

    if !canonical_file.starts_with(&canonical_assets) {
        tracing::warn!("Attempted directory traversal: {:?}", file_path);
        return Err(ErrorNotFound("Invalid path"));
    }

    tracing::debug!("Serving upload asset: {:?}", file_path);
    Ok(NamedFile::open(file_path)?)
}
