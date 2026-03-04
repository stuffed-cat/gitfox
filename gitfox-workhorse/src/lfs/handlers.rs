//! LFS HTTP 处理器
//!
//! 实现 Git LFS Batch API v1 的 HTTP 端点。

use actix_web::{web, Error, HttpRequest, HttpResponse};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::auth_client::{AuthClient, HttpAccessResult};
use crate::config::Config;
use crate::lfs::client::{LfsClient, LfsClientError};
use crate::lfs::storage::{LfsStorage, LfsStorageError};
use crate::lfs::types::*;

/// LFS 服务状态
pub struct LfsState {
    pub storage: LfsStorage,
    pub config: Arc<Config>,
    pub auth_client: Option<AuthClient>,
    pub lfs_client: Option<LfsClient>,
}

impl LfsState {
    pub fn new(config: Arc<Config>) -> Self {
        let storage = LfsStorage::new(&config.lfs_storage_path);
        
        // 初始化 LFS gRPC 客户端（如果配置了）
        let lfs_client = config.auth_grpc_address.as_ref().map(|addr| {
            LfsClient::new(addr, config.shell_secret.clone())
        });

        Self {
            storage,
            config,
            auth_client: None,
            lfs_client,
        }
    }

    /// 初始化存储
    pub async fn init(&self) -> std::io::Result<()> {
        self.storage.init().await
    }
}

/// 解析 LFS 请求路径
/// 格式: /{namespace}/{project}.git/info/lfs/...
pub fn parse_lfs_path(path: &str) -> Option<(String, String, String)> {
    // 移除开头的斜杠
    let path = path.trim_start_matches('/');
    
    // 查找 .git/info/lfs
    let git_marker = ".git/info/lfs";
    let marker_pos = path.find(git_marker)?;
    
    // 提取仓库路径
    let repo_part = &path[..marker_pos];
    let lfs_path = &path[marker_pos + git_marker.len()..];
    let lfs_path = lfs_path.trim_start_matches('/');
    
    // 分离 namespace 和 project
    let parts: Vec<&str> = repo_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return None;
    }
    
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        lfs_path.to_string(),
    ))
}

/// 从请求中提取认证信息
fn extract_auth_info(req: &HttpRequest) -> Option<(String, String)> {
    use base64::Engine;
    
    // 检查 Authorization header
    let auth_header = req.headers().get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;

    if auth_str.starts_with("Basic ") {
        // Basic 认证
        let encoded = &auth_str[6..];
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).ok()?;
        let credentials = String::from_utf8(decoded).ok()?;
        let parts: Vec<&str> = credentials.splitn(2, ':').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    } else if auth_str.starts_with("Bearer ") {
        // Bearer token (用于 LFS)
        let token = &auth_str[7..];
        return Some(("".to_string(), token.to_string()));
    }

    None
}

/// 生成 LFS 错误响应
fn lfs_error_response(status: actix_web::http::StatusCode, message: &str) -> HttpResponse {
    HttpResponse::build(status)
        .content_type("application/vnd.git-lfs+json")
        .json(serde_json::json!({
            "message": message,
            "documentation_url": "https://git-lfs.github.com/spec/v1"
        }))
}

