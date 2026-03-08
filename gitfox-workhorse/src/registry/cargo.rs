//! Cargo Registry API 处理器
//!
//! 实现 Cargo Registry API 规范 (RFC 2789 Sparse Protocol + Alternate Registries)
//! 
//! ## 协议参考
//! - Cargo Registries: https://doc.rust-lang.org/cargo/reference/registries.html
//! - Registry Index: https://doc.rust-lang.org/cargo/reference/registry-index.html
//! - Registry Web API: https://doc.rust-lang.org/cargo/reference/registry-web-api.html
//! - Sparse Protocol (RFC 2789): https://rust-lang.github.io/rfcs/2789-sparse-index.html
//!
//! ## 路由结构
//!
//! **Index API** (Sparse Protocol):
//! - `GET /index/config.json` - 注册表配置
//! - `GET /index/{prefix}/{crate}` - 获取 crate 索引条目
//!
//! **Web API**:
//! - `PUT /api/v1/crates/new` - 发布 crate
//! - `DELETE /api/v1/crates/{name}/{version}/yank` - 撤回版本
//! - `PUT /api/v1/crates/{name}/{version}/unyank` - 取消撤回
//! - `GET /api/v1/crates/{name}/owners` - 列出所有者
//! - `PUT /api/v1/crates/{name}/owners` - 添加所有者
//! - `DELETE /api/v1/crates/{name}/owners` - 移除所有者
//! - `GET /api/v1/crates/{name}` - 获取 crate 元数据
//! - `GET /api/v1/crates` - 搜索 crates
//!
//! **Download**:
//! - `GET /api/v1/crates/{name}/{version}/download` - 下载 crate

use actix_web::{web, HttpRequest, HttpResponse, http::StatusCode};
use tracing::{debug, info, warn, error};
use std::sync::Arc;
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use semver::Version;

use super::storage::{RegistryStorage, StorageError};
use super::config::RegistryConfig;
use crate::registry_client::RegistryApiClient;

/// Cargo Registry 状态
pub struct CargoRegistryState {
    pub storage: RegistryStorage,
    pub config: Arc<RegistryConfig>,
    pub registry_client: Option<RegistryApiClient>,
    pub shell_secret: String,
    pub base_url: String,
    pub backend_url: String,
}

impl CargoRegistryState {
    pub fn new(
        config: Arc<RegistryConfig>,
        shell_secret: String,
        base_url: String,
        backend_url: String,
    ) -> Self {
        let storage = RegistryStorage::new(&config.storage_path);
        let registry_client = RegistryApiClient::new(&backend_url, &shell_secret);
        Self {
            storage,
            config,
            registry_client: Some(registry_client),
            shell_secret,
            base_url,
            backend_url,
        }
    }

    /// 初始化存储
    pub async fn init(&self) -> std::io::Result<()> {
        self.storage.init().await?;
        // 创建 cargo 子目录
        tokio::fs::create_dir_all(self.storage.cargo_path()).await?;
        Ok(())
    }

    /// 生成 crate 下载 URL
    fn download_url(&self, namespace: &str, name: &str, version: &str) -> String {
        format!(
            "{}/api/v1/crates/{}/{}/download",
            self.registry_url(namespace),
            name,
            version
        )
    }

    /// 生成 API URL (Cargo 会自动追加 /api/v1/... 路径)
    fn api_url(&self, namespace: &str) -> String {
        self.registry_url(namespace)
    }

    /// 获取注册表 URL
    fn registry_url(&self, namespace: &str) -> String {
        if let Some(ref domain) = self.config.domain {
            format!("https://{}/cargo/{}", domain, namespace)
        } else {
            format!("{}/cargo/{}", self.base_url, namespace)
        }
    }
}

// ============================================================================
// Cargo Registry 类型定义
// ============================================================================

/// Cargo Registry 配置 (config.json)
#[derive(Debug, Clone, serde::Serialize)]
pub struct CargoRegistryConfig {
    /// 下载 URL 模板
    pub dl: String,
    /// API 端点 URL
    pub api: String,
    /// 认证令牌所需的操作
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_required: Option<bool>,
}

