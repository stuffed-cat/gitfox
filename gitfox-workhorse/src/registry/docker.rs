//! Docker Registry V2 API 处理器
//!
//! 实现 Docker Registry HTTP API V2 规范
//! 参考: https://docs.docker.com/registry/spec/api/

use actix_web::{web, HttpRequest, HttpResponse, http::StatusCode};
use futures::StreamExt;
use tracing::{debug, info, warn, error};
use std::sync::Arc;

use super::types::*;
use super::storage::{RegistryStorage, StorageError};
use super::auth::{docker_authenticate, extract_basic_auth, extract_bearer_token, generate_docker_token};
use super::config::RegistryConfig;
use crate::auth_client::AuthClient;
use crate::registry_client::RegistryApiClient;

/// Docker Registry 状态
pub struct DockerRegistryState {
    pub storage: RegistryStorage,
    pub config: Arc<RegistryConfig>,
    pub auth_client: Option<tokio::sync::Mutex<AuthClient>>,
    pub registry_client: Option<RegistryApiClient>,
    pub shell_secret: String,
    pub backend_url: String,
}

impl DockerRegistryState {
    pub fn new(config: Arc<RegistryConfig>, shell_secret: String, backend_url: String) -> Self {
        let storage = RegistryStorage::new(&config.storage_path);
        let registry_client = Some(RegistryApiClient::new(&backend_url, &shell_secret));
        Self {
            storage,
            config,
            auth_client: None,
            registry_client,
            shell_secret,
            backend_url,
        }
    }

    /// 初始化存储
    pub async fn init(&self) -> std::io::Result<()> {
        self.storage.init().await
    }

    /// 设置认证客户端
    pub async fn set_auth_client(&mut self, address: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = AuthClient::connect(address, self.shell_secret.clone()).await?;
        self.auth_client = Some(tokio::sync::Mutex::new(client));
        Ok(())
    }
    