/// LFS Batch API 处理器
/// POST /{namespace}/{project}.git/info/lfs/objects/batch
pub async fn handle_batch(
    req: HttpRequest,
    body: web::Json<LfsBatchRequest>,
    state: web::Data<LfsState>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    debug!("LFS batch request: {}", path);

    // 解析路径
    let (namespace, project, _) = match parse_lfs_path(path) {
        Some(p) => p,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                "Invalid LFS path",
            ));
        }
    };

    let repo_path = format!("{}/{}", namespace, project);
    let operation = &body.operation;

    // 验证操作类型
    if operation != "download" && operation != "upload" {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::BAD_REQUEST,
            &format!("Invalid operation: {}", operation),
        ));
    }

    // 认证
    let auth_info = match authenticate_lfs_request(&req, &repo_path, operation, &state).await {
        Ok(info) => info,
        Err(e) => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::UNAUTHORIZED,
                &e,
            ));
        }
    };

    // 检查写权限（upload 需要）
    if operation == "upload" && !auth_info.can_write {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::FORBIDDEN,
            "Write access required for upload",
        ));
    }

    // 生成响应
    let base_url = get_base_url(&req, &state.config);
    let ref_name = body.ref_info.as_ref().map(|r| r.name.clone());

    let mut response = LfsBatchResponse::new();

    for obj in &body.objects {
        // 检查对象大小限制
        if obj.size > state.config.lfs_max_object_size as i64 {
            response.objects.push(LfsBatchObject {
                oid: obj.oid.clone(),
                size: obj.size,
                authenticated: None,
                actions: None,
                error: Some(LfsError::object_too_large(obj.size, state.config.lfs_max_object_size)),
            });
            continue;
        }

        // 检查对象是否存在
        let exists = state.storage.exists(&obj.oid).await;

        match operation.as_str() {
            "download" => {
                if exists {
                    // 对象存在，返回下载链接
                    response.objects.push(create_download_object(
                        &obj.oid,
                        obj.size,
                        &base_url,
                        &repo_path,
                        &state.config,
                    ));
                } else {
                    // 对象不存在
                    response.objects.push(LfsBatchObject {
                        oid: obj.oid.clone(),
                        size: obj.size,
                        authenticated: None,
                        actions: None,
                        error: Some(LfsError::not_found("Object not found")),
                    });
                }
            }
            "upload" => {
                if exists {
                    // 对象已存在，不需要上传
                    response.objects.push(LfsBatchObject {
                        oid: obj.oid.clone(),
                        size: obj.size,
                        authenticated: Some(true),
                        actions: None,
                        error: None,
                    });
                } else {
                    // 需要上传
                    response.objects.push(create_upload_object(
                        &obj.oid,
                        obj.size,
                        &base_url,
                        &repo_path,
                        &state.config,
                    ));
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(HttpResponse::Ok()
        .content_type("application/vnd.git-lfs+json")
        .json(response))
}

/// 创建下载对象响应
fn create_download_object(
    oid: &str,
    size: i64,
    base_url: &str,
    repo_path: &str,
    config: &Config,
) -> LfsBatchObject {
    let download_url = format!(
        "{}/{}.git/info/lfs/objects/{}",
        base_url, repo_path, oid
    );

    let mut header = HashMap::new();
    header.insert("Accept".to_string(), "application/octet-stream".to_string());

    LfsBatchObject {
        oid: oid.to_string(),
        size,
        authenticated: Some(true),
        actions: Some(LfsBatchActions {
            download: Some(LfsAction {
                href: download_url,
                header: Some(header),
                expires_in: Some(config.lfs_link_expires as i64),
                expires_at: None,
            }),
            upload: None,
            verify: None,
        }),
        error: None,
    }
}

/// 创建上传对象响应
fn create_upload_object(
    oid: &str,
    size: i64,
    base_url: &str,
    repo_path: &str,
    config: &Config,
) -> LfsBatchObject {
    let upload_url = format!(
        "{}/{}.git/info/lfs/objects/{}",
        base_url, repo_path, oid
    );
    let verify_url = format!(
        "{}/{}.git/info/lfs/objects/verify",
        base_url, repo_path
    );

    let mut upload_header = HashMap::new();
    upload_header.insert("Content-Type".to_string(), "application/octet-stream".to_string());

    let mut verify_header = HashMap::new();
    verify_header.insert("Content-Type".to_string(), "application/vnd.git-lfs+json".to_string());

    LfsBatchObject {
        oid: oid.to_string(),
        size,
        authenticated: Some(true),
        actions: Some(LfsBatchActions {
            download: None,
            upload: Some(LfsAction {
                href: upload_url,
                header: Some(upload_header),
                expires_in: Some(config.lfs_link_expires as i64),
                expires_at: None,
            }),
            verify: Some(LfsAction {
                href: verify_url,
                header: Some(verify_header),
                expires_in: Some(config.lfs_link_expires as i64),
                expires_at: None,
            }),
        }),
        error: None,
    }
}

/// LFS 对象下载处理器
/// GET /{namespace}/{project}.git/info/lfs/objects/{oid}
pub async fn handle_download(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    state: web::Data<LfsState>,
) -> Result<HttpResponse, Error> {
    let (namespace, project, oid) = path.into_inner();
    let repo_path = format!("{}/{}", namespace, project);

    debug!("LFS download: {} from {}", oid, repo_path);

    // 验证 OID 格式 (SHA-256 = 64 hex chars)
    if oid.len() != 64 || !oid.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::BAD_REQUEST,
            "Invalid OID format",
        ));
    }

    // 认证（下载需要读权限）
    if let Err(e) = authenticate_lfs_request(&req, &repo_path, "download", &state).await {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::UNAUTHORIZED,
            &e,
        ));
    }

    // 检查对象是否存在
    let object_info = match state.storage.get_object_info(&oid).await {
        Some(info) => info,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::NOT_FOUND,
                "Object not found",
            ));
        }
    };

    // 打开文件并流式传输
    let file = match state.storage.open_object(&oid).await {
        Ok(f) => f,
        Err(e) => {
            error!("Failed to open LFS object {}: {}", oid, e);
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read object",
            ));
        }
    };

    // 使用 tokio_util 将 AsyncRead 转换为 Stream
    let stream = tokio_util::io::ReaderStream::new(file);
    let body = actix_web::body::BodyStream::new(stream);

    Ok(HttpResponse::Ok()
        .content_type("application/octet-stream")
        .insert_header(("Content-Length", object_info.size.to_string()))
        .insert_header(("X-Content-Type-Options", "nosniff"))
        .body(body))
}

