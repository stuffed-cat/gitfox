//! Git LFS (Large File Storage) 模型
//!
//! 定义 LFS 相关的数据库模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// LFS 对象 - 存储 LFS 对象的元数据
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LfsObject {
    pub id: i64,
    /// OID (SHA-256 hash)
    pub oid: String,
    /// 文件大小（字节）
    pub size: i64,
    pub created_at: DateTime<Utc>,
}

/// 项目 LFS 对象关联 - 多对多关系
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectLfsObject {
    pub id: i64,
    pub project_id: i64,
    pub lfs_object_id: i64,
    pub created_at: DateTime<Utc>,
}

/// LFS 文件锁
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LfsLock {
    pub id: i64,
    pub project_id: i64,
    pub user_id: i64,
    /// 被锁定的文件路径
    pub path: String,
    /// 可选的分支引用
    pub ref_name: Option<String>,
    pub locked_at: DateTime<Utc>,
}

/// LFS 批量操作记录（用于统计和限流）
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LfsBatchOperation {
    pub id: i64,
    pub project_id: i64,
    pub user_id: i64,
    /// 操作类型：download 或 upload
    pub operation: String,
    /// 对象数量
    pub object_count: i32,
    /// 总大小
    pub total_size: i64,
    pub created_at: DateTime<Utc>,
}

// ============ API 请求/响应类型 ============

/// 创建/获取 LFS 对象请求
#[derive(Debug, Deserialize)]
pub struct CreateLfsObjectRequest {
    pub oid: String,
    pub size: i64,
}

/// LFS 对象响应
#[derive(Debug, Serialize)]
pub struct LfsObjectResponse {
    pub id: i64,
    pub oid: String,
    pub size: i64,
    pub created_at: DateTime<Utc>,
}

impl From<LfsObject> for LfsObjectResponse {
    fn from(obj: LfsObject) -> Self {
        Self {
            id: obj.id,
            oid: obj.oid,
            size: obj.size,
            created_at: obj.created_at,
        }
    }
}

/// LFS 锁响应
#[derive(Debug, Serialize)]
pub struct LfsLockResponse {
    pub id: String,
    pub path: String,
    pub locked_at: String,
    pub owner: LfsLockOwnerResponse,
}

#[derive(Debug, Serialize)]
pub struct LfsLockOwnerResponse {
    pub name: String,
}

impl LfsLock {
    pub fn to_response(&self, username: &str) -> LfsLockResponse {
        LfsLockResponse {
            id: self.id.to_string(),
            path: self.path.clone(),
            locked_at: self.locked_at.to_rfc3339(),
            owner: LfsLockOwnerResponse {
                name: username.to_string(),
            },
        }
    }
}
