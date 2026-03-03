//! gRPC LFS Service implementation
//!
//! 主应用作为 gRPC 服务端，为 Workhorse 提供 LFS 元数据和锁管理服务。
//! 实际的 LFS 对象存储由 Workhorse 直接处理。

use tonic::{Request, Response, Status};
use sqlx::PgPool;
use log::{debug, info, warn, error};
use std::sync::Arc;
use std::collections::HashMap;

use crate::config::Config;

// 导入生成的 proto 代码
pub mod lfs_proto {
    tonic::include_proto!("gitfox.lfs");
}

use lfs_proto::lfs_service_server::{LfsService, LfsServiceServer};
use lfs_proto::*;

/// LFS gRPC 服务实现
pub struct LfsServiceImpl {
    pool: PgPool,
    config: Arc<Config>,
}

impl LfsServiceImpl {
    pub fn new(pool: PgPool, config: Arc<Config>) -> Self {
        Self { pool, config }
    }

    /// 创建 gRPC 服务
    pub fn into_service(self) -> LfsServiceServer<Self> {
        LfsServiceServer::new(self)
    }

    /// 验证内部调用 token
    fn verify_internal_token(&self, req: &Request<impl std::fmt::Debug>) -> Result<(), Status> {
        let token = req
            .metadata()
            .get("x-gitfox-shell-token")
            .and_then(|v| v.to_str().ok());

        match token {
            Some(t) if t == self.config.shell_secret => Ok(()),
            _ => {
                warn!("Invalid or missing shell token in LFS gRPC request");
                Err(Status::unauthenticated("Invalid shell token"))
            }
        }
    }