/// LFS 对象上传处理器
/// PUT /{namespace}/{project}.git/info/lfs/objects/{oid}
pub async fn handle_upload(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    mut payload: web::Payload,
    state: web::Data<LfsState>,
) -> Result<HttpResponse, Error> {
    let (namespace, project, oid) = path.into_inner();
    let repo_path = format!("{}/{}", namespace, project);

    debug!("LFS upload: {} to {}", oid, repo_path);

    // 验证 OID 格式
    if oid.len() != 64 || !oid.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::BAD_REQUEST,
            "Invalid OID format",
        ));
    }

    // 认证（上传需要写权限）
    let auth_info = match authenticate_lfs_request(&req, &repo_path, "upload", &state).await {
        Ok(info) => info,
        Err(e) => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::UNAUTHORIZED,
                &e,
            ));
        }
    };

    if !auth_info.can_write {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::FORBIDDEN,
            "Write access required",
        ));
    }

    // 获取 Content-Length
    let content_length: i64 = req
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // 检查大小限制
    if content_length > state.config.lfs_max_object_size as i64 {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
            &format!(
                "Object too large: {} > {}",
                content_length, state.config.lfs_max_object_size
            ),
        ));
    }

    // 如果对象已存在，直接返回成功
    if state.storage.exists(&oid).await {
        return Ok(HttpResponse::Ok().finish());
    }

    // 创建临时文件
    let (temp_path, mut temp_file) = match state.storage.create_temp_file().await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to create temp file: {}", e);
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create upload file",
            ));
        }
    };

    // 流式写入数据
    use tokio::io::AsyncWriteExt;
    let mut total_written = 0i64;

    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(|e| {
            error!("Failed to read chunk: {}", e);
            actix_web::error::ErrorBadRequest(e)
        })?;

        total_written += chunk.len() as i64;

        // 检查是否超过声明的大小
        if content_length > 0 && total_written > content_length {
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                "Content larger than declared size",
            ));
        }

        if let Err(e) = temp_file.write_all(&chunk).await {
            let _ = tokio::fs::remove_file(&temp_path).await;
            error!("Failed to write chunk: {}", e);
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to write data",
            ));
        }
    }

    // 确保数据写入磁盘
    if let Err(e) = temp_file.flush().await {
        let _ = tokio::fs::remove_file(&temp_path).await;
        error!("Failed to flush file: {}", e);
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to finalize upload",
        ));
    }

    drop(temp_file);

    // 验证并移动到最终位置
    match state.storage.finalize_upload(&temp_path, &oid, total_written).await {
        Ok(()) => {
            info!("LFS object {} uploaded successfully ({} bytes)", oid, total_written);
            Ok(HttpResponse::Ok().finish())
        }
        Err(LfsStorageError::SizeMismatch { expected, actual }) => {
            Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                &format!("Size mismatch: expected {}, got {}", expected, actual),
            ))
        }
        Err(LfsStorageError::OidMismatch { expected, actual }) => {
            Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                &format!("OID mismatch: expected {}, got {}", expected, actual),
            ))
        }
        Err(e) => {
            error!("Failed to finalize upload: {}", e);
            Ok(lfs_error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to finalize upload",
            ))
        }
    }
}

