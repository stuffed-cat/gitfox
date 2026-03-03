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

/// npm Registry 状态
pub struct NpmRegistryState {
    pub storage: RegistryStorage,
    pub config: Arc<RegistryConfig>,
    pub auth_client: Option<tokio::sync::Mutex<AuthClient>>,
    pub shell_secret: String,
    pub base_url: String,
}

impl NpmRegistryState {
    pub fn new(config: Arc<RegistryConfig>, shell_secret: String, base_url: String) -> Self {
        let storage = RegistryStorage::new(&config.storage_path);
        Self {
            storage,
            config,
            auth_client: None,
            shell_secret,
            base_url,
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
pub async fn handle_package_get_scoped(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    let (scope, name) = path.into_inner();
    let full_name = format!("@{}/{}", scope, name);
    debug!("npm package GET: {}", full_name);

    // TODO: 从数据库查询包信息
    // 这里返回示例结构
    
    HttpResponse::build(StatusCode::NOT_FOUND)
        .json(NpmError::not_found(&full_name))
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
            match state.storage.store_npm_tarball(
                Some(&scope),
                &name,
                version,
                &tarball_name,
                &data,
            ).await {
                Ok((path, integrity)) => {
                    info!("Stored npm tarball: {} ({} bytes)", path.display(), data.len());
                    // TODO: 在数据库中创建记录
                }
                Err(e) => {
                    error!("Failed to store tarball: {}", e);
                    return HttpResponse::InternalServerError().finish();
                }
            }
        }
    }

    // 处理 dist-tags
    for (tag, version) in &publish_req.dist_tags {
        debug!("Setting dist-tag: {} -> {}", tag, version);
        // TODO: 在数据库中更新 dist-tag
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
    debug!("npm tarball DELETE: @{}/{} -> {} (rev: {})", scope, name, tarball, rev);

    // 从 tarball 名称解析版本
    let version = tarball
        .strip_prefix(&format!("{}-", name))
        .and_then(|s| s.strip_suffix(".tgz"))
        .unwrap_or("unknown");

    // 删除文件
    match state.storage.delete_npm_tarball(Some(&scope), &name, version, &tarball).await {
        Ok(_) => {
            // TODO: 从数据库删除记录
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
    debug!("npm dist-tags GET: @{}/{}", scope, name);

    // TODO: 从数据库查询 dist-tags
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
    debug!("npm dist-tag PUT: @{}/{} -> {}", scope, name, tag);

    // 解析版本号（body 是 JSON 字符串形式的版本号）
    let version: String = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(NpmError::bad_request(&format!("Invalid version: {}", e)));
        }
    };

    // TODO: 在数据库中更新 dist-tag
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
    debug!("npm dist-tag DELETE: @{}/{} -> {}", scope, name, tag);

    // 不能删除 latest 标签
    if tag == "latest" {
        return HttpResponse::build(StatusCode::BAD_REQUEST)
            .json(NpmError::bad_request("Cannot delete 'latest' tag"));
    }

    // TODO: 从数据库删除 dist-tag
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

    // TODO: 实现搜索功能
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

    // TODO: 验证用户名密码，生成 token
    // 这里应该调用主应用的认证服务

    // 返回 token（简化处理）
    HttpResponse::Created().json(serde_json::json!({
        "ok": true,
        "id": format!("org.couchdb.user:{}", login_req.name),
        "token": "npm_token_placeholder"
    }))
}

/// GET /npm/-/whoami - 获取当前用户
pub async fn handle_whoami(
    req: HttpRequest,
    state: web::Data<NpmRegistryState>,
) -> HttpResponse {
    debug!("npm whoami");

    // TODO: 从 token 解析用户信息
    // 这里返回未认证错误
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
