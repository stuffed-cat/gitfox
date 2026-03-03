//! Git LFS API 类型定义
//!
//! 符合 Git LFS Batch API v1 规范

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LFS Batch API 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsBatchRequest {
    /// 操作类型: "download" 或 "upload"
    pub operation: String,
    /// 传输类型，默认 "basic"
    #[serde(default = "default_transfers")]
    pub transfers: Vec<String>,
    /// 请求的对象列表
    pub objects: Vec<LfsObject>,
    /// 可选的 ref 信息
    #[serde(rename = "ref")]
    pub ref_info: Option<LfsRef>,
    /// 可选的哈希算法（默认 sha256）
    #[serde(default = "default_hash_algo")]
    pub hash_algo: String,
}

fn default_transfers() -> Vec<String> {
    vec!["basic".to_string()]
}

fn default_hash_algo() -> String {
    "sha256".to_string()
}

/// LFS 对象标识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsObject {
    /// OID (SHA-256 hash)
    pub oid: String,
    /// 文件大小（字节）
    pub size: i64,
}

/// LFS Ref 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsRef {
    /// Ref 名称（如 refs/heads/main）
    pub name: String,
}

/// LFS Batch API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsBatchResponse {
    /// 传输类型
    pub transfer: String,
    /// 对象列表
    pub objects: Vec<LfsBatchObject>,
    /// 哈希算法
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_algo: Option<String>,
}

/// LFS Batch 响应中的对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsBatchObject {
    /// OID
    pub oid: String,
    /// 大小
    pub size: i64,
    /// 是否已认证
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticated: Option<bool>,
    /// 可用动作
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<LfsBatchActions>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LfsError>,
}

/// LFS 动作集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsBatchActions {
    /// 下载动作
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download: Option<LfsAction>,
    /// 上传动作
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload: Option<LfsAction>,
    /// 验证动作
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify: Option<LfsAction>,
}

/// LFS 动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsAction {
    /// URL
    pub href: String,
    /// HTTP 头
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<HashMap<String, String>>,
    /// 过期时间（秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
    /// 过期时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// LFS 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsError {
    /// HTTP 状态码
    pub code: i32,
    /// 错误消息
    pub message: String,
}

/// LFS 验证请求（上传后验证）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsVerifyRequest {
    pub oid: String,
    pub size: i64,
}

// ============ Lock API 类型 ============

/// 创建锁请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsLockRequest {
    /// 文件路径
    pub path: String,
    /// 可选的 ref
    #[serde(rename = "ref")]
    pub ref_info: Option<LfsRef>,
}

/// 锁响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsLockResponse {
    /// 创建的锁
    pub lock: LfsLock,
}

/// LFS 锁
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsLock {
    /// 锁 ID
    pub id: String,
    /// 文件路径
    pub path: String,
    /// 锁定时间（ISO 8601）
    pub locked_at: String,
    /// 锁所有者
    pub owner: LfsLockOwner,
}

/// 锁所有者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsLockOwner {
    /// 用户名
    pub name: String,
}

/// 列出锁请求（查询参数）
#[derive(Debug, Clone, Deserialize)]
pub struct LfsListLocksQuery {
    /// 按路径过滤
    pub path: Option<String>,
    /// 按 ID 过滤
    pub id: Option<String>,
    /// 分页游标
    pub cursor: Option<String>,
    /// 每页数量
    pub limit: Option<i32>,
    /// Ref 名称
    pub refspec: Option<String>,
}

/// 列出锁响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsListLocksResponse {
    /// 锁列表
    pub locks: Vec<LfsLock>,
    /// 下一页游标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// 删除锁请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsUnlockRequest {
    /// 是否强制解锁
    #[serde(default)]
    pub force: bool,
    /// 可选的 ref
    #[serde(rename = "ref")]
    pub ref_info: Option<LfsRef>,
}

/// 删除锁响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsUnlockResponse {
    /// 被删除的锁
    pub lock: LfsLock,
}

/// 验证锁请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsVerifyLocksRequest {
    /// 分页游标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// 每页数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    /// 可选的 ref
    #[serde(rename = "ref")]
    pub ref_info: Option<LfsRef>,
}

/// 验证锁响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsVerifyLocksResponse {
    /// 当前用户持有的锁
    pub ours: Vec<LfsLock>,
    /// 其他用户持有的锁
    pub theirs: Vec<LfsLock>,
    /// 下一页游标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

// ============ 内部类型 ============

/// LFS 认证信息
#[derive(Debug, Clone)]
pub struct LfsAuthInfo {
    /// 用户 ID
    pub user_id: i64,
    /// 用户名
    pub username: String,
    /// 项目 ID
    pub project_id: i64,
    /// 是否有写权限
    pub can_write: bool,
}

/// LFS 请求上下文
#[derive(Debug, Clone)]
pub struct LfsContext {
    /// 命名空间
    pub namespace: String,
    /// 项目名
    pub project: String,
    /// 认证信息
    pub auth: Option<LfsAuthInfo>,
    /// 仓库路径
    pub repo_path: String,
}

impl LfsBatchResponse {
    /// 创建一个新的 Batch 响应
    pub fn new() -> Self {
        Self {
            transfer: "basic".to_string(),
            objects: Vec::new(),
            hash_algo: Some("sha256".to_string()),
        }
    }
}

impl Default for LfsBatchResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl LfsError {
    pub fn not_found(message: &str) -> Self {
        Self {
            code: 404,
            message: message.to_string(),
        }
    }

    pub fn forbidden(message: &str) -> Self {
        Self {
            code: 403,
            message: message.to_string(),
        }
    }

    pub fn unauthorized(message: &str) -> Self {
        Self {
            code: 401,
            message: message.to_string(),
        }
    }

    pub fn internal_error(message: &str) -> Self {
        Self {
            code: 500,
            message: message.to_string(),
        }
    }

    pub fn object_too_large(size: i64, max_size: u64) -> Self {
        Self {
            code: 413,
            message: format!(
                "Object size {} exceeds maximum allowed size {}",
                size, max_size
            ),
        }
    }
}