/// Cargo Index 条目 (每行一个 JSON 对象)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CargoIndexEntry {
    /// crate 名称
    pub name: String,
    /// 版本号
    pub vers: String,
    /// 依赖列表
    pub deps: Vec<CargoIndexDependency>,
    /// SHA256 校验和
    pub cksum: String,
    /// 特性映射
    pub features: HashMap<String, Vec<String>>,
    /// 是否已撤回
    #[serde(default)]
    pub yanked: bool,
    /// links 字段 (用于 build scripts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<String>,
    /// 最低 Rust 版本要求
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_version: Option<String>,
    /// features2 (用于隐式可选依赖特性, Cargo 1.60+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features2: Option<HashMap<String, Vec<String>>>,
    /// v 字段 (索引版本, 默认 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<i32>,
}

/// Cargo 依赖定义
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CargoIndexDependency {
    /// 依赖名称 (crates.io 上的名称)
    pub name: String,
    /// 版本要求
    pub req: String,
    /// 启用的特性
    pub features: Vec<String>,
    /// 是否可选
    pub optional: bool,
    /// 是否使用默认特性
    pub default_features: bool,
    /// 目标平台
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// 依赖类型 (normal, dev, build)
    pub kind: String,
    /// 依赖的注册表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// 包名称 (如果与依赖名不同)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

/// Cargo 发布请求元数据
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CargoPublishMetadata {
    /// crate 名称
    pub name: String,
    /// 版本号
    pub vers: String,
    /// 依赖列表
    pub deps: Vec<CargoPublishDependency>,
    /// 特性映射
    pub features: HashMap<String, Vec<String>>,
    /// 作者列表
    #[serde(default)]
    pub authors: Vec<String>,
    /// 描述
    #[serde(default)]
    pub description: Option<String>,
    /// 文档 URL
    #[serde(default)]
    pub documentation: Option<String>,
    /// 主页 URL
    #[serde(default)]
    pub homepage: Option<String>,
    /// README 内容
    #[serde(default)]
    pub readme: Option<String>,
    /// README 文件名
    #[serde(default)]
    pub readme_file: Option<String>,
    /// 关键词
    #[serde(default)]
    pub keywords: Vec<String>,
    /// 分类
    #[serde(default)]
    pub categories: Vec<String>,
    /// 许可证
    #[serde(default)]
    pub license: Option<String>,
    /// 许可证文件
    #[serde(default)]
    pub license_file: Option<String>,
    /// 仓库 URL
    #[serde(default)]
    pub repository: Option<String>,
    /// 徽章
    #[serde(default)]
    pub badges: HashMap<String, HashMap<String, String>>,
    /// links 字段
    #[serde(default)]
    pub links: Option<String>,
    /// 最低 Rust 版本
    #[serde(default)]
    pub rust_version: Option<String>,
}

/// 发布请求中的依赖
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CargoPublishDependency {
    pub name: String,
    pub version_req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    #[serde(default)]
    pub target: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub registry: Option<String>,
    #[serde(default)]
    pub explicit_name_in_toml: Option<String>,
}

/// 发布成功响应
#[derive(Debug, serde::Serialize)]
pub struct CargoPublishResponse {
    /// 警告信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<CargoWarnings>,
}

#[derive(Debug, serde::Serialize)]
pub struct CargoWarnings {
    /// 无效的徽章
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub invalid_badges: Vec<String>,
    /// 无效的分类
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub invalid_categories: Vec<String>,
    /// 其他警告
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other: Vec<String>,
}

/// Yank/Unyank 响应
#[derive(Debug, serde::Serialize)]
pub struct CargoYankResponse {
    pub ok: bool,
}

/// 所有者列表响应
#[derive(Debug, serde::Serialize)]
pub struct CargoOwnersResponse {
    pub users: Vec<CargoOwner>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CargoOwner {
    pub id: i64,
    pub login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 添加/移除所有者请求
#[derive(Debug, serde::Deserialize)]
pub struct CargoOwnerModifyRequest {
    pub users: Vec<String>,
}

/// 搜索响应
#[derive(Debug, serde::Serialize)]
pub struct CargoSearchResponse {
    pub crates: Vec<CargoCrateSummary>,
    pub meta: CargoSearchMeta,
}

#[derive(Debug, serde::Serialize)]
pub struct CargoCrateSummary {
    pub name: String,
    pub max_version: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    pub downloads: i64,
    pub recent_downloads: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct CargoSearchMeta {
    pub total: i64,
}

/// Cargo 错误响应
#[derive(Debug, serde::Serialize)]
pub struct CargoError {
    pub errors: Vec<CargoErrorDetail>,
}

#[derive(Debug, serde::Serialize)]
pub struct CargoErrorDetail {
    pub detail: String,
}

impl CargoError {
    pub fn new(detail: &str) -> Self {
        Self {
            errors: vec![CargoErrorDetail {
                detail: detail.to_string(),
            }],
        }
    }

    pub fn unauthorized() -> Self {
        Self::new("must be logged in to perform that action")
    }

    pub fn forbidden() -> Self {
        Self::new("must be an owner to perform that action")
    }

    pub fn not_found(name: &str) -> Self {
        Self::new(&format!("crate `{}` does not exist", name))
    }

    pub fn version_not_found(name: &str, version: &str) -> Self {
        Self::new(&format!("crate `{}` does not have a version `{}`", name, version))
    }

    pub fn version_exists(name: &str, version: &str) -> Self {
        Self::new(&format!("crate `{}` version `{}` already exists", name, version))
    }

    pub fn invalid_name(name: &str, reason: &str) -> Self {
        Self::new(&format!("invalid crate name `{}`: {}", name, reason))
    }

    pub fn invalid_version(version: &str, reason: &str) -> Self {
        Self::new(&format!("invalid version `{}`: {}", version, reason))
    }

    pub fn internal_error(reason: &str) -> Self {
        Self::new(&format!("internal error: {}", reason))
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 计算 crate 索引文件路径前缀
/// 
/// 根据 crate 名称长度确定路径：
/// - 1 字符: `1/{name}`
/// - 2 字符: `2/{name}`
/// - 3 字符: `3/{first_char}/{name}`
/// - 4+ 字符: `{first_two}/{second_two}/{name}`
pub fn crate_index_path(name: &str) -> String {
    let name_lower = name.to_lowercase();
    match name_lower.len() {
        1 => format!("1/{}", name_lower),
        2 => format!("2/{}", name_lower),
        3 => format!("3/{}/{}", &name_lower[..1], name_lower),
        _ => format!("{}/{}/{}", &name_lower[..2], &name_lower[2..4], name_lower),
    }
}

/// 验证 crate 名称
/// 
/// crate 名称必须：
/// - 以字母开头
/// - 只包含字母、数字、`-` 和 `_`
/// - 长度在 1-64 字符之间
/// - 不能是保留名称
pub fn validate_crate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name cannot be empty".to_string());
    }
    if name.len() > 64 {
        return Err("name cannot exceed 64 characters".to_string());
    }
    
    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() {
        return Err("name must start with a letter".to_string());
    }
    
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            return Err(format!("invalid character '{}' in name", c));
        }
    }
    
    // 保留名称
    let reserved = ["std", "alloc", "core", "test", "proc_macro", "proc-macro"];
    if reserved.contains(&name.to_lowercase().as_str()) {
        return Err(format!("name '{}' is reserved", name));
    }
    
    Ok(())
}

/// 验证版本号
pub fn validate_version(version: &str) -> Result<Version, String> {
    Version::parse(version)
        .map_err(|e| format!("invalid semver: {}", e))
}

/// 从 Authorization header 提取 token
pub fn extract_token(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| {
            // 也支持老式 token 格式
            req.headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
        })
        .map(|s| s.to_string())
}

// ============================================================================
// API 端点处理器
// ============================================================================

/// GET /cargo/{namespace}/index/config.json
/// 
/// 返回注册表配置
pub async fn handle_config(
    path: web::Path<String>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let namespace = path.into_inner();
    debug!("Cargo config request for namespace: {}", namespace);

    let config = CargoRegistryConfig {
        dl: format!("{}/{{crate}}/{{version}}/download", state.api_url(&namespace)),
        api: state.api_url(&namespace),
        auth_required: Some(true),
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "application/json"))
        .insert_header(("Cache-Control", "max-age=60"))
        .json(config)
}