    /// 获取 registry client（如果不可用则返回错误）
    pub fn get_registry_client(&self) -> Result<&RegistryApiClient, HttpResponse> {
        self.registry_client.as_ref().ok_or_else(|| {
            error!("Registry client not configured");
            HttpResponse::InternalServerError().json(DockerError::internal_error("Registry not available"))
        })
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 解析 Docker 镜像名获取项目 ID 和镜像名
/// Docker name 格式: namespace/project 或 namespace/project/image_name
/// 返回 (project_id, image_name)
async fn parse_docker_name(
    name: &str,
    state: &web::Data<DockerRegistryState>,
) -> Result<(i64, String), HttpResponse> {
    let parts: Vec<&str> = name.split('/').collect();
    
    if parts.len() < 2 {
        return Err(HttpResponse::build(StatusCode::NOT_FOUND)
            .json(DockerError::name_unknown(name)));
    }
    
    // 提取 namespace 和 project
    let namespace = parts[0];
    let project = parts[1];
    
    // 剩余部分作为 image_name，如果没有则使用 project 名称
    let image_name = if parts.len() > 2 {
        parts[2..].join("/")
    } else {
        project.to_string()
    };
    
    // 调用 API 解析项目
    let registry_client = state.get_registry_client()?;
    
    match registry_client.resolve_project(namespace, project).await {
        Ok(res) => Ok((res.project_id, image_name)),
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            Err(HttpResponse::build(StatusCode::NOT_FOUND)
                .json(DockerError::name_unknown(name)))
        }
        Err(e) => {
            error!("Failed to resolve project: {}", e);
            Err(HttpResponse::InternalServerError().json(DockerError::internal_error("Project lookup failed")))
        }
    }
}

// ============================================================================
// API 端点处理器
// ============================================================================

/// GET /v2/ - API 版本检查
/// 也用于验证认证（当没有认证时返回 401）
pub async fn handle_v2_check(
    req: HttpRequest,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    debug!("Docker Registry V2 check");
    
    // 检查认证（如果提供了凭据）
    if extract_basic_auth(&req).is_some() || extract_bearer_token(&req).is_some() {
        // 有认证信息，验证它
        // 这里简单返回成功，实际认证在访问具体资源时进行
        return HttpResponse::Ok()
            .insert_header(("Docker-Distribution-API-Version", "registry/2.0"))
            .finish();
    }
    
    // 无认证信息，返回成功（允许匿名访问 /v2/）
    HttpResponse::Ok()
        .insert_header(("Docker-Distribution-API-Version", "registry/2.0"))
        .finish()
}

/// GET /v2/auth - Token 认证端点
/// Docker 客户端用此端点获取访问 token
/// 
/// **三层权限交集**:
/// 最终权限 = Token Scopes ∩ User Role ∩ Project Membership
pub async fn handle_token_auth(
    req: HttpRequest,
    query: web::Query<TokenAuthQuery>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    debug!("Docker token auth: {:?}", query);
    
    let scope = query.scope.as_deref().unwrap_or("");
    let _service = query.service.as_deref().unwrap_or("registry");
    
    // 解析 scope: repository:namespace/project:pull,push
    let (repository, actions) = if !scope.is_empty() {
        let parts: Vec<&str> = scope.split(':').collect();
        if parts.len() >= 3 && parts[0] == "repository" {
            let repo = parts[1];
            let actions: Vec<&str> = parts[2].split(',').collect();
            (repo, actions)
        } else {
            ("", vec![])
        }
    } else {
        ("", vec![])
    };

    // 确定需要的操作类型
    let needs_write = actions.iter().any(|a| *a == "push");
    let grpc_action = if needs_write { "git-receive-pack" } else { "git-upload-pack" };

    // 通过 gRPC Auth 服务验证权限
    let (username, can_write) = match &state.auth_client {
        Some(auth_mutex) => {
            let mut auth_client = auth_mutex.lock().await;
            let repo_path = repository.to_string();

            // 尝试认证
            let result = if let Some((user, pass)) = extract_basic_auth(&req) {
                // Basic Auth - password 可能是密码、PAT 或 OAuth2 token
                auth_client.check_http_access_basic(&repo_path, grpc_action, &user, &pass).await
            } else if let Some(token) = extract_bearer_token(&req) {
                // Bearer token
                auth_client.check_http_access_jwt(&repo_path, grpc_action, &token).await
            } else {
                // 匿名访问
                auth_client.check_http_access_anonymous(&repo_path, grpc_action).await
            };

            match result {
                Ok(access_result) if access_result.allowed => {
                    (access_result.username, access_result.can_write)
                }
                Ok(access_result) => {
                    warn!("Docker auth denied: {}", access_result.message);
                    ("anonymous".to_string(), false)
                }
                Err(e) => {
                    error!("Auth service error: {}", e);
                    ("anonymous".to_string(), false)
                }
            }
        }
        None => {
            // 没有 Auth 客户端 - 只允许匿名 pull
            warn!("Auth client not configured for Docker registry");
            ("anonymous".to_string(), false)
        }
    };

    // 生成 token - 根据实际权限过滤允许的操作
    let allowed_actions: Vec<&str> = if can_write {
        actions
    } else {
        actions.into_iter().filter(|a| *a == "pull").collect()
    };

    let token_response = generate_docker_token(
        &username,
        repository,
        &allowed_actions,
        state.config.token_expires,
        &state.shell_secret,
    );

    HttpResponse::Ok().json(token_response)
}

#[derive(Debug, serde::Deserialize)]
pub struct TokenAuthQuery {
    pub service: Option<String>,
    pub scope: Option<String>,
    pub account: Option<String>,
}

/// HEAD /v2/{name}/blobs/{digest} - 检查 blob 是否存在
pub async fn handle_blob_head(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, digest) = path.into_inner();
    debug!("Docker blob HEAD: name={}, digest={}", name, digest);