/// LFS 验证处理器
/// POST /{namespace}/{project}.git/info/lfs/objects/verify
pub async fn handle_verify(
    req: HttpRequest,
    body: web::Json<LfsVerifyRequest>,
    state: web::Data<LfsState>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    debug!("LFS verify: {}", path);

    // 解析路径
    let (namespace, project, _) = match parse_lfs_path(path) {
        Some(p) => p,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                "Invalid LFS path",
            ));
        }
    };

    let repo_path = format!("{}/{}", namespace, project);

    // 认证
    let auth_info = match authenticate_lfs_request(&req, &repo_path, "upload", &state).await {
        Ok(info) => info,
        Err(e) => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::UNAUTHORIZED,
                &e,
            ));
        }
    };

    if !auth_info.can_write {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::FORBIDDEN,
            "Write access required",
        ));
    }

    // 验证对象存在且大小正确
    match state.storage.get_object_info(&body.oid).await {
        Some(info) if info.size == body.size => {
            Ok(HttpResponse::Ok().finish())
        }
        Some(info) => {
            Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                &format!("Size mismatch: expected {}, got {}", body.size, info.size),
            ))
        }
        None => {
            Ok(lfs_error_response(
                actix_web::http::StatusCode::NOT_FOUND,
                "Object not found",
            ))
        }
    }
}

// ============ Lock API 处理器 ============

/// 创建锁
/// POST /{namespace}/{project}.git/info/lfs/locks
pub async fn handle_create_lock(
    req: HttpRequest,
    body: web::Json<LfsLockRequest>,
    state: web::Data<LfsState>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    debug!("LFS create lock: {}", path);

    let (namespace, project, _) = match parse_lfs_path(path) {
        Some(p) => p,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                "Invalid LFS path",
            ));
        }
    };

    let repo_path = format!("{}/{}", namespace, project);

    // 认证（锁定需要写权限）
    let auth_info = match authenticate_lfs_request(&req, &repo_path, "upload", &state).await {
        Ok(info) => info,
        Err(e) => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::UNAUTHORIZED,
                &e,
            ));
        }
    };

    if !auth_info.can_write {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::FORBIDDEN,
            "Write access required for locking",
        ));
    }

    // 调用 LFS gRPC 服务创建锁
    let lfs_client = match &state.lfs_client {
        Some(c) => c,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::NOT_IMPLEMENTED,
                "LFS locks not configured",
            ));
        }
    };

    let ref_name = body.ref_info.as_ref().map(|r| r.name.clone());

    match lfs_client
        .create_lock(
            auth_info.project_id,
            auth_info.user_id,
            &auth_info.username,
            &body.path,
            ref_name,
        )
        .await
    {
        Ok(lock) => Ok(HttpResponse::Created()
            .content_type("application/vnd.git-lfs+json")
            .json(LfsLockResponse { lock })),
        Err(LfsClientError::ServerError(msg)) if msg.contains("already locked") => {
            Ok(HttpResponse::Conflict()
                .content_type("application/vnd.git-lfs+json")
                .json(serde_json::json!({
                    "message": msg,
                })))
        }
        Err(e) => {
            error!("Failed to create lock: {}", e);
            Ok(lfs_error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to create lock: {}", e),
            ))
        }
    }
}