/// GET /cargo/{namespace}/index/{prefix}/{prefix2}/{crate} 
/// GET /cargo/{namespace}/index/{prefix}/{crate} (for 1, 2, 3 char names)
/// 
/// Sparse Protocol: 返回 crate 的索引条目
pub async fn handle_index_entry(
    req: HttpRequest,
    path: web::Path<String>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    // 路径格式: {namespace}/index/.../{crate_name}
    let full_path = path.into_inner();
    let parts: Vec<&str> = full_path.split('/').collect();
    
    if parts.len() < 3 {
        return HttpResponse::NotFound()
            .json(CargoError::new("invalid index path"));
    }
    
    let namespace = parts[0];
    let crate_name = parts.last().unwrap();
    
    debug!("Cargo index request: namespace={}, crate={}", namespace, crate_name);
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            error!("Registry client not initialized");
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    // 从后端获取 crate 所有版本的索引条目
    match client.get_cargo_index(namespace, crate_name).await {
        Ok(entries) => {
            if entries.is_empty() {
                return HttpResponse::NotFound()
                    .json(CargoError::not_found(crate_name));
            }
            
            // 每行一个 JSON 对象
            let body: String = entries
                .iter()
                .map(|e| serde_json::to_string(e).unwrap_or_default())
                .collect::<Vec<_>>()
                .join("\n");
            
            HttpResponse::Ok()
                .insert_header(("Content-Type", "application/json"))
                .insert_header(("Cache-Control", "max-age=10"))
                .body(body)
        }
        Err(e) => {
            warn!("Failed to get cargo index for {}/{}: {:?}", namespace, crate_name, e);
            HttpResponse::NotFound()
                .json(CargoError::not_found(crate_name))
        }
    }
}