    // 检查 blob 是否存在
    if !state.storage.docker_blob_exists(&digest).await {
        return HttpResponse::build(StatusCode::NOT_FOUND)
            .json(DockerError::blob_unknown());
    }

    // 获取大小
    let size = state.storage.docker_blob_size(&digest).await.unwrap_or(0);

    HttpResponse::Ok()
        .insert_header(("Docker-Content-Digest", digest.as_str()))
        .insert_header(("Content-Length", size.to_string()))
        .insert_header(("Content-Type", "application/octet-stream"))
        .finish()
}

/// GET /v2/{name}/blobs/{digest} - 下载 blob
pub async fn handle_blob_get(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, digest) = path.into_inner();
    debug!("Docker blob GET: name={}, digest={}", name, digest);

    // 打开文件
    let file = match state.storage.open_docker_blob(&digest).await {
        Ok(f) => f,
        Err(_) => {
            return HttpResponse::build(StatusCode::NOT_FOUND)
                .json(DockerError::blob_unknown());
        }
    };

    // 获取大小
    let size = state.storage.docker_blob_size(&digest).await.unwrap_or(0);

    // 流式返回文件内容
    use tokio_util::io::ReaderStream;
    let stream = ReaderStream::new(file);

    HttpResponse::Ok()
        .insert_header(("Docker-Content-Digest", digest.as_str()))
        .insert_header(("Content-Length", size.to_string()))
        .insert_header(("Content-Type", "application/octet-stream"))
        .streaming(stream)
}

/// POST /v2/{name}/blobs/uploads/ - 开始上传
pub async fn handle_blob_upload_start(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<BlobUploadQuery>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let name = path.into_inner();
    debug!("Docker blob upload start: name={}", name);

    // 解析 Docker name 获取 project_id
    let (project_id, _image_name) = match parse_docker_name(&name, &state).await {
        Ok(p) => p,
        Err(e) => return e,
    };

    // 检查是否是 mount 操作
    if let Some(ref from) = query.from {
        if let Some(ref digest) = query.mount {
            // 尝试从其他仓库挂载 blob
            if state.storage.docker_blob_exists(digest).await {
                // blob 存在，直接挂载成功
                return HttpResponse::Created()
                    .insert_header(("Location", format!("/v2/{}/blobs/{}", name, digest)))
                    .insert_header(("Docker-Content-Digest", digest.as_str()))
                    .finish();
            }
        }
    }

    // 创建上传会话
    let uuid = uuid::Uuid::new_v4().to_string();
    let (temp_path, _) = match state.storage.create_temp_file().await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create temp file: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    // 在数据库中创建上传会话记录
    if let Some(ref client) = state.registry_client {
        // 从请求头获取用户ID（如果有认证信息）
        let user_id = extract_user_id_from_request(&req).unwrap_or_else(|| "anonymous".to_string());
        
        match client.create_upload_session(
            project_id,
            &user_id,
            query.digest.as_deref(),
            temp_path.to_string_lossy().as_ref(),
        ).await {
            Ok(session) => {
                debug!("Created upload session: {} for project {}", session.uuid, project_id);
                // 使用数据库返回的 UUID
                return HttpResponse::Accepted()
                    .insert_header(("Location", format!("/v2/{}/blobs/uploads/{}", name, session.uuid)))
                    .insert_header(("Docker-Upload-UUID", session.uuid.as_str()))
                    .insert_header(("Range", "0-0"))
                    .finish();
            }
            Err(e) => {
                error!("Failed to create upload session in database: {}", e);
                // 回退到本地 UUID，不阻塞上传
            }
        }
    }

    // 使用本地生成的 UUID 作为回退
    HttpResponse::Accepted()
        .insert_header(("Location", format!("/v2/{}/blobs/uploads/{}", name, uuid)))
        .insert_header(("Docker-Upload-UUID", uuid.as_str()))
        .insert_header(("Range", "0-0"))
        .finish()
}