/// 列出锁
/// GET /{namespace}/{project}.git/info/lfs/locks
pub async fn handle_list_locks(
    req: HttpRequest,
    query: web::Query<LfsListLocksQuery>,
    state: web::Data<LfsState>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    debug!("LFS list locks: {}", path);

    let (namespace, project, _) = match parse_lfs_path(path) {
        Some(p) => p,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                "Invalid LFS path",
            ));
        }
    };

    let repo_path = format!("{}/{}", namespace, project);

    // 认证（只需读权限）
    let auth_info = match authenticate_lfs_request(&req, &repo_path, "download", &state).await {
        Ok(info) => info,
        Err(e) => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::UNAUTHORIZED,
                &e,
            ));
        }
    };

    let lfs_client = match &state.lfs_client {
        Some(c) => c,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::NOT_IMPLEMENTED,
                "LFS locks not configured",
            ));
        }
    };

    match lfs_client
        .list_locks(
            auth_info.project_id,
            query.path.clone(),
            query.id.clone(),
            query.cursor.clone(),
            query.limit,
            query.refspec.clone(),
        )
        .await
    {
        Ok((locks, next_cursor)) => Ok(HttpResponse::Ok()
            .content_type("application/vnd.git-lfs+json")
            .json(LfsListLocksResponse { locks, next_cursor })),
        Err(e) => {
            error!("Failed to list locks: {}", e);
            Ok(lfs_error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to list locks: {}", e),
            ))
        }
    }
}

/// 删除锁
/// POST /{namespace}/{project}.git/info/lfs/locks/{id}/unlock
pub async fn handle_delete_lock(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    body: web::Json<LfsUnlockRequest>,
    state: web::Data<LfsState>,
) -> Result<HttpResponse, Error> {
    let (namespace, project, lock_id) = path.into_inner();
    let repo_path = format!("{}/{}", namespace, project);

    debug!("LFS delete lock: {} in {}", lock_id, repo_path);

    // 认证（解锁需要写权限）
    let auth_info = match authenticate_lfs_request(&req, &repo_path, "upload", &state).await {
        Ok(info) => info,
        Err(e) => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::UNAUTHORIZED,
                &e,
            ));
        }
    };

    if !auth_info.can_write {
        return Ok(lfs_error_response(
            actix_web::http::StatusCode::FORBIDDEN,
            "Write access required for unlocking",
        ));
    }

    let lfs_client = match &state.lfs_client {
        Some(c) => c,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::NOT_IMPLEMENTED,
                "LFS locks not configured",
            ));
        }
    };

    let ref_name = body.ref_info.as_ref().map(|r| r.name.clone());

    match lfs_client
        .delete_lock(
            auth_info.project_id,
            auth_info.user_id,
            &lock_id,
            body.force,
            ref_name,
        )
        .await
    {
        Ok(lock) => Ok(HttpResponse::Ok()
            .content_type("application/vnd.git-lfs+json")
            .json(LfsUnlockResponse { lock })),
        Err(LfsClientError::ServerError(msg)) if msg.contains("not found") => {
            Ok(lfs_error_response(
                actix_web::http::StatusCode::NOT_FOUND,
                &msg,
            ))
        }
        Err(LfsClientError::ServerError(msg)) if msg.contains("not owner") => {
            Ok(lfs_error_response(
                actix_web::http::StatusCode::FORBIDDEN,
                &msg,
            ))
        }
        Err(e) => {
            error!("Failed to delete lock: {}", e);
            Ok(lfs_error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to delete lock: {}", e),
            ))
        }
    }
}