/// PUT /cargo/{namespace}/api/v1/crates/new
/// 
/// 发布新 crate 版本
pub async fn handle_publish(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Bytes,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let namespace = path.into_inner();
    debug!("Cargo publish request for namespace: {}", namespace);
    
    // 验证认证
    let token = match extract_token(&req) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(CargoError::unauthorized());
        }
    };
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            error!("Registry client not initialized");
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    // 验证 token 并获取用户信息
    let user_info = match client.verify_cargo_token(&token).await {
        Ok(info) => info,
        Err(_) => {
            return HttpResponse::Unauthorized()
                .json(CargoError::unauthorized());
        }
    };
    
    // 解析请求体
    // 格式: [4字节 metadata长度][metadata JSON][4字节 crate长度][crate 数据]
    if body.len() < 8 {
        return HttpResponse::BadRequest()
            .json(CargoError::new("invalid request body"));
    }
    
    let metadata_len = u32::from_le_bytes([body[0], body[1], body[2], body[3]]) as usize;
    if body.len() < 4 + metadata_len + 4 {
        return HttpResponse::BadRequest()
            .json(CargoError::new("invalid request body"));
    }
    
    let metadata_bytes = &body[4..4 + metadata_len];
    let metadata: CargoPublishMetadata = match serde_json::from_slice(metadata_bytes) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(CargoError::new(&format!("invalid metadata: {}", e)));
        }
    };
    
    let crate_len_offset = 4 + metadata_len;
    let crate_len = u32::from_le_bytes([
        body[crate_len_offset],
        body[crate_len_offset + 1],
        body[crate_len_offset + 2],
        body[crate_len_offset + 3],
    ]) as usize;
    
    let crate_data_offset = crate_len_offset + 4;
    if body.len() < crate_data_offset + crate_len {
        return HttpResponse::BadRequest()
            .json(CargoError::new("crate data truncated"));
    }
    
    let crate_data = &body[crate_data_offset..crate_data_offset + crate_len];
    
    // 验证 crate 名称
    if let Err(e) = validate_crate_name(&metadata.name) {
        return HttpResponse::BadRequest()
            .json(CargoError::invalid_name(&metadata.name, &e));
    }
    
    // 验证版本号
    if let Err(e) = validate_version(&metadata.vers) {
        return HttpResponse::BadRequest()
            .json(CargoError::invalid_version(&metadata.vers, &e));
    }
    
    // 计算 SHA256 校验和
    let mut hasher = Sha256::new();
    hasher.update(crate_data);
    let cksum = format!("{:x}", hasher.finalize());
    
    // 存储 crate 文件
    let (crate_path, _) = match state.storage.store_cargo_crate(&namespace, &metadata.name, &metadata.vers, crate_data).await {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to store crate file: {}", e);
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to store crate"));
        }
    };
    
    // 转换依赖格式（使用 registry_client 的类型）
    let deps: Vec<crate::registry_client::CargoIndexDependency> = metadata.deps.iter().map(|d| {
        crate::registry_client::CargoIndexDependency {
            name: d.explicit_name_in_toml.clone().unwrap_or_else(|| d.name.clone()),
            req: d.version_req.clone(),
            features: d.features.clone(),
            optional: d.optional,
            default_features: d.default_features,
            target: d.target.clone(),
            kind: d.kind.clone(),
            registry: d.registry.clone(),
            package: if d.explicit_name_in_toml.is_some() { Some(d.name.clone()) } else { None },
        }
    }).collect();
    
    // 调用后端 API 创建 crate 记录
    let create_request = crate::registry_client::CreateCargoPackageRequest {
        namespace: namespace.clone(),
        name: metadata.name.clone(),
        version: metadata.vers.clone(),
        user_id: user_info.user_id,
        deps,
        features: metadata.features.clone(),
        cksum: cksum.clone(),
        description: metadata.description.clone(),
        documentation: metadata.documentation.clone(),
        homepage: metadata.homepage.clone(),
        repository: metadata.repository.clone(),
        readme: metadata.readme.clone(),
        readme_file: metadata.readme_file.clone(),
        license: metadata.license.clone(),
        license_file: metadata.license_file.clone(),
        keywords: metadata.keywords.clone(),
        categories: metadata.categories.clone(),
        authors: metadata.authors.clone(),
        links: metadata.links.clone(),
        rust_version: metadata.rust_version.clone(),
        file_path: crate_path.to_string_lossy().to_string(),
        file_size: crate_len as i64,
    };
    
    match client.create_cargo_package(&create_request).await {
        Ok(_) => {
            info!("Published crate: {}@{} to {}", metadata.name, metadata.vers, namespace);
            
            HttpResponse::Ok().json(CargoPublishResponse {
                warnings: Some(CargoWarnings {
                    invalid_badges: vec![],
                    invalid_categories: vec![],
                    other: vec![],
                }),
            })
        }
        Err(crate::registry_client::RegistryApiError::Conflict) => {
            HttpResponse::Conflict()
                .json(CargoError::version_exists(&metadata.name, &metadata.vers))
        }
        Err(e) => {
            error!("Failed to create cargo package: {:?}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to publish crate"))
        }
    }
}