/// 从请求头提取用户标识
/// 
/// 支持以下认证方式：
/// - Basic Auth: 直接提取用户名
/// - Bearer Token (JWT): 解析 JWT payload 提取 sub (用户名) 或 user_id
/// - Bearer Token (PAT): gitfox-pat_ 开头的 token，返回脱敏标识
/// - Bearer Token (OAuth): 其他 token，返回 SHA-256 哈希前缀作为标识
fn extract_user_id_from_request(req: &HttpRequest) -> Option<String> {
    let auth_header = req.headers().get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    if auth_str.starts_with("Basic ") {
        // Basic auth: 直接提取用户名
        return extract_basic_auth(req).map(|(username, _)| username);
    }
    
    if auth_str.starts_with("Bearer ") {
        let token = &auth_str[7..];
        
        // 尝试解析为 JWT token
        if let Some(user_id) = parse_jwt_user_info(token) {
            return Some(user_id);
        }
        
        // 检查是否是 PAT token
        if token.starts_with("gitfox-pat_") {
            // 返回脱敏的 PAT 标识符（保留前缀 + 部分字符）
            let safe_len = token.len().min(20);
            return Some(format!("pat:{}", &token[..safe_len]));
        }
        
        // 其他 token（如 OAuth access token）：返回 SHA-256 哈希前缀
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let hash = hex::encode(hasher.finalize());
        return Some(format!("oauth:{}", &hash[..16]));
    }
    
    None
}

/// 从 JWT token 中解析用户信息
/// 
/// 不验证签名，只解析 payload 部分（因为验证已在之前的认证步骤完成）
/// 返回用户名或用户 ID
fn parse_jwt_user_info(token: &str) -> Option<String> {
    use base64::Engine;
    
    // JWT 格式: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    
    // 解码 payload（第二部分）
    let payload = parts[1];
    // JWT 使用 URL-safe base64 编码，可能没有 padding
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(payload))
        .ok()?;
    
    let payload_str = String::from_utf8(decoded).ok()?;
    
    // 解析为 JSON 并提取用户信息
    #[derive(serde::Deserialize)]
    struct JwtPayload {
        /// 用户名（标准 JWT subject claim）
        sub: Option<String>,
        /// 用户 ID（GitFox 自定义）
        user_id: Option<i64>,
        /// 用户名（有些系统使用 username 字段）
        username: Option<String>,
    }
    
    let payload: JwtPayload = serde_json::from_str(&payload_str).ok()?;
    
    // 优先返回用户名，其次返回 user_id
    if let Some(sub) = payload.sub {
        return Some(sub);
    }
    if let Some(username) = payload.username {
        return Some(username);
    }
    if let Some(user_id) = payload.user_id {
        return Some(format!("user:{}", user_id));
    }
    
    None
}

#[derive(Debug, serde::Deserialize)]
pub struct BlobUploadQuery {
    pub digest: Option<String>,
    pub mount: Option<String>,
    pub from: Option<String>,
}