/// 验证锁
/// POST /{namespace}/{project}.git/info/lfs/locks/verify
pub async fn handle_verify_locks(
    req: HttpRequest,
    body: web::Json<LfsVerifyLocksRequest>,
    state: web::Data<LfsState>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    debug!("LFS verify locks: {}", path);

    let (namespace, project, _) = match parse_lfs_path(path) {
        Some(p) => p,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::BAD_REQUEST,
                "Invalid LFS path",
            ));
        }
    };

    let repo_path = format!("{}/{}", namespace, project);

    // 认证（验证锁需要读权限）
    let auth_info = match authenticate_lfs_request(&req, &repo_path, "download", &state).await {
        Ok(info) => info,
        Err(e) => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::UNAUTHORIZED,
                &e,
            ));
        }
    };

    let lfs_client = match &state.lfs_client {
        Some(c) => c,
        None => {
            return Ok(lfs_error_response(
                actix_web::http::StatusCode::NOT_IMPLEMENTED,
                "LFS locks not configured",
            ));
        }
    };

    let ref_name = body.ref_info.as_ref().map(|r| r.name.clone());

    match lfs_client
        .verify_locks(
            auth_info.project_id,
            auth_info.user_id,
            body.cursor.clone(),
            body.limit,
            ref_name,
        )
        .await
    {
        Ok(response) => Ok(HttpResponse::Ok()
            .content_type("application/vnd.git-lfs+json")
            .json(response)),
        Err(e) => {
            error!("Failed to verify locks: {}", e);
            Ok(lfs_error_response(
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to verify locks: {}", e),
            ))
        }
    }
}

// ============ 辅助函数 ============

/// 认证 LFS 请求
/// 
/// 通过 gRPC Auth 服务验证权限，实现三层权限交集：
/// Token Scopes ∩ User Permission ∩ Project Membership
async fn authenticate_lfs_request(
    req: &HttpRequest,
    repo_path: &str,
    action: &str,
    state: &LfsState,
) -> Result<LfsAuthInfo, String> {
    let auth_client = match &state.auth_client {
        Some(c) => c.clone(),
        None => {
            // 没有 Auth 客户端配置 - 只允许公开仓库的读取
            if action == "download" {
                return Ok(LfsAuthInfo {
                    user_id: 0,
                    username: "anonymous".to_string(),
                    project_id: 0,
                    can_write: false,
                });
            }
            return Err("Authentication service not configured".to_string());
        }
    };

    let mut auth_client = auth_client;
    let auth_header = extract_auth_info(req);

    // 调用 gRPC Auth 服务
    let access_result = match auth_header {
        Some((username, password)) => {
            // 有认证信息：Basic auth 或 Bearer token
            // password 可能是密码、PAT 或 OAuth2 token
            if username.is_empty() {
                // Bearer token - 作为 JWT 或 OAuth2 token
                auth_client.check_http_access_jwt(repo_path, action, &password).await
            } else {
                // Basic auth - password 可能是密码、PAT 或 OAuth2 token
                auth_client.check_http_access_basic(repo_path, action, &username, &password).await
            }
        }
        None => {
            // 无认证 - 匿名访问
            auth_client.check_http_access_anonymous(repo_path, action).await
        }
    };

    match access_result {
        Ok(result) => {
            if result.allowed {
                Ok(LfsAuthInfo {
                    user_id: result.user_id,
                    username: result.username,
                    project_id: result.project_id,
                    can_write: result.can_write,
                })
            } else {
                Err(result.message)
            }
        }
        Err(e) => {
            tracing::error!("Auth gRPC error: {}", e);
            Err("Authentication service unavailable".to_string())
        }
    }
}

/// 获取基础 URL
fn get_base_url(req: &HttpRequest, config: &Config) -> String {
    // 优先使用配置的外部 URL
    if let Some(ref url) = config.lfs_external_url {
        return url.clone();
    }

    // 从请求头中构建
    let scheme = req.connection_info().scheme().to_string();
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    format!("{}://{}", scheme, host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lfs_path() {
        let result = parse_lfs_path("/user/repo.git/info/lfs/objects/batch");
        assert!(result.is_some());
        let (ns, proj, path) = result.unwrap();
        assert_eq!(ns, "user");
        assert_eq!(proj, "repo");
        assert_eq!(path, "objects/batch");

        let result = parse_lfs_path("/org/project.git/info/lfs/locks");
        assert!(result.is_some());
        let (ns, proj, path) = result.unwrap();
        assert_eq!(ns, "org");
        assert_eq!(proj, "project");
        assert_eq!(path, "locks");
    }

    #[test]
    fn test_parse_invalid_path() {
        assert!(parse_lfs_path("/user/repo/info/lfs/batch").is_none());
        assert!(parse_lfs_path("/user.git/info/lfs/batch").is_none());
    }
}
