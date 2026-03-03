//! Package Registry 模型定义
//! 
//! 支持 npm、Docker 等包管理器的软件包注册表

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;

/// 包类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "package_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    Npm,
    Docker,
    Generic,
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageType::Npm => write!(f, "npm"),
            PackageType::Docker => write!(f, "docker"),
            PackageType::Generic => write!(f, "generic"),
        }
    }
}

impl std::str::FromStr for PackageType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "npm" => Ok(PackageType::Npm),
            "docker" => Ok(PackageType::Docker),
            "generic" => Ok(PackageType::Generic),
            _ => Err(format!("Unknown package type: {}", s)),
        }
    }
}

/// 包状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "package_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PackageStatus {
    Default,
    Hidden,
    Deleted,
}

/// 包实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Package {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub package_type: PackageType,
    pub version: String,
    pub status: PackageStatus,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建包请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePackageRequest {
    pub name: String,
    pub version: String,
    pub package_type: PackageType,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// 包文件实体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PackageFile {
    pub id: i64,
    pub package_id: i64,
    pub file_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub file_sha256: String,
    pub created_at: DateTime<Utc>,
}

/// 包列表响应
#[derive(Debug, Clone, Serialize)]
pub struct PackageListResponse {
    pub packages: Vec<PackageSummary>,
    pub total: i64,
}

/// 包摘要（用于列表）
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PackageSummary {
    pub id: i64,
    pub name: String,
    pub package_type: PackageType,
    pub version: String,
    pub created_at: DateTime<Utc>,
}

/// 包详情响应
#[derive(Debug, Clone, Serialize)]
pub struct PackageDetailResponse {
    pub package: Package,
    pub files: Vec<PackageFile>,
}

// ============================================================================
// Docker Registry 特定类型
// ============================================================================

/// Docker Manifest
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DockerManifest {
    pub id: i64,
    pub package_id: i64,
    pub digest: String,
    pub media_type: String,
    pub schema_version: i32,
    pub config_digest: Option<String>,
    pub total_size: i64,
    pub manifest_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Docker Blob
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DockerBlob {
    pub id: i64,
    pub project_id: i64,
    pub digest: String,
    pub media_type: Option<String>,
    pub size: i64,
    pub file_path: String,
    pub created_at: DateTime<Utc>,
}

/// Docker 上传会话
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DockerUploadSession {
    pub id: i64,
    pub uuid: String,
    pub project_id: i64,
    pub user_id: uuid::Uuid,
    pub digest: Option<String>,
    pub uploaded_bytes: i64,
    pub temp_path: Option<String>,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Docker Registry V2 错误响应
#[derive(Debug, Clone, Serialize)]
pub struct DockerRegistryError {
    pub errors: Vec<DockerRegistryErrorDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerRegistryErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

impl DockerRegistryError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            errors: vec![DockerRegistryErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
                detail: None,
            }],
        }
    }
    
    pub fn unauthorized() -> Self {
        Self::new("UNAUTHORIZED", "authentication required")
    }
    
    pub fn denied() -> Self {
        Self::new("DENIED", "requested access to the resource is denied")
    }
    
    pub fn name_unknown(name: &str) -> Self {
        Self {
            errors: vec![DockerRegistryErrorDetail {
                code: "NAME_UNKNOWN".to_string(),
                message: format!("repository name not known to registry"),
                detail: Some(serde_json::json!({ "name": name })),
            }],
        }
    }
    
    pub fn manifest_unknown() -> Self {
        Self::new("MANIFEST_UNKNOWN", "manifest unknown")
    }
    
    pub fn blob_unknown() -> Self {
        Self::new("BLOB_UNKNOWN", "blob unknown to registry")
    }
    
    pub fn digest_invalid() -> Self {
        Self::new("DIGEST_INVALID", "provided digest did not match uploaded content")
    }
    
    pub fn size_invalid() -> Self {
        Self::new("SIZE_INVALID", "provided length did not match content length")
    }
}

// ============================================================================
// npm Registry 特定类型
// ============================================================================

/// npm 包元数据
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NpmPackageMetadata {
    pub id: i64,
    pub package_id: i64,
    pub dist_tag: String,
    pub tarball_sha512: Option<String>,
    pub npm_readme: Option<String>,
    pub npm_keywords: Option<Vec<String>>,
    pub npm_license: Option<String>,
    pub npm_repository: Option<serde_json::Value>,
    pub npm_dependencies: Option<serde_json::Value>,
    pub npm_dev_dependencies: Option<serde_json::Value>,
    pub npm_peer_dependencies: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// npm dist-tag
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NpmDistTag {
    pub id: i64,
    pub project_id: i64,
    pub package_name: String,
    pub tag: String,
    pub version: String,
    pub updated_at: DateTime<Utc>,
}

/// npm 包文档响应（GET /<package>）
#[derive(Debug, Clone, Serialize)]
pub struct NpmPackageDocument {
    /// 包名
    pub name: String,
    /// dist-tags 映射
    #[serde(rename = "dist-tags")]
    pub dist_tags: std::collections::HashMap<String, String>,
    /// 版本映射
    pub versions: std::collections::HashMap<String, NpmVersionInfo>,
    /// 修改时间
    pub time: std::collections::HashMap<String, String>,
    /// 可选字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
}

/// npm 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmVersionInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<serde_json::Value>,
    #[serde(rename = "devDependencies", skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<serde_json::Value>,
    #[serde(rename = "peerDependencies", skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// dist 信息（tarball URL, shasum 等）
    pub dist: NpmDistInfo,
}

/// npm dist 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmDistInfo {
    pub tarball: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shasum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
}

/// npm 发布请求
#[derive(Debug, Clone, Deserialize)]
pub struct NpmPublishRequest {
    pub name: String,
    #[serde(rename = "dist-tags")]
    pub dist_tags: std::collections::HashMap<String, String>,
    pub versions: std::collections::HashMap<String, NpmVersionInfo>,
    #[serde(rename = "_attachments")]
    pub attachments: std::collections::HashMap<String, NpmAttachment>,
}

/// npm 附件（tarball）
#[derive(Debug, Clone, Deserialize)]
pub struct NpmAttachment {
    pub content_type: String,
    pub data: String,  // base64 编码的数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<i64>,
}

/// npm 错误响应
#[derive(Debug, Clone, Serialize)]
pub struct NpmError {
    pub error: String,
    pub reason: String,
}

impl NpmError {
    pub fn new(error: &str, reason: &str) -> Self {
        Self {
            error: error.to_string(),
            reason: reason.to_string(),
        }
    }
    
    pub fn unauthorized() -> Self {
        Self::new("unauthorized", "Authentication required")
    }
    
    pub fn forbidden() -> Self {
        Self::new("forbidden", "Access denied")
    }
    
    pub fn not_found(pkg: &str) -> Self {
        Self::new("not_found", &format!("Package {} not found", pkg))
    }
    
    pub fn conflict(version: &str) -> Self {
        Self::new("conflict", &format!("Version {} already exists", version))
    }
}
