//! Registry 类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 包类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageType {
    Npm,
    Docker,
}

impl std::fmt::Display for PackageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageType::Npm => write!(f, "npm"),
            PackageType::Docker => write!(f, "docker"),
        }
    }
}

// ============================================================================
// Docker Registry V2 Types
// ============================================================================

/// Docker Registry V2 错误响应
#[derive(Debug, Clone, Serialize)]
pub struct DockerError {
    pub errors: Vec<DockerErrorDetail>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockerErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

impl DockerError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            errors: vec![DockerErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
                detail: None,
            }],
        }
    }
    
    pub fn with_detail(code: &str, message: &str, detail: serde_json::Value) -> Self {
        Self {
            errors: vec![DockerErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
                detail: Some(detail),
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
        Self::with_detail(
            "NAME_UNKNOWN",
            "repository name not known to registry",
            serde_json::json!({ "name": name }),
        )
    }
    
    pub fn manifest_unknown() -> Self {
        Self::new("MANIFEST_UNKNOWN", "manifest unknown to registry")
    }
    
    pub fn manifest_invalid(reason: &str) -> Self {
        Self::with_detail(
            "MANIFEST_INVALID",
            "manifest invalid",
            serde_json::json!({ "reason": reason }),
        )
    }
    
    pub fn blob_unknown() -> Self {
        Self::new("BLOB_UNKNOWN", "blob unknown to registry")
    }
    
    pub fn blob_upload_unknown() -> Self {
        Self::new("BLOB_UPLOAD_UNKNOWN", "blob upload unknown to registry")
    }
    
    pub fn digest_invalid() -> Self {
        Self::new("DIGEST_INVALID", "provided digest did not match uploaded content")
    }
    
    pub fn size_invalid() -> Self {
        Self::new("SIZE_INVALID", "provided length did not match content length")
    }
    
    pub fn name_invalid(reason: &str) -> Self {
        Self::with_detail(
            "NAME_INVALID",
            "invalid repository name",
            serde_json::json!({ "reason": reason }),
        )
    }
    
    pub fn unsupported() -> Self {
        Self::new("UNSUPPORTED", "the operation is unsupported")
    }
    
    pub fn internal_error(reason: &str) -> Self {
        Self::with_detail(
            "INTERNAL_ERROR",
            "internal server error",
            serde_json::json!({ "reason": reason }),
        )
    }
}

/// Docker Manifest (V2 Schema 2)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifestV2 {
    pub schema_version: i32,
    pub media_type: String,
    pub config: DockerDescriptor,
    pub layers: Vec<DockerDescriptor>,
}

/// Docker Manifest List (多架构镜像)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifestList {
    pub schema_version: i32,
    pub media_type: String,
    pub manifests: Vec<DockerManifestListEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifestListEntry {
    pub media_type: String,
    pub size: i64,
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<DockerPlatform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerPlatform {
    pub architecture: String,
    pub os: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

/// 描述符（用于 config 和 layers）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerDescriptor {
    pub media_type: String,
    pub size: i64,
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,
}

/// 标签列表响应
#[derive(Debug, Clone, Serialize)]
pub struct DockerTagList {
    pub name: String,
    pub tags: Vec<String>,
}

/// Catalog 响应
#[derive(Debug, Clone, Serialize)]
pub struct DockerCatalog {
    pub repositories: Vec<String>,
}

// ============================================================================
// npm Registry Types
// ============================================================================

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
    
    pub fn bad_request(reason: &str) -> Self {
        Self::new("bad_request", reason)
    }
    
    pub fn internal_error(reason: &str) -> Self {
        Self::new("internal_error", reason)
    }
}

/// npm 包文档（GET /{package} 响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmPackageDoc {
    /// 包名
    pub name: String,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// dist-tags
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
    /// 版本信息
    pub versions: HashMap<String, NpmVersionDoc>,
    /// 时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<HashMap<String, String>>,
    /// README
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
    /// 许可证
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// 关键词
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
}

/// npm 版本文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmVersionDoc {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fileCount: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unpackedSize: Option<i64>,
}

/// npm 发布请求体
#[derive(Debug, Clone, Deserialize)]
pub struct NpmPublishRequest {
    pub name: String,
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
    pub versions: HashMap<String, NpmVersionDoc>,
    #[serde(rename = "_attachments")]
    pub attachments: HashMap<String, NpmAttachment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readme: Option<String>,
}

/// npm 附件
#[derive(Debug, Clone, Deserialize)]
pub struct NpmAttachment {
    pub content_type: String,
    pub data: String,  // base64
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<i64>,
}

/// npm 发布成功响应
#[derive(Debug, Clone, Serialize)]
pub struct NpmPublishResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
}

// ============================================================================
// 认证相关类型
// ============================================================================

/// 认证结果
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub project_id: i64,
    pub can_read: bool,
    pub can_write: bool,
}

/// Docker Registry Token 响应
#[derive(Debug, Clone, Serialize)]
pub struct DockerTokenResponse {
    pub token: String,
    pub access_token: Option<String>,
    pub expires_in: i64,
    pub issued_at: String,
}

/// WWW-Authenticate 头信息
#[derive(Debug, Clone)]
pub struct WwwAuthenticate {
    pub realm: String,
    pub service: String,
    pub scope: Option<String>,
}

impl WwwAuthenticate {
    pub fn to_header_value(&self) -> String {
        let mut parts = vec![
            format!("realm=\"{}\"", self.realm),
            format!("service=\"{}\"", self.service),
        ];
        if let Some(ref scope) = self.scope {
            parts.push(format!("scope=\"{}\"", scope));
        }
        format!("Bearer {}", parts.join(","))
    }
}
