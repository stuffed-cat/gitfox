//! npm Registry API 处理器
//!
//! 实现 npm Registry API 规范
//! 参考: https://github.com/npm/registry/blob/master/docs/REGISTRY-API.md

use actix_web::{web, HttpRequest, HttpResponse, http::StatusCode};
use base64::Engine;
use tracing::{debug, info, warn, error};
use std::collections::HashMap;
use std::sync::Arc;

use super::types::*;
use super::storage::{RegistryStorage, StorageError};
use super::auth::npm_authenticate;
use super::config::RegistryConfig;
use crate::auth_client::AuthClient;
use crate::registry_client::RegistryApiClient;

/// npm Registry 状态
pub struct NpmRegistryState {
    pub storage: RegistryStorage,
    pub config: Arc<RegistryConfig>,
    pub auth_client: Option<tokio::sync::Mutex<AuthClient>>,
    pub registry_client: Option<RegistryApiClient>,
    pub shell_secret: String,
    pub base_url: String,
    pub backend_url: String,
}

impl NpmRegistryState {
    pub fn new(config: Arc<RegistryConfig>, shell_secret: String, base_url: String, backend_url: String) -> Self {
        let storage = RegistryStorage::new(&config.storage_path);
        let registry_client = RegistryApiClient::new(&backend_url, &shell_secret);
        Self {
            storage,
            config,
            auth_client: None,
            registry_client: Some(registry_client),
            shell_secret,
            base_url,
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

    /// 生成 tarball URL
    fn tarball_url(&self, scope: &str, name: &str, filename: &str) -> String {
        if self.config.domain.is_some() {
            format!("{}/npm/@{}/{}/-/{}", self.base_url, scope, name, filename)
        } else {
            format!("{}/npm/@{}/{}/-/{}", self.base_url, scope, name, filename)
        }
    }
}

// ============================================================================
// API 端点处理器
// ============================================================================

/// GET /npm/@{scope}/{name} - 获取 scoped 包信息
/// 
/// 按 scope 和 name 在 namespace 下查找包。包可以属于该 namespace 下的任何项目。
pub async fn handle_package_get_scoped(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let (scope, name) = path.into_inner();
    let full_name = format!("@{}/{}", scope, name);
    debug!("npm package GET: {}", full_name);

    // 调用 registry client 查找包
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            error!("Registry client not initialized");
            return HttpResponse::InternalServerError()
                .json(NpmError::internal_error("Registry client not initialized"));
        }
    };

    // 使用 lookup_npm_package API 按 scope+name 查找包（无需预先知道 project_id）
    match client.lookup_npm_package(&scope, &name).await {
        Ok(doc) => {
            // 构建标准 npm registry 响应格式
            let mut versions_map = serde_json::Map::new();
            let mut time_map = serde_json::Map::new();
            
            for ver in &doc.versions {
                // 生成 tarball URL
                let tarball_filename = format!("{}-{}.tgz", name, ver.version);
                let tarball_url = state.tarball_url(&scope, &name, &tarball_filename);
                
                let version_obj = serde_json::json!({
                    "name": full_name,
                    "version": ver.version,
                    "dist": {
                        "tarball": tarball_url,
                        "integrity": ver.integrity
                    }
                });
                versions_map.insert(ver.version.clone(), version_obj);
                time_map.insert(ver.version.clone(), 
                    serde_json::Value::String(ver.created_at.to_rfc3339()));
            }
            
            let response = serde_json::json!({
                "name": doc.name,
                "versions": versions_map,
                "time": time_map,
                "dist-tags": doc.dist_tags
            });
            
            HttpResponse::Ok()
                .content_type("application/json")
                .json(response)
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            debug!("Package {} not found in scope {}", full_name, scope);
            HttpResponse::build(StatusCode::NOT_FOUND)
                .json(NpmError::not_found(&full_name))
        }
        Err(e) => {
            error!("Failed to lookup npm package {}: {}", full_name, e);
            HttpResponse::InternalServerError()
                .json(NpmError::internal_error(&format!("Failed to lookup package: {}", e)))
        }
    }
}