    /// 检查项目是否启用 LFS
    async fn check_project_lfs_enabled(&self, project_id: i64) -> Result<bool, Status> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT lfs_enabled FROM projects WHERE id = $1"
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(result.unwrap_or(false))
    }

    /// 检查用户对项目的访问权限
    async fn check_project_access(
        &self,
        project_id: i64,
        user_id: i64,
        require_write: bool,
    ) -> Result<bool, Status> {
        // 检查用户是否是项目成员
        let member = sqlx::query_as::<_, (String,)>(
            "SELECT role::text FROM project_members WHERE project_id = $1 AND user_id = $2"
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if let Some((role,)) = member {
            if require_write {
                // 只有 maintainer, owner, developer 有写权限
                Ok(role == "owner" || role == "maintainer" || role == "developer")
            } else {
                // 所有成员都有读权限
                Ok(true)
            }
        } else {
            // 检查项目是否公开
            let visibility = sqlx::query_scalar::<_, String>(
                "SELECT visibility::text FROM projects WHERE id = $1"
            )
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

            if let Some(visibility) = visibility {
                if require_write {
                    Ok(false) // 非成员没有写权限
                } else {
                    // 公开项目可以读
                    Ok(visibility == "public")
                }
            } else {
                Ok(false)
            }
        }
    }

    /// 获取用户名
    async fn get_username(&self, user_id: i64) -> Result<String, Status> {
        sqlx::query_scalar::<_, String>(
            "SELECT username FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::not_found("User not found"))
    }
}

#[tonic::async_trait]
impl LfsService for LfsServiceImpl {
    /// 批量处理 LFS 对象请求（核心 Batch API）
    async fn batch_objects(
        &self,
        request: Request<BatchObjectsRequest>,
    ) -> Result<Response<BatchObjectsResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "BatchObjects: project_id={}, user_id={}, operation={:?}, objects={}",
            req.project_id, req.user_id, req.operation, req.objects.len()
        );

        // 检查 LFS 是否启用
        if !self.check_project_lfs_enabled(req.project_id).await? {
            return Ok(Response::new(BatchObjectsResponse {
                transfer: String::new(),
                objects: vec![],
                message: "LFS is not enabled for this project".to_string(),
            }));
        }

        // 检查权限
        let is_upload = req.operation == BatchOperation::Upload as i32;
        if !self.check_project_access(req.project_id, req.user_id, is_upload).await? {
            return Err(Status::permission_denied("Access denied"));
        }

        let mut batch_objects = Vec::with_capacity(req.objects.len());
        let mut total_size: i64 = 0;

        for obj_id in req.objects {
            let oid = obj_id.oid.clone();
            let size = obj_id.size;
            total_size += size;

            if is_upload {
                // Upload: 检查对象是否已存在
                let existing = sqlx::query_scalar::<_, i64>(
                    "SELECT id FROM lfs_objects WHERE oid = $1"
                )
                .bind(&oid)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

                let actions = if existing.is_some() {
                    // 对象已存在，只需要关联到项目
                    if let Some(lfs_id) = existing {
                        // 检查项目关联是否存在
                        let link_exists = sqlx::query_scalar::<_, bool>(
                            "SELECT EXISTS(SELECT 1 FROM project_lfs_objects WHERE project_id = $1 AND lfs_object_id = $2)"
                        )
                        .bind(req.project_id)
                        .bind(lfs_id)
                        .fetch_one(&self.pool)
                        .await
                        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

                        if !link_exists {
                            // 创建项目关联
                            sqlx::query(
                                "INSERT INTO project_lfs_objects (project_id, lfs_object_id) VALUES ($1, $2)"
                            )
                            .bind(req.project_id)
                            .bind(lfs_id)
                            .execute(&self.pool)
                            .await
                            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
                        }
                    }
                    // 对象已存在，不需要上传动作
                    HashMap::new()
                } else {
                    // 对象不存在，需要上传
                    // 注意：实际的 upload/verify URL 由 Workhorse 生成
                    // 这里只返回空的 actions map，Workhorse 会填充具体 URL
                    let mut actions = HashMap::new();
                    actions.insert("upload".to_string(), LfsAction {
                        href: String::new(), // Workhorse 填充
                        header: HashMap::new(),
                        expires_in: self.config.lfs_link_expires.unwrap_or(3600),
                    });
                    actions.insert("verify".to_string(), LfsAction {
                        href: String::new(), // Workhorse 填充
                        header: HashMap::new(),
                        expires_in: self.config.lfs_link_expires.unwrap_or(3600),
                    });
                    actions
                };

                batch_objects.push(BatchObject {
                    oid,
                    size,
                    authenticated: true,
                    actions,
                    error: None,
                });
            } else {
                // Download: 检查对象是否存在且关联到项目
                let lfs_obj = sqlx::query_as::<_, (i64, String, i64)>(
                    r#"
                    SELECT lo.id, lo.oid, lo.size 
                    FROM lfs_objects lo
                    JOIN project_lfs_objects plo ON lo.id = plo.lfs_object_id
                    WHERE lo.oid = $1 AND plo.project_id = $2
                    "#
                )
                .bind(&oid)
                .bind(req.project_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

                if let Some((_, _, actual_size)) = lfs_obj {
                    // 对象存在，返回下载动作
                    let mut actions = HashMap::new();
                    actions.insert("download".to_string(), LfsAction {
                        href: String::new(), // Workhorse 填充
                        header: HashMap::new(),
                        expires_in: self.config.lfs_link_expires.unwrap_or(3600),
                    });

                    batch_objects.push(BatchObject {
                        oid,
                        size: actual_size,
                        authenticated: true,
                        actions,
                        error: None,
                    });
                } else {
                    // 对象不存在
                    batch_objects.push(BatchObject {
                        oid,
                        size,
                        authenticated: true,
                        actions: HashMap::new(),
                        error: Some(LfsObjectError {
                            code: 404,
                            message: "Object not found".to_string(),
                        }),
                    });
                }
            }
        }

        // 记录批量操作
        let operation_str = if is_upload { "upload" } else { "download" };
        let _ = sqlx::query(
            r#"
            INSERT INTO lfs_batch_operations (project_id, user_id, operation, object_count, total_size)
            VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(req.project_id)
        .bind(req.user_id)
        .bind(operation_str)
        .bind(batch_objects.len() as i32)
        .bind(total_size)
        .execute(&self.pool)
        .await;

        Ok(Response::new(BatchObjectsResponse {
            transfer: "basic".to_string(),
            objects: batch_objects,
            message: String::new(),
        }))
    }

    /// 验证上传完成的对象
    async fn verify_object(
        &self,
        request: Request<VerifyObjectRequest>,
    ) -> Result<Response<VerifyObjectResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "VerifyObject: project_id={}, oid={}, size={}",
            req.project_id, req.oid, req.size
        );

        // 检查权限
        if !self.check_project_access(req.project_id, req.user_id, true).await? {
            return Err(Status::permission_denied("Access denied"));
        }

        // 创建或获取 LFS 对象记录
        let lfs_object = sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO lfs_objects (oid, size) 
            VALUES ($1, $2)
            ON CONFLICT (oid) DO UPDATE SET size = EXCLUDED.size
            RETURNING id
            "#
        )
        .bind(&req.oid)
        .bind(req.size)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // 关联到项目
        sqlx::query(
            r#"
            INSERT INTO project_lfs_objects (project_id, lfs_object_id)
            VALUES ($1, $2)
            ON CONFLICT (project_id, lfs_object_id) DO NOTHING
            "#
        )
        .bind(req.project_id)
        .bind(lfs_object.0)
        .execute(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // 更新项目 LFS 存储使用量
        sqlx::query(
            "UPDATE projects SET lfs_storage_used = lfs_storage_used + $1 WHERE id = $2"
        )
        .bind(req.size)
        .bind(req.project_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update LFS storage: {}", e);
            Status::internal("Failed to update storage")
        })?;

        info!("LFS object verified: oid={}, size={}, project={}", req.oid, req.size, req.project_id);

        Ok(Response::new(VerifyObjectResponse {
            success: true,
            message: String::new(),
        }))
    }

    /// 获取单个 LFS 对象信息
    async fn get_object(
        &self,
        request: Request<GetObjectRequest>,
    ) -> Result<Response<GetObjectResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!("GetObject: project_id={}, oid={}", req.project_id, req.oid);

        let lfs_obj = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT lo.oid, lo.size 
            FROM lfs_objects lo
            JOIN project_lfs_objects plo ON lo.id = plo.lfs_object_id
            WHERE lo.oid = $1 AND plo.project_id = $2
            "#
        )
        .bind(&req.oid)
        .bind(req.project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if let Some((oid, size)) = lfs_obj {
            // 构建存储路径（格式：{oid[0:2]}/{oid[2:4]}/{oid}）
            let storage_path = format!("{}/{}/{}", &oid[0..2], &oid[2..4], &oid);

            Ok(Response::new(GetObjectResponse {
                found: true,
                oid,
                size,
                storage_path,
            }))
        } else {
            Ok(Response::new(GetObjectResponse {
                found: false,
                oid: req.oid,
                size: 0,
                storage_path: String::new(),
            }))
        }
    }

    /// 创建文件锁
    async fn create_lock(
        &self,
        request: Request<CreateLockRequest>,
    ) -> Result<Response<CreateLockResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "CreateLock: project_id={}, user_id={}, path={}",
            req.project_id, req.user_id, req.path
        );

        // 检查权限
        if !self.check_project_access(req.project_id, req.user_id, true).await? {
            return Err(Status::permission_denied("Access denied"));
        }

        // 检查是否已存在锁
        let existing_lock = sqlx::query_as::<_, (i64, i64, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, user_id, path, locked_at FROM lfs_locks WHERE project_id = $1 AND path = $2"
        )
        .bind(req.project_id)
        .bind(&req.path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if let Some((lock_id, lock_user_id, path, locked_at)) = existing_lock {
            // 锁已存在
            let owner_name = self.get_username(lock_user_id).await?;
            return Ok(Response::new(CreateLockResponse {
                success: false,
                lock: Some(LfsLock {
                    id: lock_id.to_string(),
                    path,
                    owner: Some(LfsLockOwner { name: owner_name }),
                    locked_at: locked_at.to_rfc3339(),
                }),
                message: "Lock already exists".to_string(),
            }));
        }

        // 创建新锁
        let ref_name = if req.r#ref.is_empty() { None } else { Some(req.r#ref.clone()) };
        let lock = sqlx::query_as::<_, (i64, String, chrono::DateTime<chrono::Utc>)>(
            r#"
            INSERT INTO lfs_locks (project_id, user_id, path, ref_name)
            VALUES ($1, $2, $3, $4)
            RETURNING id, path, locked_at
            "#
        )
        .bind(req.project_id)
        .bind(req.user_id)
        .bind(&req.path)
        .bind(ref_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        info!("LFS lock created: id={}, path={}, project={}", lock.0, lock.1, req.project_id);

        Ok(Response::new(CreateLockResponse {
            success: true,
            lock: Some(LfsLock {
                id: lock.0.to_string(),
                path: lock.1,
                owner: Some(LfsLockOwner { name: req.username }),
                locked_at: lock.2.to_rfc3339(),
            }),
            message: String::new(),
        }))
    }

    /// 列出文件锁
    async fn list_locks(
        &self,
        request: Request<ListLocksRequest>,
    ) -> Result<Response<ListLocksResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "ListLocks: project_id={}, path={:?}, id={:?}",
            req.project_id, req.path, req.id
        );

        let limit = if req.limit > 0 { req.limit.min(100) } else { 100 };
        let cursor: i64 = req.cursor.parse().unwrap_or(0);

        // 构建查询
        let locks = if !req.path.is_empty() {
            sqlx::query_as::<_, (i64, i64, String, chrono::DateTime<chrono::Utc>)>(
                r#"
                SELECT id, user_id, path, locked_at 
                FROM lfs_locks 
                WHERE project_id = $1 AND path = $2 AND id > $3
                ORDER BY id
                LIMIT $4
                "#
            )
            .bind(req.project_id)
            .bind(&req.path)
            .bind(cursor)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        } else if !req.id.is_empty() {
            let lock_id: i64 = req.id.parse().map_err(|_| Status::invalid_argument("Invalid lock ID"))?;
            sqlx::query_as::<_, (i64, i64, String, chrono::DateTime<chrono::Utc>)>(
                r#"
                SELECT id, user_id, path, locked_at 
                FROM lfs_locks 
                WHERE project_id = $1 AND id = $2
                "#
            )
            .bind(req.project_id)
            .bind(lock_id)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, (i64, i64, String, chrono::DateTime<chrono::Utc>)>(
                r#"
                SELECT id, user_id, path, locked_at 
                FROM lfs_locks 
                WHERE project_id = $1 AND id > $2
                ORDER BY id
                LIMIT $3
                "#
            )
            .bind(req.project_id)
            .bind(cursor)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
        }.map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // 转换为响应格式
        let mut response_locks = Vec::with_capacity(locks.len());
        let mut next_cursor = String::new();

        for (id, user_id, path, locked_at) in locks {
            let owner_name = self.get_username(user_id).await.unwrap_or_else(|_| "unknown".to_string());
            response_locks.push(LfsLock {
                id: id.to_string(),
                path,
                owner: Some(LfsLockOwner { name: owner_name }),
                locked_at: locked_at.to_rfc3339(),
            });
            next_cursor = id.to_string();
        }

        // 如果返回数量等于 limit，可能还有更多
        let next_cursor = if response_locks.len() as i32 == limit {
            next_cursor
        } else {
            String::new()
        };

        Ok(Response::new(ListLocksResponse {
            locks: response_locks,
            next_cursor,
        }))
    }

    /// 删除文件锁
    async fn delete_lock(
        &self,
        request: Request<DeleteLockRequest>,
    ) -> Result<Response<DeleteLockResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "DeleteLock: project_id={}, user_id={}, id={}, force={}",
            req.project_id, req.user_id, req.id, req.force
        );

        let lock_id: i64 = req.id.parse().map_err(|_| Status::invalid_argument("Invalid lock ID"))?;

        // 获取锁信息
        let lock = sqlx::query_as::<_, (i64, i64, String, chrono::DateTime<chrono::Utc>)>(
            "SELECT id, user_id, path, locked_at FROM lfs_locks WHERE id = $1 AND project_id = $2"
        )
        .bind(lock_id)
        .bind(req.project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let Some((id, lock_user_id, path, locked_at)) = lock else {
            return Ok(Response::new(DeleteLockResponse {
                success: false,
                lock: None,
                message: "Lock not found".to_string(),
            }));
        };

        // 检查是否是锁的所有者或强制删除
        if lock_user_id != req.user_id && !req.force {
            let owner_name = self.get_username(lock_user_id).await?;
            return Ok(Response::new(DeleteLockResponse {
                success: false,
                lock: Some(LfsLock {
                    id: id.to_string(),
                    path,
                    owner: Some(LfsLockOwner { name: owner_name }),
                    locked_at: locked_at.to_rfc3339(),
                }),
                message: "Lock owned by another user. Use force to delete.".to_string(),
            }));
        }

        // 删除锁
        sqlx::query("DELETE FROM lfs_locks WHERE id = $1")
            .bind(lock_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let owner_name = self.get_username(lock_user_id).await.unwrap_or_else(|_| "unknown".to_string());
        info!("LFS lock deleted: id={}, path={}, project={}", id, path, req.project_id);

        Ok(Response::new(DeleteLockResponse {
            success: true,
            lock: Some(LfsLock {
                id: id.to_string(),
                path,
                owner: Some(LfsLockOwner { name: owner_name }),
                locked_at: locked_at.to_rfc3339(),
            }),
            message: String::new(),
        }))
    }

    /// 验证锁状态（返回当前用户和其他用户的锁）
    async fn verify_locks(
        &self,
        request: Request<VerifyLocksRequest>,
    ) -> Result<Response<VerifyLocksResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "VerifyLocks: project_id={}, user_id={}",
            req.project_id, req.user_id
        );

        let limit = if req.limit > 0 { req.limit.min(100) } else { 100 };
        let cursor: i64 = req.cursor.parse().unwrap_or(0);

        // 获取所有锁
        let locks = sqlx::query_as::<_, (i64, i64, String, chrono::DateTime<chrono::Utc>)>(
            r#"
            SELECT id, user_id, path, locked_at 
            FROM lfs_locks 
            WHERE project_id = $1 AND id > $2
            ORDER BY id
            LIMIT $3
            "#
        )
        .bind(req.project_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let mut ours = Vec::new();
        let mut theirs = Vec::new();
        let mut next_cursor = String::new();

        for (id, lock_user_id, path, locked_at) in locks {
            let owner_name = self.get_username(lock_user_id).await.unwrap_or_else(|_| "unknown".to_string());
            let lock = LfsLock {
                id: id.to_string(),
                path,
                owner: Some(LfsLockOwner { name: owner_name }),
                locked_at: locked_at.to_rfc3339(),
            };

            if lock_user_id == req.user_id {
                ours.push(lock);
            } else {
                theirs.push(lock);
            }
            next_cursor = id.to_string();
        }

        // 如果返回数量等于 limit，可能还有更多
        let next_cursor = if (ours.len() + theirs.len()) as i32 == limit {
            next_cursor
        } else {
            String::new()
        };

        Ok(Response::new(VerifyLocksResponse {
            ours,
            theirs,
            next_cursor,
        }))
    }
}