/// PATCH /v2/{name}/blobs/uploads/{uuid} - 追加上传数据
pub async fn handle_blob_upload_patch(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Bytes,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, uuid) = path.into_inner();
    debug!("Docker blob upload PATCH: name={}, uuid={}, bytes={}", name, uuid, body.len());

    // 获取临时文件路径
    let temp_path = state.storage.tmp_path().join(format!("upload-{}", uuid));
    
    // 追加数据
    let new_size = match state.storage.append_to_temp(&temp_path, &body).await {
        Ok(s) => s,
        Err(e) => {
            // 如果文件不存在，创建它
            if let Ok((_, mut file)) = state.storage.create_temp_file().await {
                use tokio::io::AsyncWriteExt;
                if let Err(e) = file.write_all(&body).await {
                    error!("Failed to write to temp file: {}", e);
                    return HttpResponse::InternalServerError().finish();
                }
                body.len() as i64
            } else {
                error!("Failed to create temp file: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    };

    HttpResponse::Accepted()
        .insert_header(("Location", format!("/v2/{}/blobs/uploads/{}", name, uuid)))
        .insert_header(("Docker-Upload-UUID", uuid.as_str()))
        .insert_header(("Range", format!("0-{}", new_size - 1)))
        .finish()
}

/// PUT /v2/{name}/blobs/uploads/{uuid} - 完成上传
pub async fn handle_blob_upload_put(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    query: web::Query<BlobUploadQuery>,
    body: web::Bytes,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, uuid) = path.into_inner();
    debug!("Docker blob upload PUT: name={}, uuid={}", name, uuid);

    let digest = match &query.digest {
        Some(d) => d.clone(),
        None => {
            return HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(DockerError::digest_invalid());
        }
    };

    // 获取临时文件路径
    let temp_path = state.storage.tmp_path().join(format!("upload-{}", uuid));

    // 如果有 body，追加到临时文件
    if !body.is_empty() {
        if let Err(e) = state.storage.append_to_temp(&temp_path, &body).await {
            // 如果临时文件不存在，直接存储
            match state.storage.store_docker_blob(&digest, &body).await {
                Ok(path) => {
                    return HttpResponse::Created()
                        .insert_header(("Location", format!("/v2/{}/blobs/{}", name, digest)))
                        .insert_header(("Docker-Content-Digest", digest.as_str()))
                        .finish();
                }
                Err(StorageError::DigestMismatch { .. }) => {
                    return HttpResponse::build(StatusCode::BAD_REQUEST)
                        .json(DockerError::digest_invalid());
                }
                Err(e) => {
                    error!("Failed to store blob: {}", e);
                    return HttpResponse::InternalServerError().finish();
                }
            }
        }
    }

    // 完成上传
    match state.storage.finalize_docker_blob(&temp_path, &digest).await {
        Ok(_) => {
            HttpResponse::Created()
                .insert_header(("Location", format!("/v2/{}/blobs/{}", name, digest)))
                .insert_header(("Docker-Content-Digest", digest.as_str()))
                .finish()
        }
        Err(StorageError::DigestMismatch { .. }) => {
            HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(DockerError::digest_invalid())
        }
        Err(e) => {
            error!("Failed to finalize blob: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// DELETE /v2/{name}/blobs/uploads/{uuid} - 取消上传
pub async fn handle_blob_upload_delete(
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, uuid) = path.into_inner();
    debug!("Docker blob upload DELETE: name={}, uuid={}", name, uuid);

    // 删除临时文件
    let temp_path = state.storage.tmp_path().join(format!("upload-{}", uuid));
    let _ = state.storage.delete_temp_file(&temp_path).await;

    HttpResponse::NoContent().finish()
}

/// DELETE /v2/{name}/blobs/{digest} - 删除 blob
/// 
/// 删除前检查引用计数：如果 blob 仍被 manifest 引用，则返回 405 Method Not Allowed。
/// 成功删除数据库记录后，同时删除物理文件。
pub async fn handle_blob_delete(
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, digest) = path.into_inner();
    debug!("Docker blob DELETE: name={}, digest={}", name, digest);

    // 解析 name 获取 project_id
    let (project_id, _image_name) = match parse_docker_name(&name, &state).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let registry_client = match state.get_registry_client() {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // 调用 API 删除 blob（API 会检查引用计数）
    match registry_client.delete_docker_blob(project_id, &digest).await {
        Ok(blob) => {
            // 数据库记录已删除，现在删除物理文件
            if let Err(e) = state.storage.delete_docker_blob(&blob.digest).await {
                // 文件删除失败只记录警告，不影响删除结果
                // 因为数据库记录已删除，文件已成为孤立文件，后续可通过 GC 清理
                warn!(
                    "Database record deleted but failed to delete blob file {}: {}",
                    blob.file_path, e
                );
            }
            HttpResponse::Accepted().finish()
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            // Blob 不存在
            HttpResponse::build(StatusCode::NOT_FOUND)
                .json(DockerError::blob_unknown())
        }
        Err(crate::registry_client::RegistryApiError::Conflict) => {
            // Blob 仍被 manifest 引用，不能删除
            // Docker Registry V2 规范建议返回 405 Method Not Allowed
            warn!(
                "Cannot delete blob {} for project {}: still referenced by manifests",
                digest, project_id
            );
            HttpResponse::build(StatusCode::METHOD_NOT_ALLOWED)
                .json(DockerError::new(
                    "BLOB_REFERENCED",
                    "blob is referenced by one or more manifests",
                ))
        }
        Err(e) => {
            error!("Failed to delete blob: {}", e);
            HttpResponse::InternalServerError()
                .json(DockerError::internal_error("Failed to delete blob"))
        }
    }
}

/// HEAD /v2/{name}/manifests/{reference} - 检查 manifest
pub async fn handle_manifest_head(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    debug!("Docker manifest HEAD: name={}, reference={}", name, reference);

    // 解析 name 获取 project_id
    let (project_id, image_name) = match parse_docker_name(&name, &state).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let registry_client = match state.get_registry_client() {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // 根据 reference 类型查询 manifest
    let manifest_result = if reference.starts_with("sha256:") {
        registry_client.get_docker_manifest_by_digest(project_id, &image_name, &reference).await
    } else {
        registry_client.get_docker_manifest_by_tag(project_id, &image_name, &reference).await
    };

    match manifest_result {
        Ok(manifest) => {
            HttpResponse::Ok()
                .insert_header(("Docker-Content-Digest", manifest.digest.as_str()))
                .insert_header(("Content-Type", manifest.media_type.as_str()))
                .insert_header(("Content-Length", manifest.manifest_json.to_string().len().to_string()))
                .finish()
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::build(StatusCode::NOT_FOUND)
                .json(DockerError::manifest_unknown())
        }
        Err(e) => {
            error!("Failed to get manifest: {}", e);
            HttpResponse::InternalServerError().json(DockerError::internal_error("Database error"))
        }
    }
}

/// GET /v2/{name}/manifests/{reference} - 获取 manifest
pub async fn handle_manifest_get(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    debug!("Docker manifest GET: name={}, reference={}", name, reference);

    // 解析 name 获取 project_id
    let (project_id, image_name) = match parse_docker_name(&name, &state).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let registry_client = match state.get_registry_client() {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    // 根据 reference 类型查询 manifest
    let manifest_result = if reference.starts_with("sha256:") {
        registry_client.get_docker_manifest_by_digest(project_id, &image_name, &reference).await
    } else {
        registry_client.get_docker_manifest_by_tag(project_id, &image_name, &reference).await
    };

    match manifest_result {
        Ok(manifest) => {
            HttpResponse::Ok()
                .insert_header(("Docker-Content-Digest", manifest.digest.as_str()))
                .insert_header(("Content-Type", manifest.media_type.as_str()))
                .json(manifest.manifest_json)
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::build(StatusCode::NOT_FOUND)
                .json(DockerError::manifest_unknown())
        }
        Err(e) => {
            error!("Failed to get manifest: {}", e);
            HttpResponse::InternalServerError().json(DockerError::internal_error("Database error"))
        }
    }
}

/// PUT /v2/{name}/manifests/{reference} - 上传 manifest
pub async fn handle_manifest_put(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Bytes,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    debug!("Docker manifest PUT: name={}, reference={}", name, reference);

    // 解析 name 获取 project_id
    let (project_id, image_name) = match parse_docker_name(&name, &state).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    // 获取 Content-Type
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/vnd.docker.distribution.manifest.v2+json");

    // 计算 digest
    let digest = format!("sha256:{}", RegistryStorage::calculate_sha256_bytes(&body));

    // 解析 manifest
    let manifest: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(DockerError::manifest_invalid(&e.to_string()));
        }
    };

    // 收集所有 blob digests
    let mut blob_digests = Vec::new();
    let mut config_digest = None;
    let mut total_size: i64 = 0;

    // 验证所有引用的 blob 存在并收集信息
    if let Some(config) = manifest.get("config") {
        if let Some(digest_str) = config.get("digest").and_then(|d| d.as_str()) {
            if !state.storage.docker_blob_exists(digest_str).await {
                return HttpResponse::build(StatusCode::BAD_REQUEST)
                    .json(DockerError::blob_unknown());
            }
            config_digest = Some(digest_str.to_string());
            blob_digests.push(digest_str.to_string());
            if let Some(size) = config.get("size").and_then(|s| s.as_i64()) {
                total_size += size;
            }
        }
    }

    if let Some(layers) = manifest.get("layers").and_then(|l| l.as_array()) {
        for layer in layers {
            if let Some(layer_digest) = layer.get("digest").and_then(|d| d.as_str()) {
                if !state.storage.docker_blob_exists(layer_digest).await {
                    return HttpResponse::build(StatusCode::BAD_REQUEST)
                        .json(DockerError::blob_unknown());
                }
                blob_digests.push(layer_digest.to_string());
                if let Some(size) = layer.get("size").and_then(|s| s.as_i64()) {
                    total_size += size;
                }
            }
        }
    }

    // 获取 schema version
    let schema_version = manifest.get("schemaVersion")
        .and_then(|v| v.as_i64())
        .unwrap_or(2) as i32;

    // 确定 tag（如果 reference 是 digest，则使用 digest 作为 tag）
    let tag = if reference.starts_with("sha256:") {
        reference.clone()
    } else {
        reference.clone()
    };

    // 存储 manifest 到数据库
    let registry_client = match state.get_registry_client() {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let create_request = crate::registry_client::CreateDockerManifestRequest {
        project_id,
        image_name: image_name.clone(),
        tag,
        digest: digest.clone(),
        media_type: content_type.to_string(),
        schema_version,
        config_digest,
        total_size,
        manifest_json: manifest,
        blob_digests,
    };

    match registry_client.create_docker_manifest(&create_request).await {
        Ok(_) => {
            info!("Docker manifest uploaded: name={}, reference={}, digest={}", name, reference, digest);
            HttpResponse::Created()
                .insert_header(("Location", format!("/v2/{}/manifests/{}", name, reference)))
                .insert_header(("Docker-Content-Digest", digest.as_str()))
                .finish()
        }
        Err(e) => {
            error!("Failed to create manifest: {}", e);
            HttpResponse::InternalServerError().json(DockerError::internal_error("Failed to store manifest"))
        }
    }
}

/// DELETE /v2/{name}/manifests/{reference} - 删除 manifest
pub async fn handle_manifest_delete(
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    debug!("Docker manifest DELETE: name={}, reference={}", name, reference);

    // 解析 name 获取 project_id
    let (project_id, image_name) = match parse_docker_name(&name, &state).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let registry_client = match state.get_registry_client() {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    match registry_client.delete_docker_manifest(project_id, &image_name, &reference).await {
        Ok(_) => HttpResponse::Accepted().finish(),
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::build(StatusCode::NOT_FOUND)
                .json(DockerError::manifest_unknown())
        }
        Err(e) => {
            error!("Failed to delete manifest: {}", e);
            HttpResponse::InternalServerError().json(DockerError::internal_error("Failed to delete manifest"))
        }
    }
}

/// GET /v2/{name}/tags/list - 列出标签
pub async fn handle_tags_list(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<TagsListQuery>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let name = path.into_inner();
    debug!("Docker tags list: name={}", name);

    // 解析 name 获取 project_id
    let (project_id, image_name) = match parse_docker_name(&name, &state).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let registry_client = match state.get_registry_client() {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    match registry_client.list_docker_tags(project_id, &image_name).await {
        Ok(tag_list) => {
            // 处理分页（可选）
            let mut tags = tag_list.tags;
            
            // 如果指定了 last，从 last 之后开始
            if let Some(ref last) = query.last {
                if let Some(pos) = tags.iter().position(|t| t == last) {
                    tags = tags.into_iter().skip(pos + 1).collect();
                }
            }
            
            // 如果指定了 n，限制数量
            if let Some(n) = query.n {
                tags.truncate(n as usize);
            }
            
            HttpResponse::Ok().json(DockerTagList {
                name: format!("{}/{}", name, image_name),
                tags,
            })
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::build(StatusCode::NOT_FOUND)
                .json(DockerError::name_unknown(&name))
        }
        Err(e) => {
            error!("Failed to list tags: {}", e);
            HttpResponse::InternalServerError().json(DockerError::internal_error("Failed to list tags"))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct TagsListQuery {
    pub n: Option<i32>,
    pub last: Option<String>,
}

/// GET /v2/_catalog - 列出所有仓库
/// 
/// 注意: Docker Registry V2 的 _catalog 端点需要列出所有用户有权访问的仓库，
/// 但标准 Docker 客户端通常不使用此端点。此端点主要用于管理工具。
/// 
/// 当前实现返回空列表，因为：
/// 1. 需要认证上下文来确定用户有权访问哪些项目
/// 2. 跨所有项目聚合 Docker 镜像列表需要复杂的权限检查
/// 
/// 如果需要列出特定项目的镜像，请使用 /v2/{namespace}/{project}/tags/list
pub async fn handle_catalog(
    query: web::Query<CatalogQuery>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    debug!("Docker catalog request (returning empty - not implemented for multi-tenant)");

    // _catalog 端点在多租户环境下难以实现，因为需要：
    // 1. 获取当前用户的认证信息
    // 2. 查询用户有权访问的所有项目
    // 3. 聚合每个项目的 Docker 镜像列表
    // 
    // 大多数 Docker 客户端不依赖此端点，因此返回空列表是安全的
    let response = DockerCatalog {
        repositories: vec![],
    };

    HttpResponse::Ok().json(response)
}

#[derive(Debug, serde::Deserialize)]
pub struct CatalogQuery {
    pub n: Option<i32>,
    pub last: Option<String>,
}

// ============================================================================
// 路由配置
// ============================================================================

/// 配置 Docker Registry 路由
pub fn configure_docker_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // V2 API 检查
        .route("/v2/", web::get().to(handle_v2_check))
        .route("/v2", web::get().to(handle_v2_check))
        // Token 认证
        .route("/v2/auth", web::get().to(handle_token_auth))
        // Catalog（必须在通配路由之前）
        .route("/v2/_catalog", web::get().to(handle_catalog))
        // Blob 操作
        .route("/v2/{name:.*}/blobs/{digest}", web::head().to(handle_blob_head))
        .route("/v2/{name:.*}/blobs/{digest}", web::get().to(handle_blob_get))
        .route("/v2/{name:.*}/blobs/{digest}", web::delete().to(handle_blob_delete))
        // Blob 上传
        .route("/v2/{name:.*}/blobs/uploads/", web::post().to(handle_blob_upload_start))
        .route("/v2/{name:.*}/blobs/uploads/{uuid}", web::patch().to(handle_blob_upload_patch))
        .route("/v2/{name:.*}/blobs/uploads/{uuid}", web::put().to(handle_blob_upload_put))
        .route("/v2/{name:.*}/blobs/uploads/{uuid}", web::delete().to(handle_blob_upload_delete))
        // Manifest 操作
        .route("/v2/{name:.*}/manifests/{reference}", web::head().to(handle_manifest_head))
        .route("/v2/{name:.*}/manifests/{reference}", web::get().to(handle_manifest_get))
        .route("/v2/{name:.*}/manifests/{reference}", web::put().to(handle_manifest_put))
        .route("/v2/{name:.*}/manifests/{reference}", web::delete().to(handle_manifest_delete))
        // Tags 列表
        .route("/v2/{name:.*}/tags/list", web::get().to(handle_tags_list));
}