/// GET /npm/{name} - 获取非 scoped 包信息（不推荐，但需要支持）
pub async fn handle_package_get(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let name = path.into_inner();
    debug!("npm package GET (unscoped): {}", name);

    // GitFox 要求使用 scoped 包名
    HttpResponse::build(StatusCode::BAD_REQUEST)
        .json(NpmError::bad_request("GitFox npm registry requires scoped packages (@namespace/name)"))
}

/// PUT /npm/@{scope}/{name} - 发布 scoped 包
pub async fn handle_package_publish_scoped(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Bytes,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let (scope, name) = path.into_inner();
    let full_name = format!("@{}/{}", scope, name);
    debug!("npm package PUBLISH: {}", full_name);

    // 解析请求体
    let publish_req: NpmPublishRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to parse npm publish request: {}", e);
            return HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(NpmError::bad_request(&format!("Invalid request body: {}", e)));
        }
    };

    // 验证包名匹配
    if publish_req.name != full_name {
        return HttpResponse::build(StatusCode::BAD_REQUEST)
            .json(NpmError::bad_request("Package name in URL doesn't match body"));
    }

    // 解析项目ID - scope 对应 namespace，尝试使用 scope 作为 namespace 和 project name
    let project_id = if let Some(ref client) = state.registry_client {
        match client.resolve_project(&scope, &name).await {
            Ok(resolved) => resolved.project_id,
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                // 尝试使用 scope 作为 namespace, 空项目名 (查找默认项目)
                // 如果也找不到，则返回错误
                warn!("Cannot resolve project for scope: {}, name: {}", scope, name);
                return HttpResponse::build(StatusCode::NOT_FOUND)
                    .json(NpmError::not_found(&format!("Project not found for @{}/{}", scope, name)));
            }
            Err(e) => {
                error!("Failed to resolve project: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    } else {
        error!("Registry client not initialized");
        return HttpResponse::InternalServerError().finish();
    };

    // 处理每个版本
    for (version, version_info) in &publish_req.versions {
        debug!("Processing version: {}", version);

        // 查找对应的 attachment
        let tarball_name = format!("{}-{}.tgz", name, version);
        let attachment = publish_req.attachments.get(&tarball_name);
        
        if let Some(att) = attachment {
            // 解码 base64 数据
            let data = match base64::engine::general_purpose::STANDARD.decode(&att.data) {
                Ok(d) => d,
                Err(e) => {
                    warn!("Failed to decode tarball data: {}", e);
                    return HttpResponse::build(StatusCode::BAD_REQUEST)
                        .json(NpmError::bad_request("Invalid tarball data"));
                }
            };

            // 存储 tarball
            let (file_path, integrity) = match state.storage.store_npm_tarball(
                Some(&scope),
                &name,
                version,
                &tarball_name,
                &data,
            ).await {
                Ok((path, integrity)) => {
                    info!("Stored npm tarball: {} ({} bytes)", path.display(), data.len());
                    (path.to_string_lossy().to_string(), integrity)
                }
                Err(e) => {
                    error!("Failed to store tarball: {}", e);
                    return HttpResponse::InternalServerError().finish();
                }
            };

            // 在数据库中创建包记录
            if let Some(ref client) = state.registry_client {
                // 获取第一个 dist-tag (通常是 latest)
                let dist_tag = publish_req.dist_tags.keys().next()
                    .map(|s| s.as_str())
                    .unwrap_or("latest");

                let create_req = crate::registry_client::CreateNpmPackageRequest {
                    project_id,
                    name: full_name.clone(),
                    version: version.clone(),
                    dist_tag: dist_tag.to_string(),
                    tarball_sha512: Some(integrity),
                    readme: None, // npm publish 请求中可能没有 readme
                    keywords: version_info.keywords.clone(),
                    license: version_info.license.clone(),
                    repository: version_info.repository.clone(),
                    dependencies: version_info.dependencies.clone(),
                    dev_dependencies: version_info.dev_dependencies.clone(),
                    peer_dependencies: version_info.peer_dependencies.clone(),
                    file_path,
                    file_size: data.len() as i64,
                };

                match client.create_npm_package(&create_req).await {
                    Ok(_pkg) => {
                        info!("Created npm package record: {}@{}", full_name, version);
                    }
                    Err(crate::registry_client::RegistryApiError::Conflict) => {
                        warn!("Package {}@{} already exists", full_name, version);
                        return HttpResponse::Conflict()
                            .json(NpmError::conflict(&format!("Version {} already exists", version)));
                    }
                    Err(e) => {
                        error!("Failed to create npm package record: {}", e);
                        return HttpResponse::InternalServerError().finish();
                    }
                }
            }
        }
    }

    // 处理 dist-tags
    if let Some(ref client) = state.registry_client {
        for (tag, version) in &publish_req.dist_tags {
            debug!("Setting dist-tag: {} -> {}", tag, version);
            
            if let Err(e) = client.update_npm_dist_tag(project_id, &full_name, tag, version).await {
                error!("Failed to update dist-tag {}: {}", tag, e);
                // 继续处理其他 dist-tags，不中断
            }
        }
    }

    HttpResponse::Ok().json(NpmPublishResponse {
        ok: true,
        id: Some(full_name),
        rev: Some("1-0".to_string()),
    })
}

/// GET /npm/@{scope}/{name}/-/{tarball} - 下载 tarball
pub async fn handle_tarball_get_scoped(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let (scope, name, tarball) = path.into_inner();
    debug!("npm tarball GET: @{}/{} -> {}", scope, name, tarball);

    // 从 tarball 名称解析版本
    // 格式: {name}-{version}.tgz
    let version = tarball
        .strip_prefix(&format!("{}-", name))
        .and_then(|s| s.strip_suffix(".tgz"))
        .unwrap_or("unknown");

    // 打开文件
    let file = match state.storage.open_npm_tarball(Some(&scope), &name, version, &tarball).await {
        Ok(f) => f,
        Err(_) => {
            return HttpResponse::build(StatusCode::NOT_FOUND)
                .json(NpmError::not_found(&tarball));
        }
    };

    // 获取大小
    let size = state.storage.npm_tarball_size(Some(&scope), &name, version, &tarball).await.unwrap_or(0);

    // 流式返回
    use tokio_util::io::ReaderStream;
    let stream = ReaderStream::new(file);

    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/octet-stream"))
        .insert_header(("Content-Length", size.to_string()))
        .streaming(stream)
}

/// DELETE /npm/@{scope}/{name}/-/{tarball}/-rev/{rev} - 删除版本
pub async fn handle_tarball_delete_scoped(
    req: HttpRequest,
    path: web::Path<(String, String, String, String)>,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let (scope, name, tarball, rev) = path.into_inner();
    let full_name = format!("@{}/{}", scope, name);
    debug!("npm tarball DELETE: @{}/{} -> {} (rev: {})", scope, name, tarball, rev);

    // 从 tarball 名称解析版本
    let version = tarball
        .strip_prefix(&format!("{}-", name))
        .and_then(|s| s.strip_suffix(".tgz"))
        .unwrap_or("unknown");

    // 解析项目ID
    let project_id = if let Some(ref client) = state.registry_client {
        match client.resolve_project(&scope, &name).await {
            Ok(resolved) => resolved.project_id,
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                return HttpResponse::build(StatusCode::NOT_FOUND)
                    .json(NpmError::not_found(&format!("Project not found for @{}/{}", scope, name)));
            }
            Err(e) => {
                error!("Failed to resolve project: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    } else {
        error!("Registry client not initialized");
        return HttpResponse::InternalServerError().finish();
    };

    // 从数据库删除记录
    if let Some(ref client) = state.registry_client {
        if let Err(e) = client.delete_npm_package(project_id, &full_name, version).await {
            match e {
                crate::registry_client::RegistryApiError::NotFound => {
                    warn!("Package {}@{} not found in database", full_name, version);
                    // 继续删除本地文件
                }
                _ => {
                    error!("Failed to delete npm package from database: {}", e);
                    return HttpResponse::InternalServerError().finish();
                }
            }
        } else {
            info!("Deleted npm package from database: {}@{}", full_name, version);
        }
    }

    // 删除本地文件
    match state.storage.delete_npm_tarball(Some(&scope), &name, version, &tarball).await {
        Ok(_) => {
            info!("Deleted npm tarball: @{}/{} version {}", scope, name, version);
            HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
        }
        Err(e) => {
            error!("Failed to delete tarball: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// GET /npm/-/package/@{scope}/{name}/dist-tags - 获取 dist-tags
pub async fn handle_dist_tags_get(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let (scope, name) = path.into_inner();
    let full_name = format!("@{}/{}", scope, name);
    debug!("npm dist-tags GET: @{}/{}", scope, name);

    // 解析项目ID
    let project_id = if let Some(ref client) = state.registry_client {
        match client.resolve_project(&scope, &name).await {
            Ok(resolved) => resolved.project_id,
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                return HttpResponse::build(StatusCode::NOT_FOUND)
                    .json(NpmError::not_found(&format!("Project not found for @{}/{}", scope, name)));
            }
            Err(e) => {
                error!("Failed to resolve project: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    } else {
        error!("Registry client not initialized");
        return HttpResponse::InternalServerError().finish();
    };

    // 从数据库查询 dist-tags
    if let Some(ref client) = state.registry_client {
        match client.get_npm_dist_tags(project_id, &full_name).await {
            Ok(response) => {
                return HttpResponse::Ok().json(response.tags);
            }
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                // 包不存在，返回空 tags
                let tags: HashMap<String, String> = HashMap::new();
                return HttpResponse::Ok().json(tags);
            }
            Err(e) => {
                error!("Failed to get dist-tags: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    }

    let tags: HashMap<String, String> = HashMap::new();
    HttpResponse::Ok().json(tags)
}

/// PUT /npm/-/package/@{scope}/{name}/dist-tags/{tag} - 设置 dist-tag
pub async fn handle_dist_tag_put(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    body: web::Bytes,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let (scope, name, tag) = path.into_inner();
    let full_name = format!("@{}/{}", scope, name);
    debug!("npm dist-tag PUT: @{}/{} -> {}", scope, name, tag);

    // 解析版本号（body 是 JSON 字符串形式的版本号）
    let version: String = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(NpmError::bad_request(&format!("Invalid version: {}", e)));
        }
    };

    // 解析项目ID
    let project_id = if let Some(ref client) = state.registry_client {
        match client.resolve_project(&scope, &name).await {
            Ok(resolved) => resolved.project_id,
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                return HttpResponse::build(StatusCode::NOT_FOUND)
                    .json(NpmError::not_found(&format!("Project not found for @{}/{}", scope, name)));
            }
            Err(e) => {
                error!("Failed to resolve project: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    } else {
        error!("Registry client not initialized");
        return HttpResponse::InternalServerError().finish();
    };

    // 在数据库中更新 dist-tag
    if let Some(ref client) = state.registry_client {
        if let Err(e) = client.update_npm_dist_tag(project_id, &full_name, &tag, &version).await {
            error!("Failed to update dist-tag: {}", e);
            return HttpResponse::InternalServerError().finish();
        }
    }

    info!("Set dist-tag @{}/{} {} -> {}", scope, name, tag, version);
    HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
}

/// DELETE /npm/-/package/@{scope}/{name}/dist-tags/{tag} - 删除 dist-tag
pub async fn handle_dist_tag_delete(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let (scope, name, tag) = path.into_inner();
    let full_name = format!("@{}/{}", scope, name);
    debug!("npm dist-tag DELETE: @{}/{} -> {}", scope, name, tag);

    // 不能删除 latest 标签
    if tag == "latest" {
        return HttpResponse::build(StatusCode::BAD_REQUEST)
            .json(NpmError::bad_request("Cannot delete 'latest' tag"));
    }

    // 解析项目ID
    let project_id = if let Some(ref client) = state.registry_client {
        match client.resolve_project(&scope, &name).await {
            Ok(resolved) => resolved.project_id,
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                return HttpResponse::build(StatusCode::NOT_FOUND)
                    .json(NpmError::not_found(&format!("Project not found for @{}/{}", scope, name)));
            }
            Err(e) => {
                error!("Failed to resolve project: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    } else {
        error!("Registry client not initialized");
        return HttpResponse::InternalServerError().finish();
    };

    // 从数据库删除 dist-tag
    if let Some(ref client) = state.registry_client {
        match client.delete_npm_dist_tag(project_id, &full_name, &tag).await {
            Ok(_) => {}
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                // dist-tag 不存在也视为成功
                warn!("Dist-tag {} not found for {}", tag, full_name);
            }
            Err(e) => {
                error!("Failed to delete dist-tag: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    }

    info!("Delete dist-tag @{}/{} -> {}", scope, name, tag);
    HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
}

/// GET /npm/-/v1/search - 搜索包
pub async fn handle_search(
    req: HttpRequest,
    query: web::Query<SearchQuery>,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    debug!("npm search: {:?}", query);

    let search_text = query.text.as_deref().unwrap_or("");
    let limit = query.size.unwrap_or(20).min(100);
    let offset = query.from.unwrap_or(0);

    // 调用后端搜索 API
    if let Some(ref client) = state.registry_client {
        match client.search_npm_packages(search_text, limit, offset).await {
            Ok(search_result) => {
                // 转换为 npm registry 标准格式
                let objects: Vec<serde_json::Value> = search_result.packages.iter().map(|pkg| {
                    serde_json::json!({
                        "package": {
                            "name": pkg.name,
                            "scope": pkg.scope,
                            "version": pkg.version,
                            "description": pkg.description,
                            "keywords": pkg.keywords,
                            "date": pkg.date,
                            "links": {
                                "npm": pkg.links.npm,
                                "homepage": pkg.links.homepage,
                                "repository": pkg.links.repository,
                                "bugs": pkg.links.bugs
                            },
                            "publisher": pkg.publisher.as_ref().map(|p| serde_json::json!({
                                "username": p.username,
                                "email": p.email
                            }))
                        },
                        "score": {
                            "final": 1.0,
                            "detail": {
                                "quality": 1.0,
                                "popularity": 1.0,
                                "maintenance": 1.0
                            }
                        },
                        "searchScore": 1.0
                    })
                }).collect();

                let response = SearchResponse {
                    objects,
                    total: search_result.total as i32,
                    time: chrono::Utc::now().to_rfc3339(),
                };

                return HttpResponse::Ok().json(response);
            }
            Err(e) => {
                error!("Failed to search npm packages: {}", e);
                // 返回空结果而不是错误
            }
        }
    }

    // 默认空响应
    let response = SearchResponse {
        objects: vec![],
        total: 0,
        time: chrono::Utc::now().to_rfc3339(),
    };

    HttpResponse::Ok().json(response)
}

#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub size: Option<i32>,
    pub from: Option<i32>,
}

#[derive(Debug, serde::Serialize)]
pub struct SearchResponse {
    pub objects: Vec<serde_json::Value>,
    pub total: i32,
    pub time: String,
}

/// PUT /npm/-/user/org.couchdb.user:{username} - 用户登录（生成 token）
pub async fn handle_user_login(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Bytes,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let username = path.into_inner();
    debug!("npm user login: {}", username);

    // 解析登录请求
    #[derive(Debug, serde::Deserialize)]
    struct LoginRequest {
        name: String,
        password: String,
    }

    let login_req: LoginRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(NpmError::bad_request(&format!("Invalid login request: {}", e)));
        }
    };

    // 调用后端认证服务验证用户并生成 token
    if let Some(ref client) = state.registry_client {
        match client.npm_login(&login_req.name, &login_req.password).await {
            Ok(response) => {
                info!("npm login successful for user: {}", response.username);
                return HttpResponse::Created().json(serde_json::json!({
                    "ok": response.ok,
                    "id": format!("org.couchdb.user:{}", response.username),
                    "token": response.token
                }));
            }
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                warn!("npm login failed: invalid credentials for {}", login_req.name);
                return HttpResponse::build(StatusCode::UNAUTHORIZED)
                    .json(NpmError::unauthorized());
            }
            Err(e) => {
                error!("npm login error: {}", e);
                return HttpResponse::InternalServerError().finish();
            }
        }
    }

    // 无法连接后端服务
    error!("Registry client not initialized");
    HttpResponse::InternalServerError().finish()
}

/// GET /npm/-/whoami - 获取当前用户
pub async fn handle_whoami(
    req: HttpRequest,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    debug!("npm whoami");

    // 获取 Authorization header
    let auth_header = match req.headers().get("Authorization") {
        Some(h) => h,
        None => {
            return HttpResponse::build(StatusCode::UNAUTHORIZED)
                .json(NpmError::unauthorized());
        }
    };

    let auth_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => {
            return HttpResponse::build(StatusCode::UNAUTHORIZED)
                .json(NpmError::unauthorized());
        }
    };

    // 支持 Bearer token 格式
    let token = if auth_str.starts_with("Bearer ") {
        auth_str.strip_prefix("Bearer ").unwrap()
    } else {
        // 也尝试支持基本的 token 格式 (npm 旧版本)
        auth_str
    };

    // 调用后端服务验证 token
    if let Some(ref client) = state.registry_client {
        match client.npm_whoami(token).await {
            Ok(response) => {
                return HttpResponse::Ok().json(serde_json::json!({
                    "username": response.username
                }));
            }
            Err(crate::registry_client::RegistryApiError::NotFound) => {
                return HttpResponse::build(StatusCode::UNAUTHORIZED)
                    .json(NpmError::unauthorized());
            }
            Err(e) => {
                error!("npm whoami error: {}", e);
                return HttpResponse::build(StatusCode::UNAUTHORIZED)
                    .json(NpmError::unauthorized());
            }
        }
    }

    HttpResponse::build(StatusCode::UNAUTHORIZED)
        .json(NpmError::unauthorized())
}

/// GET /npm/-/ping - 健康检查
pub async fn handle_ping(state: web::Data<NpmRegistryState>) -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({}))
}

// ============================================================================
// 路由配置
// ============================================================================

/// 配置 npm Registry 路由
pub fn configure_npm_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // 健康检查
        .route("/npm/-/ping", web::get().to(handle_ping))
        // 用户相关
        .route("/npm/-/whoami", web::get().to(handle_whoami))
        .route("/npm/-/user/org.couchdb.user:{username}", web::put().to(handle_user_login))
        // 搜索
        .route("/npm/-/v1/search", web::get().to(handle_search))
        // dist-tags（必须在包路由之前）
        .route("/npm/-/package/@{scope}/{name}/dist-tags", web::get().to(handle_dist_tags_get))
        .route("/npm/-/package/@{scope}/{name}/dist-tags/{tag}", web::get().to(handle_dist_tags_get))
        .route("/npm/-/package/@{scope}/{name}/dist-tags/{tag}", web::put().to(handle_dist_tag_put))
        .route("/npm/-/package/@{scope}/{name}/dist-tags/{tag}", web::delete().to(handle_dist_tag_delete))
        // Scoped 包操作
        .route("/npm/@{scope}/{name}", web::get().to(handle_package_get_scoped))
        .route("/npm/@{scope}/{name}", web::put().to(handle_package_publish_scoped))
        // Tarball 下载和删除
        .route("/npm/@{scope}/{name}/-/{tarball}", web::get().to(handle_tarball_get_scoped))
        .route("/npm/@{scope}/{name}/-/{tarball}/-rev/{rev}", web::delete().to(handle_tarball_delete_scoped))
        // 非 scoped 包（返回错误提示）
        .route("/npm/{name}", web::get().to(handle_package_get));
}
