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

/// Docker Registry 状态
pub struct DockerRegistryState {
    pub storage: RegistryStorage,
    pub config: Arc<RegistryConfig>,
    pub auth_client: Option<tokio::sync::Mutex<AuthClient>>,
    pub shell_secret: String,
}

impl DockerRegistryState {
    pub fn new(config: Arc<RegistryConfig>, shell_secret: String) -> Self {
        let storage = RegistryStorage::new(&config.storage_path);
        Self {
            storage,
            config,
            auth_client: None,
            shell_secret,
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

    // TODO: 在数据库中创建上传会话记录
    // 这里简化处理，使用 UUID 作为临时文件名

    HttpResponse::Accepted()
        .insert_header(("Location", format!("/v2/{}/blobs/uploads/{}", name, uuid)))
        .insert_header(("Docker-Upload-UUID", uuid.as_str()))
        .insert_header(("Range", "0-0"))
        .finish()
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
pub async fn handle_blob_delete(
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, digest) = path.into_inner();
    debug!("Docker blob DELETE: name={}, digest={}", name, digest);

    // 注意：通常不应该直接删除 blob，因为可能被多个 manifest 引用
    // 这里简化处理，实际应该检查引用计数

    match state.storage.delete_docker_blob(&digest).await {
        Ok(_) => HttpResponse::Accepted().finish(),
        Err(e) => {
            error!("Failed to delete blob: {}", e);
            HttpResponse::InternalServerError().finish()
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

    // TODO: 从数据库查询 manifest
    // 这里返回 404，实际应该查询数据库

    HttpResponse::build(StatusCode::NOT_FOUND)
        .json(DockerError::manifest_unknown())
}

/// GET /v2/{name}/manifests/{reference} - 获取 manifest
pub async fn handle_manifest_get(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    debug!("Docker manifest GET: name={}, reference={}", name, reference);

    // TODO: 从数据库查询 manifest
    HttpResponse::build(StatusCode::NOT_FOUND)
        .json(DockerError::manifest_unknown())
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

    // 验证所有引用的 blob 存在
    if let Some(config) = manifest.get("config") {
        if let Some(config_digest) = config.get("digest").and_then(|d| d.as_str()) {
            if !state.storage.docker_blob_exists(config_digest).await {
                return HttpResponse::build(StatusCode::BAD_REQUEST)
                    .json(DockerError::blob_unknown());
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
            }
        }
    }

    // TODO: 存储 manifest 到数据库
    info!("Docker manifest uploaded: name={}, reference={}, digest={}", name, reference, digest);

    HttpResponse::Created()
        .insert_header(("Location", format!("/v2/{}/manifests/{}", name, reference)))
        .insert_header(("Docker-Content-Digest", digest.as_str()))
        .finish()
}

/// DELETE /v2/{name}/manifests/{reference} - 删除 manifest
pub async fn handle_manifest_delete(
    path: web::Path<(String, String)>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    let (name, reference) = path.into_inner();
    debug!("Docker manifest DELETE: name={}, reference={}", name, reference);

    // TODO: 从数据库删除 manifest

    HttpResponse::Accepted().finish()
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

    // TODO: 从数据库查询标签列表
    let response = DockerTagList {
        name: name.clone(),
        tags: vec![], // 从数据库获取
    };

    HttpResponse::Ok().json(response)
}

#[derive(Debug, serde::Deserialize)]
pub struct TagsListQuery {
    pub n: Option<i32>,
    pub last: Option<String>,
}

/// GET /v2/_catalog - 列出所有仓库
pub async fn handle_catalog(
    query: web::Query<CatalogQuery>,
    state: web::Data<DockerRegistryState>,
) -> HttpResponse {
    debug!("Docker catalog");

    // TODO: 从数据库查询仓库列表
    let response = DockerCatalog {
        repositories: vec![], // 从数据库获取
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