/// DELETE /cargo/{namespace}/api/v1/crates/{name}/{version}/yank
/// 
/// 撤回 crate 版本
pub async fn handle_yank(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let (namespace, name, version) = path.into_inner();
    debug!("Cargo yank request: {}/{}@{}", namespace, name, version);
    
    let token = match extract_token(&req) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(CargoError::unauthorized());
        }
    };
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    match client.yank_cargo_crate(&namespace, &name, &version, &token, true).await {
        Ok(_) => {
            info!("Yanked crate: {}/{}@{}", namespace, name, version);
            HttpResponse::Ok().json(CargoYankResponse { ok: true })
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::NotFound()
                .json(CargoError::version_not_found(&name, &version))
        }
        Err(crate::registry_client::RegistryApiError::Unauthorized) => {
            HttpResponse::Forbidden()
                .json(CargoError::forbidden())
        }
        Err(e) => {
            error!("Failed to yank crate: {:?}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to yank crate"))
        }
    }
}

/// PUT /cargo/{namespace}/api/v1/crates/{name}/{version}/unyank
/// 
/// 取消撤回 crate 版本
pub async fn handle_unyank(
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let (namespace, name, version) = path.into_inner();
    debug!("Cargo unyank request: {}/{}@{}", namespace, name, version);
    
    let token = match extract_token(&req) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(CargoError::unauthorized());
        }
    };
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    match client.yank_cargo_crate(&namespace, &name, &version, &token, false).await {
        Ok(_) => {
            info!("Unyanked crate: {}/{}@{}", namespace, name, version);
            HttpResponse::Ok().json(CargoYankResponse { ok: true })
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::NotFound()
                .json(CargoError::version_not_found(&name, &version))
        }
        Err(crate::registry_client::RegistryApiError::Unauthorized) => {
            HttpResponse::Forbidden()
                .json(CargoError::forbidden())
        }
        Err(e) => {
            error!("Failed to unyank crate: {:?}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to unyank crate"))
        }
    }
}

