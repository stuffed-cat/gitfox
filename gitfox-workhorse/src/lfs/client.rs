//! LFS gRPC 客户端
//!
//! 通过 gRPC 调用主应用的 LFS 服务进行元数据管理。

use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{debug, error, info};

use crate::lfs::types::{
    LfsError, LfsLock, LfsLockOwner, LfsObject, LfsVerifyLocksResponse,
};

// 导入生成的 proto 代码
pub mod lfs_proto {
    tonic::include_proto!("gitfox.lfs");
}

use lfs_proto::lfs_service_client::LfsServiceClient;
use lfs_proto::{
    BatchObjectsRequest, BatchOperation, CreateLockRequest, DeleteLockRequest,
    GetObjectRequest, ListLocksRequest, LfsObjectId, VerifyLocksRequest,
    VerifyObjectRequest,
};

/// LFS gRPC 客户端
#[derive(Clone)]
pub struct LfsClient {
    address: String,
    shell_token: String,
    channel: Arc<RwLock<Option<Channel>>>,
}

impl LfsClient {
    /// 创建新的 LFS 客户端
    pub fn new(address: &str, shell_token: String) -> Self {
        Self {
            address: address.to_string(),
            shell_token,
            channel: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取或创建 gRPC channel
    async fn get_channel(&self) -> Result<Channel, LfsClientError> {
        // 检查是否有有效的 channel
        {
            let read_guard = self.channel.read().await;
            if let Some(ref ch) = *read_guard {
                return Ok(ch.clone());
            }
        }

        // 创建新 channel
        let mut write_guard = self.channel.write().await;

        // Double-check
        if let Some(ref ch) = *write_guard {
            return Ok(ch.clone());
        }

        debug!("Connecting to LFS service at {}", self.address);

        let channel = Channel::from_shared(self.address.clone())
            .map_err(|e| LfsClientError::Connection(format!("Invalid address: {}", e)))?
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(30))
            .connect()
            .await
            .map_err(|e| LfsClientError::Connection(format!("Failed to connect: {}", e)))?;

        *write_guard = Some(channel.clone());
        Ok(channel)
    }

    /// 添加认证 header
    fn add_auth<T>(&self, request: &mut Request<T>) {
        if let Ok(token) = self.shell_token.parse::<MetadataValue<_>>() {
            request.metadata_mut().insert("x-gitfox-shell-token", token);
        }
    }

    /// 批量对象操作（Batch API 核心）
    pub async fn batch_objects(
        &self,
        project_id: i64,
        user_id: i64,
        operation: &str,
        objects: Vec<LfsObject>,
        ref_name: Option<String>,
    ) -> Result<Vec<BatchObjectResult>, LfsClientError> {
        let channel = self.get_channel().await?;
        let mut client = LfsServiceClient::new(channel);

        let proto_operation = match operation {
            "download" => BatchOperation::Download,
            "upload" => BatchOperation::Upload,
            _ => return Err(LfsClientError::InvalidOperation(operation.to_string())),
        };

        let proto_objects: Vec<LfsObjectId> = objects
            .iter()
            .map(|o| LfsObjectId {
                oid: o.oid.clone(),
                size: o.size,
            })
            .collect();

        let mut request = Request::new(BatchObjectsRequest {
            project_id,
            user_id,
            operation: proto_operation as i32,
            objects: proto_objects,
            r#ref: ref_name.unwrap_or_default(),
        });
        self.add_auth(&mut request);

        let response = client
            .batch_objects(request)
            .await
            .map_err(|e| LfsClientError::Rpc(format!("BatchObjects failed: {}", e)))?;

        let resp = response.into_inner();
        
        // 如果有全局错误消息
        if !resp.message.is_empty() {
            return Err(LfsClientError::ServerError(resp.message));
        }

        // 转换响应
        let results: Vec<BatchObjectResult> = resp
            .objects
            .into_iter()
            .map(|obj| {
                let error = if obj.error.is_some() {
                    let err = obj.error.unwrap();
                    Some(LfsError {
                        code: err.code,
                        message: err.message,
                    })
                } else {
                    None
                };

                BatchObjectResult {
                    oid: obj.oid,
                    size: obj.size,
                    authenticated: obj.authenticated,
                    actions: obj.actions,
                    error,
                }
            })
            .collect();

        Ok(results)
    }

    /// 验证上传完成的对象
    pub async fn verify_object(
        &self,
        project_id: i64,
        user_id: i64,
        oid: &str,
        size: i64,
    ) -> Result<bool, LfsClientError> {
        let channel = self.get_channel().await?;
        let mut client = LfsServiceClient::new(channel);

        let mut request = Request::new(VerifyObjectRequest {
            project_id,
            user_id,
            oid: oid.to_string(),
            size,
        });
        self.add_auth(&mut request);

        let response = client
            .verify_object(request)
            .await
            .map_err(|e| LfsClientError::Rpc(format!("VerifyObject failed: {}", e)))?;

        let resp = response.into_inner();
        if !resp.success {
            return Err(LfsClientError::ServerError(resp.message));
        }

        Ok(true)
    }

    /// 获取单个对象信息
    pub async fn get_object(
        &self,
        project_id: i64,
        oid: &str,
    ) -> Result<Option<LfsObjectMeta>, LfsClientError> {
        let channel = self.get_channel().await?;
        let mut client = LfsServiceClient::new(channel);

        let mut request = Request::new(GetObjectRequest {
            project_id,
            oid: oid.to_string(),
        });
        self.add_auth(&mut request);

        let response = client
            .get_object(request)
            .await
            .map_err(|e| LfsClientError::Rpc(format!("GetObject failed: {}", e)))?;

        let resp = response.into_inner();
        if !resp.found {
            return Ok(None);
        }

        Ok(Some(LfsObjectMeta {
            oid: resp.oid,
            size: resp.size,
            storage_path: resp.storage_path,
        }))
    }

    // ============ Lock API ============

    /// 创建文件锁
    pub async fn create_lock(
        &self,
        project_id: i64,
        user_id: i64,
        username: &str,
        path: &str,
        ref_name: Option<String>,
    ) -> Result<LfsLock, LfsClientError> {
        let channel = self.get_channel().await?;
        let mut client = LfsServiceClient::new(channel);

        let mut request = Request::new(CreateLockRequest {
            project_id,
            user_id,
            username: username.to_string(),
            path: path.to_string(),
            r#ref: ref_name.unwrap_or_default(),
        });
        self.add_auth(&mut request);

        let response = client
            .create_lock(request)
            .await
            .map_err(|e| LfsClientError::Rpc(format!("CreateLock failed: {}", e)))?;

        let resp = response.into_inner();
        if !resp.success {
            return Err(LfsClientError::ServerError(resp.message));
        }

        let lock = resp.lock.ok_or_else(|| {
            LfsClientError::ServerError("No lock returned".to_string())
        })?;

        Ok(convert_proto_lock(lock))
    }

    /// 列出文件锁
    pub async fn list_locks(
        &self,
        project_id: i64,
        path: Option<String>,
        id: Option<String>,
        cursor: Option<String>,
        limit: Option<i32>,
        ref_name: Option<String>,
    ) -> Result<(Vec<LfsLock>, Option<String>), LfsClientError> {
        let channel = self.get_channel().await?;
        let mut client = LfsServiceClient::new(channel);

        let mut request = Request::new(ListLocksRequest {
            project_id,
            path: path.unwrap_or_default(),
            id: id.unwrap_or_default(),
            cursor: cursor.unwrap_or_default(),
            limit: limit.unwrap_or(100),
            r#ref: ref_name.unwrap_or_default(),
        });
        self.add_auth(&mut request);

        let response = client
            .list_locks(request)
            .await
            .map_err(|e| LfsClientError::Rpc(format!("ListLocks failed: {}", e)))?;

        let resp = response.into_inner();
        let locks: Vec<LfsLock> = resp.locks.into_iter().map(convert_proto_lock).collect();
        let next_cursor = if resp.next_cursor.is_empty() {
            None
        } else {
            Some(resp.next_cursor)
        };

        Ok((locks, next_cursor))
    }

    /// 删除文件锁
    pub async fn delete_lock(
        &self,
        project_id: i64,
        user_id: i64,
        lock_id: &str,
        force: bool,
        ref_name: Option<String>,
    ) -> Result<LfsLock, LfsClientError> {
        let channel = self.get_channel().await?;
        let mut client = LfsServiceClient::new(channel);

        let mut request = Request::new(DeleteLockRequest {
            project_id,
            user_id,
            id: lock_id.to_string(),
            force,
            r#ref: ref_name.unwrap_or_default(),
        });
        self.add_auth(&mut request);

        let response = client
            .delete_lock(request)
            .await
            .map_err(|e| LfsClientError::Rpc(format!("DeleteLock failed: {}", e)))?;

        let resp = response.into_inner();
        if !resp.success {
            return Err(LfsClientError::ServerError(resp.message));
        }

        let lock = resp.lock.ok_or_else(|| {
            LfsClientError::ServerError("No lock returned".to_string())
        })?;

        Ok(convert_proto_lock(lock))
    }

    /// 验证锁状态
    pub async fn verify_locks(
        &self,
        project_id: i64,
        user_id: i64,
        cursor: Option<String>,
        limit: Option<i32>,
        ref_name: Option<String>,
    ) -> Result<LfsVerifyLocksResponse, LfsClientError> {
        let channel = self.get_channel().await?;
        let mut client = LfsServiceClient::new(channel);

        let mut request = Request::new(VerifyLocksRequest {
            project_id,
            user_id,
            cursor: cursor.unwrap_or_default(),
            limit: limit.unwrap_or(100),
            r#ref: ref_name.unwrap_or_default(),
        });
        self.add_auth(&mut request);

        let response = client
            .verify_locks(request)
            .await
            .map_err(|e| LfsClientError::Rpc(format!("VerifyLocks failed: {}", e)))?;

        let resp = response.into_inner();
        let ours: Vec<LfsLock> = resp.ours.into_iter().map(convert_proto_lock).collect();
        let theirs: Vec<LfsLock> = resp.theirs.into_iter().map(convert_proto_lock).collect();
        let next_cursor = if resp.next_cursor.is_empty() {
            None
        } else {
            Some(resp.next_cursor)
        };

        Ok(LfsVerifyLocksResponse {
            ours,
            theirs,
            next_cursor,
        })
    }
}

/// 将 proto Lock 转换为 API Lock
fn convert_proto_lock(lock: lfs_proto::LfsLock) -> LfsLock {
    LfsLock {
        id: lock.id,
        path: lock.path,
        locked_at: lock.locked_at,
        owner: LfsLockOwner {
            name: lock.owner.map(|o| o.name).unwrap_or_default(),
        },
    }
}

/// Batch 对象结果
#[derive(Debug, Clone)]
pub struct BatchObjectResult {
    pub oid: String,
    pub size: i64,
    pub authenticated: bool,
    pub actions: std::collections::HashMap<String, lfs_proto::LfsAction>,
    pub error: Option<LfsError>,
}

/// LFS 对象元数据
#[derive(Debug, Clone)]
pub struct LfsObjectMeta {
    pub oid: String,
    pub size: i64,
    pub storage_path: String,
}

/// LFS 客户端错误
#[derive(Debug, thiserror::Error)]
pub enum LfsClientError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}