/// GET /cargo/{namespace}/api/v1/crates/{name}/owners
/// 
/// 列出 crate 所有者
pub async fn handle_owners_list(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let (namespace, name) = path.into_inner();
    debug!("Cargo owners list request: {}/{}", namespace, name);
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    match client.get_cargo_owners(&namespace, &name).await {
        Ok(owners) => {
            // 转换类型
            let users: Vec<CargoOwner> = owners.into_iter().map(|o| CargoOwner {
                id: o.id,
                login: o.login,
                name: o.name,
            }).collect();
            HttpResponse::Ok().json(CargoOwnersResponse { users })
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::NotFound()
                .json(CargoError::not_found(&name))
        }
        Err(e) => {
            error!("Failed to get crate owners: {:?}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to get owners"))
        }
    }
}

/// PUT /cargo/{namespace}/api/v1/crates/{name}/owners
/// 
/// 添加 crate 所有者
pub async fn handle_owners_add(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<CargoOwnerModifyRequest>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let (namespace, name) = path.into_inner();
    debug!("Cargo add owners request: {}/{}", namespace, name);
    
    let token = match extract_token(&req) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(CargoError::unauthorized());
        }
    };
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    match client.modify_cargo_owners(&namespace, &name, &body.users, &token, true).await {
        Ok(_) => {
            HttpResponse::Ok().json(serde_json::json!({
                "ok": true,
                "msg": "user added as owner"
            }))
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::NotFound()
                .json(CargoError::not_found(&name))
        }
        Err(crate::registry_client::RegistryApiError::Unauthorized) => {
            HttpResponse::Forbidden()
                .json(CargoError::forbidden())
        }
        Err(e) => {
            error!("Failed to add crate owners: {:?}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to add owners"))
        }
    }
}

/// DELETE /cargo/{namespace}/api/v1/crates/{name}/owners
/// 
/// 移除 crate 所有者
pub async fn handle_owners_remove(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<CargoOwnerModifyRequest>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let (namespace, name) = path.into_inner();
    debug!("Cargo remove owners request: {}/{}", namespace, name);
    
    let token = match extract_token(&req) {
        Some(t) => t,
        None => {
            return HttpResponse::Unauthorized()
                .json(CargoError::unauthorized());
        }
    };
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    match client.modify_cargo_owners(&namespace, &name, &body.users, &token, false).await {
        Ok(_) => {
            HttpResponse::Ok().json(serde_json::json!({
                "ok": true,
                "msg": "user removed as owner"
            }))
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::NotFound()
                .json(CargoError::not_found(&name))
        }
        Err(crate::registry_client::RegistryApiError::Unauthorized) => {
            HttpResponse::Forbidden()
                .json(CargoError::forbidden())
        }
        Err(e) => {
            error!("Failed to remove crate owners: {:?}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to remove owners"))
        }
    }
}

/// GET /cargo/{namespace}/api/v1/crates/{name}/{version}/download
/// 
/// 下载 crate
pub async fn handle_download(
    path: web::Path<(String, String, String)>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    use tokio::io::AsyncReadExt;
    
    let (namespace, name, version) = path.into_inner();
    debug!("Cargo download request: {}/{}@{}", namespace, name, version);
    
    // 读取 crate 文件
    match state.storage.open_cargo_crate(&namespace, &name, &version).await {
        Ok(mut file) => {
            let mut data = Vec::new();
            if let Err(e) = file.read_to_end(&mut data).await {
                error!("Failed to read crate file: {}", e);
                return HttpResponse::InternalServerError()
                    .json(CargoError::internal_error("failed to read crate"));
            }
            
            // 记录下载统计（异步，不阻塞响应）
            if let Some(client) = state.registry_client.as_ref() {
                let client = client.clone();
                let ns = namespace.clone();
                let n = name.clone();
                let v = version.clone();
                tokio::spawn(async move {
                    let _ = client.record_cargo_download(&ns, &n, &v).await;
                });
            }
            
            HttpResponse::Ok()
                .insert_header(("Content-Type", "application/x-tar"))
                .insert_header(("Content-Disposition", format!(
                    "attachment; filename=\"{}-{}.crate\"",
                    name, version
                )))
                .body(data)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::NotFound()
                .json(CargoError::version_not_found(&name, &version))
        }
        Err(e) => {
            error!("Failed to read crate file: {}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to read crate"))
        }
    }
}

/// GET /cargo/{namespace}/api/v1/crates
/// 
/// 搜索 crates
pub async fn handle_search(
    path: web::Path<String>,
    query: web::Query<CargoSearchQuery>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let namespace = path.into_inner();
    debug!("Cargo search request: namespace={}, q={:?}", namespace, query.q);
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    let per_page = query.per_page.unwrap_or(10).min(100);
    
    match client.search_cargo_crates(
        &namespace,
        query.q.as_deref().unwrap_or(""),
        per_page,
    ).await {
        Ok(results) => {
            // 转换类型
            let crates: Vec<CargoCrateSummary> = results.crates.into_iter().map(|c| CargoCrateSummary {
                name: c.name,
                max_version: c.max_version,
                description: c.description,
                homepage: c.homepage,
                repository: c.repository,
                documentation: c.documentation,
                downloads: c.downloads,
                recent_downloads: c.recent_downloads,
            }).collect();
            
            HttpResponse::Ok().json(CargoSearchResponse {
                crates,
                meta: CargoSearchMeta {
                    total: results.total,
                },
            })
        }
        Err(e) => {
            error!("Failed to search crates: {:?}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("search failed"))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct CargoSearchQuery {
    pub q: Option<String>,
    pub per_page: Option<i32>,
}

/// GET /cargo/{namespace}/api/v1/crates/{name}
/// 
/// 获取 crate 元数据
pub async fn handle_crate_info(
    path: web::Path<(String, String)>,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    let (namespace, name) = path.into_inner();
    debug!("Cargo crate info request: {}/{}", namespace, name);
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    match client.get_cargo_crate_info(&namespace, &name).await {
        Ok(info) => {
            // 转换为前端期望的格式: { crate: {...}, versions: [...] }
            HttpResponse::Ok().json(serde_json::json!({
                "crate": {
                    "name": info.name,
                    "description": info.description,
                    "homepage": info.homepage,
                    "documentation": info.documentation,
                    "repository": info.repository,
                    "downloads": info.downloads,
                    "recent_downloads": 0,
                    "max_version": info.max_version,
                    "created_at": info.versions.last().map(|v| v.created_at.to_rfc3339()).unwrap_or_default(),
                    "updated_at": info.versions.first().map(|v| v.created_at.to_rfc3339()).unwrap_or_default(),
                },
                "versions": info.versions.iter().map(|v| serde_json::json!({
                    "num": v.version,
                    "yanked": v.yanked,
                    "created_at": v.created_at.to_rfc3339(),
                    "dl_path": format!("/cargo/{}/api/v1/crates/{}/{}/download", namespace, name, v.version),
                    "crate_size": serde_json::Value::Null,
                })).collect::<Vec<_>>()
            }))
        }
        Err(crate::registry_client::RegistryApiError::NotFound) => {
            HttpResponse::NotFound()
                .json(CargoError::not_found(&name))
        }
        Err(e) => {
            error!("Failed to get crate info: {:?}", e);
            HttpResponse::InternalServerError()
                .json(CargoError::internal_error("failed to get crate info"))
        }
    }
}

/// POST /cargo/{namespace}/me
/// 
/// Cargo login token 验证端点
pub async fn handle_login(
    body: web::Bytes,
    state: web::Data<CargoRegistryState>,
) -> HttpResponse {
    // cargo login 发送的是纯文本 token
    let token = match String::from_utf8(body.to_vec()) {
        Ok(t) => t.trim().to_string(),
        Err(_) => {
            return HttpResponse::BadRequest()
                .json(CargoError::new("invalid token format"));
        }
    };
    
    let client = match state.registry_client.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError()
                .json(CargoError::internal_error("registry client not initialized"));
        }
    };
    
    match client.verify_cargo_token(&token).await {
        Ok(info) => {
            HttpResponse::Ok().json(serde_json::json!({
                "ok": true,
                "user": {
                    "id": info.user_id,
                    "login": info.username,
                }
            }))
        }
        Err(_) => {
            HttpResponse::Unauthorized()
                .json(CargoError::unauthorized())
        }
    }
}
