//! gRPC Auth Service implementation
//! 
//! 主应用作为 gRPC 服务端，为 Workhorse 和 Shell 提供权限认证服务。
//! 这符合 GitLab 的架构模式：主应用只负责权限管理，不直接操作 Git。
//!
//! ## 权限验证三层交集
//! 
//! 最终权限 = Token Scopes ∩ User Role ∩ Project Membership
//! 
//! 1. **Token Scopes** - Token 本身的权限上限
//!    - JWT: 完全访问 (Full)
//!    - PAT: 创建时指定的 scopes
//!    - OAuth2: 授权时指定的 scopes
//! 
//! 2. **User Role** - 用户账户层面的权限
//!    - 用户是否激活
//!    - 用户是否为管理员
//! 
//! 3. **Project Membership** - 项目层面的权限
//!    - Owner: 完全控制
//!    - Maintainer/Developer: 可读写
//!    - Reporter/Guest: 只读
//!    - 非成员: 取决于项目可见性

use tonic::{Request, Response, Status};
use sqlx::PgPool;
use log::{debug, info, warn};
use std::sync::Arc;

use crate::config::Config;
use crate::services::CiService;
use crate::services::gitlayer::{CommitServiceClient, IsAncestorRequest, Repository};

// 导入生成的 proto 代码
pub mod auth_proto {
    tonic::include_proto!("gitfox.auth");
}

use auth_proto::auth_service_server::{AuthService, AuthServiceServer};
use auth_proto::*;

/// 认证结果，包含用户信息和 token 权限范围
/// 
/// 用于统一处理三种认证方式：JWT、PAT、OAuth2
#[derive(Debug, Clone)]
struct TokenInfo {
    user_id: i64,
    username: String,
    /// Token 的权限范围
    /// - None = Full 完全访问（JWT）
    /// - Some(scopes) = Limited 受限访问（PAT/OAuth2）
    scopes: Option<Vec<String>>,
}

impl TokenInfo {
    /// 匿名用户（未认证）
    fn anonymous() -> Self {
        Self {
            user_id: 0,
            username: String::new(),
            scopes: None,
        }
    }
    
    /// 完全访问（JWT）
    fn full_access(user_id: i64, username: String) -> Self {
        Self {
            user_id,
            username,
            scopes: None,
        }
    }
    
    /// 受限访问（PAT/OAuth2）
    fn limited_access(user_id: i64, username: String, scopes: Vec<String>) -> Self {
        Self {
            user_id,
            username,
            scopes: Some(scopes),
        }
    }
    
    /// 检查 token 是否有 repository:write 权限（push）
    fn can_write_repo(&self) -> bool {
        match &self.scopes {
            None => true, // Full access (JWT with no scope restrictions)
            Some(scopes) => {
                scopes.iter().any(|s| {
                    // 新格式
                    s == "repository:write" ||
                    // 旧格式（兼容）
                    s == "write_repository" ||
                    // Admin 权限包含所有
                    s == "admin"
                })
            }
        }
    }
    
    /// 检查 token 是否有 repository:read 权限（clone/fetch）
    fn can_read_repo(&self) -> bool {
        match &self.scopes {
            None => true, // Full access
            Some(scopes) => {
                scopes.iter().any(|s| {
                    // read 权限
                    s == "repository:read" ||
                    s == "read_repository" ||
                    // write 隐含 read
                    s == "repository:write" ||
                    s == "write_repository" ||
                    // Admin 权限包含所有
                    s == "admin"
                })
            }
        }
    }
    
    /// 是否已认证
    fn is_authenticated(&self) -> bool {
        self.user_id != 0
    }
}

/// Auth gRPC 服务实现
pub struct AuthServiceImpl {
    pool: PgPool,
    config: Arc<Config>,
    ci_service: CiService,
}

impl AuthServiceImpl {
    pub fn new(pool: PgPool, config: Arc<Config>) -> Self {
        let ci_service = CiService::new(pool.clone(), config.clone());
        Self { pool, config, ci_service }
    }

    /// 创建 gRPC 服务
    pub fn into_service(self) -> AuthServiceServer<Self> {
        AuthServiceServer::new(self)
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
                warn!("Invalid or missing shell token in gRPC request");
                Err(Status::unauthenticated("Invalid shell token"))
            }
        }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    /// 检查 SSH 访问权限
    /// 
    /// **三层权限交集** (与 HTTP 认证保持一致):
    /// 最终权限 = SSH Key (Full Access) ∩ User Role ∩ Project Membership
    /// 
    /// SSH 密钥代表用户的完全访问权限（等同于 JWT），但用户在项目上的权限
    /// 仍然受限于项目成员角色 (owner/maintainer/developer/reporter/guest)
    async fn check_ssh_access(
        &self,
        request: Request<SshAccessRequest>,
    ) -> Result<Response<SshAccessResponse>, Status> {
        self.verify_internal_token(&request)?;
        
        let req = request.into_inner();
        debug!(
            "CheckSSHAccess: key_id={}, repo={}, action={}",
            req.key_id, req.repo_path, req.action
        );

        // 解析 key_id (格式: "key-123")
        let key_id: i64 = req
            .key_id
            .strip_prefix("key-")
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| Status::invalid_argument("Invalid key_id format"))?;

        // 获取 SSH 密钥
        let key = sqlx::query_as::<_, (i64, i64, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT id, user_id, expires_at FROM ssh_keys WHERE id = $1"
        )
        .bind(key_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| {
            info!("SSH key not found: {}", key_id);
            Status::not_found("SSH key not found")
        })?;

        let (_, user_id, expires_at) = key;

        // 检查密钥是否过期
        if let Some(exp) = expires_at {
            if exp < chrono::Utc::now() {
                return Ok(Response::new(SshAccessResponse {
                    status: false,
                    message: "SSH key has expired".to_string(),
                    ..Default::default()
                }));
            }
        }

        // 获取用户信息
        let user = sqlx::query_as::<_, (i64, String, bool)>(
            "SELECT id, username, is_active FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::not_found("User not found"))?;

        let (user_id, username, is_active) = user;

        if !is_active {
            return Ok(Response::new(SshAccessResponse {
                status: false,
                message: "User account is deactivated".to_string(),
                ..Default::default()
            }));
        }

        // 检查仓库访问权限
        let (can_access, can_write, project_id, repo_path) = 
            self.check_repo_access(user_id, &req.repo_path, &req.action).await?;

        if !can_access {
            info!("Access denied for user {} on repo {}", username, req.repo_path);
            return Ok(Response::new(SshAccessResponse {
                status: false,
                message: "You don't have access to this repository".to_string(),
                ..Default::default()
            }));
        }

        let needs_write = req.action == "git-receive-pack";
        if needs_write && !can_write {
            info!("Write access denied for user {} on repo {}", username, req.repo_path);
            return Ok(Response::new(SshAccessResponse {
                status: false,
                message: "You don't have write access to this repository".to_string(),
                ..Default::default()
            }));
        }

        // 更新密钥最后使用时间
        let _ = sqlx::query("UPDATE ssh_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(key_id)
            .execute(&self.pool)
            .await;

        info!(
            "SSH access granted for user {} on repo {} (write: {})",
            username, req.repo_path, can_write
        );

        Ok(Response::new(SshAccessResponse {
            status: true,
            message: String::new(),
            user_id,
            username,
            can_write,
            project_id,
            repository_path: repo_path,
            gitlayer_address: self.config.gitlayer_address.clone().unwrap_or_default(),
            repository_status: "active".to_string(),
        }))
    }

    /// 检查 HTTP 访问权限
    /// 
    /// **三层权限交集**：
    /// 最终权限 = Token Scopes ∩ User Role ∩ Project Membership
    /// 
    /// 1. Token Scopes: JWT (full) / PAT (limited) / OAuth2 (limited)
    /// 2. User Role: 用户是否激活
    /// 3. Project Membership: owner/maintainer/developer/reporter/guest/非成员
    async fn check_http_access(
        &self,
        request: Request<HttpAccessRequest>,
    ) -> Result<Response<HttpAccessResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "CheckHTTPAccess: repo={}, action={}",
            req.repo_path, req.action
        );

        let needs_write = req.action == "git-receive-pack";

        // ========================================
        // 第一层：Token 认证（获取 token scopes）
        // ========================================
        let token_info = match req.auth {
            Some(http_access_request::Auth::JwtAuth(jwt)) => {
                match self.authenticate_jwt(&jwt.token).await {
                    Ok(info) => info,
                    Err(e) => {
                        // JWT 认证失败
                        // 对于写操作，直接返回错误
                        if needs_write {
                            return Err(e);
                        }
                        // 对于读操作，回退到匿名访问（公开仓库可以匿名读）
                        debug!("JWT auth failed, falling back to anonymous: {}", e);
                        TokenInfo::anonymous()
                    }
                }
            }
            Some(http_access_request::Auth::BasicAuth(basic)) => {
                // Basic auth 可能是密码、PAT 或 OAuth2 token
                match self.authenticate_basic(&basic.username, &basic.password).await {
                    Ok(info) => info,
                    Err(e) => {
                        // Basic auth 认证失败
                        // 对于写操作，直接返回错误
                        if needs_write {
                            return Err(e);
                        }
                        // 对于读操作，回退到匿名访问（公开仓库可以匿名读）
                        // 这符合 GitLab 的行为：公开仓库忽略错误凭据
                        debug!("Basic auth failed, falling back to anonymous: {}", e);
                        TokenInfo::anonymous()
                    }
                }
            }
            None => {
                // 未认证 - 匿名访问
                TokenInfo::anonymous()
            }
        };

        // 写操作必须认证
        if needs_write && !token_info.is_authenticated() {
            return Ok(Response::new(HttpAccessResponse {
                status: false,
                message: "Authentication required for push".to_string(),
                ..Default::default()
            }));
        }

        // ========================================
        // 第二层：Token Scope 检查
        // ========================================
        if token_info.is_authenticated() {
            if needs_write && !token_info.can_write_repo() {
                warn!(
                    "Token scope insufficient for push: user={}, scopes={:?}",
                    token_info.username, token_info.scopes
                );
                return Ok(Response::new(HttpAccessResponse {
                    status: false,
                    message: "Token scope insufficient for push (need repository:write or write_repository)".to_string(),
                    ..Default::default()
                }));
            }
            
            if !needs_write && !token_info.can_read_repo() {
                warn!(
                    "Token scope insufficient for read: user={}, scopes={:?}",
                    token_info.username, token_info.scopes
                );
                return Ok(Response::new(HttpAccessResponse {
                    status: false,
                    message: "Token scope insufficient for clone/fetch (need repository:read or read_repository)".to_string(),
                    ..Default::default()
                }));
            }
        }

        // ========================================
        // 第三层：项目权限检查（User × Project Membership）
        // ========================================
        let (can_access, can_write, project_id, repo_path) = 
            self.check_repo_access(token_info.user_id, &req.repo_path, &req.action).await?;

        if !can_access {
            return Ok(Response::new(HttpAccessResponse {
                status: false,
                message: "You don't have access to this repository".to_string(),
                ..Default::default()
            }));
        }

        if needs_write && !can_write {
            return Ok(Response::new(HttpAccessResponse {
                status: false,
                message: "You don't have write access to this repository".to_string(),
                ..Default::default()
            }));
        }

        // ========================================
        // 权限检查通过！
        // ========================================
        info!(
            "HTTP access granted: user={} (id={}), repo={}, write={}, scopes={:?}",
            if token_info.is_authenticated() { &token_info.username } else { "anonymous" },
            token_info.user_id,
            req.repo_path,
            can_write,
            token_info.scopes
        );

        Ok(Response::new(HttpAccessResponse {
            status: true,
            message: String::new(),
            user_id: token_info.user_id,
            username: token_info.username,
            can_write,
            project_id,
            repository_path: repo_path,
            gitlayer_address: self.config.gitlayer_address.clone().unwrap_or_default(),
        }))
    }

    /// 通过指纹查找 SSH 密钥
    async fn find_ssh_key(
        &self,
        request: Request<FindSshKeyRequest>,
    ) -> Result<Response<FindSshKeyResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!("FindSSHKey: fingerprint={}", req.fingerprint);

        let result = sqlx::query_as::<_, (i64, i64, String, String, String)>(
            r#"
            SELECT k.id, k.user_id, u.username, k.key_type, k.public_key
            FROM ssh_keys k
            JOIN users u ON k.user_id = u.id
            WHERE k.fingerprint = $1
              AND (k.expires_at IS NULL OR k.expires_at > NOW())
              AND u.is_active = true
            "#,
        )
        .bind(&req.fingerprint)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match result {
            Some((id, user_id, username, key_type, public_key)) => {
                info!("Found SSH key {} for user {}", id, username);
                Ok(Response::new(FindSshKeyResponse {
                    found: true,
                    id,
                    user_id,
                    username,
                    key_type,
                    public_key,
                }))
            }
            None => {
                debug!("SSH key not found for fingerprint: {}", req.fingerprint);
                Ok(Response::new(FindSshKeyResponse {
                    found: false,
                    ..Default::default()
                }))
            }
        }
    }

    /// 通过 ID 获取 SSH 密钥
    async fn get_ssh_key(
        &self,
        request: Request<GetSshKeyRequest>,
    ) -> Result<Response<GetSshKeyResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();

        let result = sqlx::query_as::<_, (i64, i64, String, String, String)>(
            r#"
            SELECT k.id, k.user_id, u.username, k.key_type, k.public_key
            FROM ssh_keys k
            JOIN users u ON k.user_id = u.id
            WHERE k.id = $1
            "#,
        )
        .bind(req.id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match result {
            Some((id, user_id, username, key_type, public_key)) => {
                Ok(Response::new(GetSshKeyResponse {
                    found: true,
                    id,
                    user_id,
                    username,
                    key_type,
                    public_key,
                }))
            }
            None => Ok(Response::new(GetSshKeyResponse {
                found: false,
                ..Default::default()
            })),
        }
    }

    /// Post-receive 通知
    ///
    /// 完整流程：
    /// 1. 解析 CI 配置文件
    /// 2. 创建 pipeline 和 jobs
    /// 3. 返回创建的 pipeline IDs
    async fn notify_post_receive(
        &self,
        request: Request<PostReceiveRequest>,
    ) -> Result<Response<PostReceiveResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        info!(
            "PostReceive notification: user_id={}, repo={}, project_id={}, changes={}",
            req.user_id,
            req.repository,
            req.project_id,
            req.changes.len()
        );

        let mut pipeline_ids = Vec::new();
        let mut total_jobs = 0;

        // 为每个 ref change 触发 CI/CD pipeline
        for change in &req.changes {
            // 跳过删除操作
            if change.new_sha == "0000000000000000000000000000000000000000" {
                debug!("Ref {} deleted, skipping CI", change.ref_name);
                continue;
            }

            // 使用完整的 CI 服务触发 pipeline
            match self
                .ci_service
                .trigger_pipeline(
                    req.project_id,
                    req.user_id,
                    &change.ref_name,
                    &change.new_sha,
                    crate::models::PipelineTriggerType::Push,
                )
                .await
            {
                Ok(Some(result)) => {
                    info!(
                        "Pipeline {} created with {} jobs (status: {:?})",
                        result.pipeline_id, result.jobs_created, result.status
                    );
                    pipeline_ids.push(result.pipeline_id);
                    total_jobs += result.jobs_created;
                }
                Ok(None) => {
                    debug!("No pipeline triggered for ref {}", change.ref_name);
                }
                Err(e) => {
                    warn!("Failed to trigger pipeline for ref {}: {}", change.ref_name, e);
                }
            }
        }

        Ok(Response::new(PostReceiveResponse {
            success: true,
            message: format!(
                "Triggered {} pipelines with {} jobs",
                pipeline_ids.len(),
                total_jobs
            ),
            pipeline_ids,
        }))
    }

    /// 检查 ref 更新权限 (pre-receive hook)
    async fn check_ref_update(
        &self,
        request: Request<RefUpdateRequest>,
    ) -> Result<Response<RefUpdateResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "CheckRefUpdate: user={}, repo={}, ref={}",
            req.user_id, req.repository, req.ref_name
        );

        // 解析仓库路径获取项目 ID
        let parts: Vec<&str> = req.repository.rsplitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(Status::invalid_argument("Invalid repository path"));
        }
        let (project_name, namespace) = (parts[0].trim_end_matches(".git"), parts[1]);
        
        // 查询项目
        let project = sqlx::query_as::<_, (i64, i64)>(
            "SELECT p.id, p.owner_id FROM projects p
             JOIN namespaces n ON p.namespace_id = n.id
             WHERE n.path = $1 AND p.name = $2"
        )
        .bind(namespace)
        .bind(project_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        
        let (project_id, owner_id) = match project {
            Some(p) => p,
            None => return Ok(Response::new(RefUpdateResponse {
                allowed: false,
                message: "Project not found".to_string(),
            })),
        };

        // 只对分支进行保护检查（不检查标签）
        if !req.ref_name.starts_with("refs/heads/") {
            return Ok(Response::new(RefUpdateResponse {
                allowed: true,
                message: String::new(),
            }));
        }
        
        let branch_name = req.ref_name.trim_start_matches("refs/heads/");
        
        // 检查变更类型
        let is_creation = req.change_type == "create" 
            || req.old_sha == "0000000000000000000000000000000000000000";
        let is_deletion = req.change_type == "delete" 
            || req.new_sha == "0000000000000000000000000000000000000000";
        
        // 检测强制推送：当 old_sha 不是 new_sha 的祖先时（非 fast-forward）
        // 创建和删除操作不视为强制推送
        let is_force_push = if is_creation || is_deletion {
            false
        } else {
            // 构建仓库磁盘路径
            let repo_disk_path = format!(
                "{}/{}/{}.git",
                self.config.git_repos_path, namespace, project_name
            );
            // 调用 GitLayer 检查祖先关系
            // 如果检测失败，直接拒绝推送 - 不传播损坏
            self.check_force_push(&repo_disk_path, &req.old_sha, &req.new_sha).await?
        };

        // 查询分支保护规则
        let rules = sqlx::query_as::<_, (String, bool, i32, bool, bool, bool)>(
            "SELECT branch_pattern, require_review, required_reviewers, 
                    require_ci_pass, allow_force_push, allow_deletion
             FROM branch_protection_rules
             WHERE project_id = $1"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        // 检查分支是否匹配任何保护规则
        for (pattern, require_review, required_reviewers, require_ci_pass, allow_force_push, allow_deletion) in rules {
            if !self.branch_matches_pattern(branch_name, &pattern) {
                continue;
            }

            // 找到匹配的保护规则，检查权限
            debug!("Branch {} matches protection pattern {}", branch_name, pattern);

            // 检查用户角色
            let user_role = self.get_user_project_role(req.user_id, project_id, owner_id).await?;
            
            // 只有 owner 和 maintainer 可以推送到受保护分支
            if !matches!(user_role.as_str(), "owner" | "maintainer") {
                return Ok(Response::new(RefUpdateResponse {
                    allowed: false,
                    message: format!(
                        "Branch '{}' is protected. Only maintainers and owners can push.",
                        branch_name
                    ),
                }));
            }

            // 检查删除
            if is_deletion && !allow_deletion {
                return Ok(Response::new(RefUpdateResponse {
                    allowed: false,
                    message: format!(
                        "Branch '{}' is protected. Deletion is not allowed.",
                        branch_name
                    ),
                }));
            }

            // 检查强制推送
            if is_force_push && !allow_force_push {
                return Ok(Response::new(RefUpdateResponse {
                    allowed: false,
                    message: format!(
                        "Branch '{}' is protected. Force push is not allowed.",
                        branch_name
                    ),
                }));
            }

            // require_review 和 require_ci_pass 通常在 MR 合并时检查
            // pre-receive hook 阶段如果不是通过 MR 合并，可以选择拒绝
            // 这里允许 maintainer/owner 直接推送（他们可以绕过 review）
            
            debug!("Protected branch check passed for user role: {}", user_role);
        }

        Ok(Response::new(RefUpdateResponse {
            allowed: true,
            message: String::new(),
        }))
    }

    /// 生成 LFS 认证 token
    async fn generate_lfs_token(
        &self,
        request: Request<LfsTokenRequest>,
    ) -> Result<Response<LfsTokenResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        debug!(
            "GenerateLfsToken: user_id={}, repo={}, operation={}",
            req.user_id, req.repo_path, req.operation
        );

        // 验证用户存在
        let user_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND is_active = true)"
        )
        .bind(req.user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        if !user_exists {
            return Ok(Response::new(LfsTokenResponse {
                success: false,
                message: "User not found or inactive".to_string(),
                ..Default::default()
            }));
        }

        // 如果是 upload 操作，需要验证写权限
        if req.operation == "upload" {
            let (can_access, can_write, _, _) = self.check_repo_access(
                req.user_id,
                &req.repo_path,
                "git-receive-pack"
            ).await?;

            if !can_access {
                return Ok(Response::new(LfsTokenResponse {
                    success: false,
                    message: "You don't have access to this repository".to_string(),
                    ..Default::default()
                }));
            }

            if !can_write {
                return Ok(Response::new(LfsTokenResponse {
                    success: false,
                    message: "You don't have write access to this repository".to_string(),
                    ..Default::default()
                }));
            }
        }

        // 生成 LFS token (JWT)
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde::Serialize;

        #[derive(Serialize)]
        struct LfsClaims {
            sub: String,           // subject: "lfs"
            user_id: i64,
            username: String,
            repo: String,
            operation: String,     // "download" 或 "upload"
            exp: i64,             // 过期时间
            iat: i64,             // 签发时间
        }

        let now = chrono::Utc::now();
        // 使用配置的 LFS 链接过期时间，默认 1800 秒（30分钟）
        let expires_in = self.config.lfs_link_expires.unwrap_or(1800);
        let exp = now + chrono::Duration::seconds(expires_in);

        let claims = LfsClaims {
            sub: "lfs".to_string(),
            user_id: req.user_id,
            username: req.username.clone(),
            repo: req.repo_path.clone(),
            operation: req.operation.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|e| Status::internal(format!("Failed to generate LFS token: {}", e)))?;

        // 构建 LFS href
        let href = format!(
            "{}/{}.git/info/lfs",
            self.config.base_url.trim_end_matches('/'),
            req.repo_path
        );

        info!(
            "Generated LFS token for user {} on repo {} (operation: {})",
            req.user_id, req.repo_path, req.operation
        );

        Ok(Response::new(LfsTokenResponse {
            success: true,
            message: String::new(),
            token,
            href,
            expires_in,
        }))
    }
    
    /// 验证GPG签名
    async fn verify_gpg_signature(
        &self,
        request: Request<VerifyGpgSignatureRequest>,
    ) -> Result<Response<VerifyGpgSignatureResponse>, Status> {
        self.verify_internal_token(&request)?;
        
        let req = request.into_inner();
        debug!(
            "VerifyGpgSignature: commit={}, committer_email={}",
            req.commit_sha, req.committer_email
        );
        
        // 首先检查缓存
        if req.project_id > 0 {
            let cached: Option<(bool, String, Option<String>, Option<i64>, Option<String>)> = sqlx::query_as(
                r#"
                SELECT 
                    (verification_status = 'verified') as valid,
                    verification_status,
                    signer_key_id,
                    signer_user_id,
                    u.username
                FROM gpg_signatures gs
                LEFT JOIN users u ON gs.signer_user_id = u.id
                WHERE gs.project_id = $1 AND gs.commit_sha = $2
                "#
            )
            .bind(req.project_id)
            .bind(&req.commit_sha)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
            
            if let Some((valid, status, key_id, user_id, username)) = cached {
                return Ok(Response::new(VerifyGpgSignatureResponse {
                    valid,
                    status,
                    message: String::new(),
                    key_id: key_id.unwrap_or_default(),
                    signer_user_id: user_id.unwrap_or(0),
                    signer_username: username.unwrap_or_default(),
                }));
            }
        }
        
        // 调用验证逻辑
        let result = crate::handlers::gpg_key::verify_gpg_signature(
            &self.pool,
            &req.signature,
            &req.signed_data,
            &req.committer_email,
        ).await;
        
        match result {
            Ok((valid, status, key_id, user_id, username)) => {
                // 缓存结果
                if req.project_id > 0 {
                    let _ = sqlx::query(
                        r#"
                        INSERT INTO gpg_signatures (
                            project_id, commit_sha, signer_key_id, verification_status,
                            signer_email, signer_user_id
                        )
                        VALUES ($1, $2, $3, $4, $5, $6)
                        ON CONFLICT (project_id, commit_sha) DO UPDATE SET
                            verification_status = EXCLUDED.verification_status,
                            signer_user_id = EXCLUDED.signer_user_id
                        "#
                    )
                    .bind(req.project_id)
                    .bind(&req.commit_sha)
                    .bind(&key_id)
                    .bind(&status)
                    .bind(&req.committer_email)
                    .bind(user_id)
                    .execute(&self.pool)
                    .await;
                }
                
                Ok(Response::new(VerifyGpgSignatureResponse {
                    valid,
                    status,
                    message: String::new(),
                    key_id: key_id.unwrap_or_default(),
                    signer_user_id: user_id.unwrap_or(0),
                    signer_username: username.unwrap_or_default(),
                }))
            }
            Err(e) => {
                warn!("GPG signature verification failed: {}", e);
                Ok(Response::new(VerifyGpgSignatureResponse {
                    valid: false,
                    status: "unknown_key".to_string(),
                    message: e.to_string(),
                    ..Default::default()
                }))
            }
        }
    }
    
    /// 查找GPG密钥
    async fn find_gpg_key(
        &self,
        request: Request<FindGpgKeyRequest>,
    ) -> Result<Response<FindGpgKeyResponse>, Status> {
        self.verify_internal_token(&request)?;
        
        let req = request.into_inner();
        debug!("FindGpgKey: key_id={}", req.key_id);
        
        // 尝试通过主密钥或子密钥查找
        let result: Option<(i64, i64, String, String, String, Vec<String>, bool, bool, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
            r#"
            SELECT 
                g.id,
                g.user_id,
                u.username,
                g.primary_key_id,
                g.fingerprint,
                g.emails,
                g.verified,
                g.revoked,
                g.key_expires_at
            FROM gpg_keys g
            JOIN users u ON g.user_id = u.id
            WHERE g.fingerprint = $1
               OR g.primary_key_id = $1
               OR g.fingerprint LIKE '%' || $1
            "#
        )
        .bind(&req.key_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        
        if let Some((id, user_id, username, primary_key_id, fingerprint, emails, verified, revoked, expires_at)) = result {
            let expired = expires_at.map(|e| e < chrono::Utc::now()).unwrap_or(false);
            
            return Ok(Response::new(FindGpgKeyResponse {
                found: true,
                id,
                user_id,
                username,
                primary_key_id,
                fingerprint,
                emails,
                verified,
                revoked,
                expired,
            }));
        }
        
        // 尝试通过子密钥查找
        let subkey_result: Option<(i64, String)> = sqlx::query_as(
            r#"
            SELECT gpg_key_id, fingerprint
            FROM gpg_key_subkeys
            WHERE fingerprint = $1
               OR key_id = $1
               OR fingerprint LIKE '%' || $1
            "#
        )
        .bind(&req.key_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
        
        if let Some((gpg_key_id, _)) = subkey_result {
            // 获取主密钥信息
            let result: Option<(i64, i64, String, String, String, Vec<String>, bool, bool, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
                r#"
                SELECT 
                    g.id,
                    g.user_id,
                    u.username,
                    g.primary_key_id,
                    g.fingerprint,
                    g.emails,
                    g.verified,
                    g.revoked,
                    g.key_expires_at
                FROM gpg_keys g
                JOIN users u ON g.user_id = u.id
                WHERE g.id = $1
                "#
            )
            .bind(gpg_key_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;
            
            if let Some((id, user_id, username, primary_key_id, fingerprint, emails, verified, revoked, expires_at)) = result {
                let expired = expires_at.map(|e| e < chrono::Utc::now()).unwrap_or(false);
                
                return Ok(Response::new(FindGpgKeyResponse {
                    found: true,
                    id,
                    user_id,
                    username,
                    primary_key_id,
                    fingerprint,
                    emails,
                    verified,
                    revoked,
                    expired,
                }));
            }
        }
        
        Ok(Response::new(FindGpgKeyResponse {
            found: false,
            ..Default::default()
        }))
    }
}

impl AuthServiceImpl {
    /// 检查仓库访问权限
    async fn check_repo_access(
        &self,
        user_id: i64,
        repo_path: &str,
        _action: &str,
    ) -> Result<(bool, bool, i64, String), Status> {
        // 解析仓库路径 (格式: namespace/project.git)
        let parts: Vec<&str> = repo_path.rsplitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(Status::invalid_argument("Invalid repository path format"));
        }
        let (project_with_git, namespace) = (parts[0], parts[1]);
        // 去掉 .git 后缀
        let project_name = project_with_git.strip_suffix(".git").unwrap_or(project_with_git);

        // 查找项目
        let project = sqlx::query_as::<_, (i64, String, i64, String)>(
            r#"
            SELECT p.id, p.name, p.owner_id, p.visibility::text
            FROM projects p
            JOIN namespaces n ON p.namespace_id = n.id
            WHERE n.path = $1 AND p.name = $2
            "#,
        )
        .bind(namespace)
        .bind(project_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let (project_id, project_name, owner_id, visibility) = match project {
            Some(p) => p,
            None => {
                return Ok((false, false, 0, String::new()));
            }
        };

        let repo_disk_path = format!(
            "{}/{}/{}.git",
            self.config.git_repos_path, namespace, project_name
        );

        // 未认证用户（匿名）只能访问公开仓库
        // 注意：internal 可见性需要登录用户，不允许匿名访问
        if user_id == 0 {
            let can_access = visibility == "public";
            return Ok((can_access, false, project_id, repo_disk_path));
        }

        // Owner 总是有完整权限
        if user_id == owner_id {
            return Ok((true, true, project_id, repo_disk_path));
        }

        // 检查项目成员权限
        let membership = sqlx::query_as::<_, (String,)>(
            "SELECT role::text FROM project_members WHERE project_id = $1 AND user_id = $2"
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match membership {
            Some((role,)) => {
                let can_write = matches!(role.as_str(), "owner" | "maintainer" | "developer");
                Ok((true, can_write, project_id, repo_disk_path))
            }
            None => {
                // 无成员关系，检查可见性
                // public: 任何人可读
                // internal: 仅登录用户可读（user_id != 0 已在上面检查）
                // private: 仅成员可访问
                let can_access = visibility == "public" || visibility == "internal";
                Ok((can_access, false, project_id, repo_disk_path))
            }
        }
    }

    // ========================================
    // 统一认证方法（返回 TokenInfo）
    // ========================================

    /// 验证 JWT token 并返回 TokenInfo
    /// 
    /// JWT 是通过 /auth/login 获取的，代表用户会话，享有完全访问权限
    async fn authenticate_jwt(&self, token: &str) -> Result<TokenInfo, Status> {
        use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
        
        #[derive(serde::Deserialize)]
        struct Claims {
            sub: String,
            exp: usize,
        }

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|e| Status::unauthenticated(format!("Invalid JWT: {}", e)))?;

        let user_id: i64 = token_data
            .claims
            .sub
            .parse()
            .map_err(|_| Status::unauthenticated("Invalid user ID in token"))?;

        let username = sqlx::query_scalar::<_, String>(
            "SELECT username FROM users WHERE id = $1 AND is_active = true"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::unauthenticated("User not found or inactive"))?;

        // JWT = 完全访问权限
        Ok(TokenInfo::full_access(user_id, username))
    }

    /// 验证 Basic Auth 并返回 TokenInfo
    /// 
    /// Basic Auth 可能是：
    /// 1. PAT (gitfox-pat_ 前缀) → 受限访问
    /// 2. OAuth2 token → 受限访问
    /// 3. 用户名/密码 → 完全访问
    async fn authenticate_basic(&self, username: &str, password: &str) -> Result<TokenInfo, Status> {
        // 1. 首先尝试作为 PAT 验证
        if let Some(token_info) = self.authenticate_pat(password).await? {
            return Ok(token_info);
        }

        // 2. 尝试作为 OAuth2 token 验证
        if let Some(token_info) = self.authenticate_oauth(password).await? {
            return Ok(token_info);
        }

        // 3. 最后尝试密码验证
        let user = sqlx::query_as::<_, (i64, String, String)>(
            "SELECT id, username, password_hash FROM users WHERE username = $1 AND is_active = true"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::unauthenticated("Invalid credentials"))?;

        let (user_id, username, password_hash) = user;

        if bcrypt::verify(password, &password_hash).unwrap_or(false) {
            // 密码登录 = 完全访问权限
            Ok(TokenInfo::full_access(user_id, username))
        } else {
            Err(Status::unauthenticated("Invalid credentials"))
        }
    }

    /// 验证 Personal Access Token 并返回 TokenInfo
    /// 
    /// PAT 格式: gitfox-pat_xxxx 或 glpat-xxxx (兼容旧格式)
    async fn authenticate_pat(&self, token: &str) -> Result<Option<TokenInfo>, Status> {
        // PAT 格式检查
        if !token.starts_with("gitfox-pat_") && !token.starts_with("glpat-") {
            return Ok(None);
        }

        // 计算 token hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        // 查询 PAT 及其 scopes
        let result = sqlx::query_as::<_, (i64, String, Vec<String>)>(
            r#"
            SELECT p.user_id, u.username, p.scopes
            FROM personal_access_tokens p
            JOIN users u ON p.user_id = u.id
            WHERE p.token_hash = $1
              AND p.is_active = true
              AND (p.expires_at IS NULL OR p.expires_at > NOW())
              AND u.is_active = true
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match result {
            Some((user_id, username, scopes)) => {
                Ok(Some(TokenInfo::limited_access(user_id, username, scopes)))
            }
            None => Ok(None),
        }
    }

    /// 验证 OAuth2 access token 并返回 TokenInfo
    /// 
    /// OAuth2 token 存储在 oauth_access_tokens 表中
    async fn authenticate_oauth(&self, token: &str) -> Result<Option<TokenInfo>, Status> {
        // OAuth2 token 通常是 URL-safe base64 编码的随机字符串
        // 我们通过 hash 查找来验证
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        let result = sqlx::query_as::<_, (i64, String, Vec<String>)>(
            r#"
            SELECT t.user_id, u.username, t.scopes
            FROM oauth_access_tokens t
            JOIN users u ON t.user_id = u.id
            WHERE t.token_hash = $1
              AND t.revoked_at IS NULL
              AND (t.expires_at IS NULL OR t.expires_at > NOW())
              AND u.is_active = true
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        match result {
            Some((user_id, username, scopes)) => {
                Ok(Some(TokenInfo::limited_access(user_id, username, scopes)))
            }
            None => Ok(None),
        }
    }

    // ========================================
    // 遗留方法（为兼容性保留）
    // ========================================

    /// 验证 JWT token (旧实现，为兼容性保留)
    async fn verify_jwt(&self, token: &str) -> Result<(i64, String), Status> {
        use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
        
        #[derive(serde::Deserialize)]
        struct Claims {
            sub: String,
            exp: usize,
        }

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|e| Status::unauthenticated(format!("Invalid JWT: {}", e)))?;

        let user_id: i64 = token_data
            .claims
            .sub
            .parse()
            .map_err(|_| Status::unauthenticated("Invalid user ID in token"))?;

        let username = sqlx::query_scalar::<_, String>(
            "SELECT username FROM users WHERE id = $1 AND is_active = true"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::unauthenticated("User not found"))?;

        Ok((user_id, username))
    }

    /// 验证 Basic Auth (密码或 PAT)
    async fn verify_basic_auth(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(i64, String), Status> {
        // 首先尝试作为 PAT 验证
        if let Some((user_id, username)) = self.verify_pat(password).await? {
            return Ok((user_id, username));
        }

        // 然后尝试密码验证
        let user = sqlx::query_as::<_, (i64, String, String)>(
            "SELECT id, username, password_hash FROM users WHERE username = $1 AND is_active = true"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| Status::unauthenticated("Invalid credentials"))?;

        let (user_id, username, password_hash) = user;

        if bcrypt::verify(password, &password_hash).unwrap_or(false) {
            Ok((user_id, username))
        } else {
            Err(Status::unauthenticated("Invalid credentials"))
        }
    }

    /// 验证 Personal Access Token
    async fn verify_pat(&self, token: &str) -> Result<Option<(i64, String)>, Status> {
        // PAT 格式: glpat-xxxx
        if !token.starts_with("glpat-") {
            return Ok(None);
        }

        // 计算 token hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());

        let result = sqlx::query_as::<_, (i64, String)>(
            r#"
            SELECT p.user_id, u.username
            FROM personal_access_tokens p
            JOIN users u ON p.user_id = u.id
            WHERE p.token_hash = $1
              AND p.is_active = true
              AND (p.expires_at IS NULL OR p.expires_at > NOW())
              AND u.is_active = true
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(result)
    }

    // ========================================
    // 分支保护辅助方法
    // ========================================

    /// 检测是否为强制推送（non-fast-forward push）
    /// 
    /// 强制推送定义：新的 commit 不是旧 commit 的后代
    /// 即 old_sha 不是 new_sha 的祖先
    /// 
    /// # Arguments
    /// * `repo_path` - 仓库磁盘路径（如 "./repos/namespace/project.git"）
    /// * `old_sha` - 更新前的 commit SHA
    /// * `new_sha` - 更新后的 commit SHA
    /// 
    /// # Returns
    /// * `Ok(true)` - 是强制推送
    /// * `Ok(false)` - 不是强制推送（fast-forward）
    /// * `Err` - 检测失败
    async fn check_force_push(
        &self,
        repo_path: &str,
        old_sha: &str,
        new_sha: &str,
    ) -> Result<bool, Status> {
        // 获取 GitLayer 地址
        let gitlayer_addr = self.config.gitlayer_address.clone()
            .ok_or_else(|| Status::internal("GitLayer address not configured"))?;
        
        // 连接 GitLayer CommitService
        let mut client = CommitServiceClient::connect(gitlayer_addr)
            .await
            .map_err(|e| Status::internal(format!("Failed to connect to GitLayer: {}", e)))?;
        
        // 检查 old_sha 是否是 new_sha 的祖先
        // 如果 old_sha 是 new_sha 的祖先，则这是一个 fast-forward push，不是强制推送
        // 如果 old_sha 不是 new_sha 的祖先，则这是一个 non-fast-forward (force) push
        let request = IsAncestorRequest {
            repository: Some(Repository {
                storage_path: repo_path.to_string(),
                relative_path: String::new(),  // storage_path 已包含完整路径
            }),
            ancestor: old_sha.to_string(),
            descendant: new_sha.to_string(),
        };
        
        let response = client.is_ancestor(request)
            .await
            .map_err(|e| Status::internal(format!("GitLayer IsAncestor RPC failed: {}", e)))?;
        
        // 如果 old_sha 不是 new_sha 的祖先，则是强制推送
        let is_force = !response.into_inner().is_ancestor;
        
        if is_force {
            debug!("Detected force push: {} is not an ancestor of {}", old_sha, new_sha);
        }
        
        Ok(is_force)
    }

    /// 检查分支名是否匹配保护模式
    /// 支持 glob 模式：* 匹配任意字符，? 匹配单个字符
    fn branch_matches_pattern(&self, branch: &str, pattern: &str) -> bool {
        // 特殊情况：精确匹配
        if !pattern.contains('*') && !pattern.contains('?') {
            return branch == pattern;
        }

        // 将 glob 模式转换为正则表达式
        let regex_pattern = pattern
            .replace('.', r"\.")
            .replace('*', ".*")
            .replace('?', ".");
        
        let regex_pattern = format!("^{}$", regex_pattern);
        
        match regex::Regex::new(&regex_pattern) {
            Ok(re) => re.is_match(branch),
            Err(_) => branch == pattern, // 回退到精确匹配
        }
    }

    /// 获取用户在项目中的角色
    async fn get_user_project_role(
        &self,
        user_id: i64,
        project_id: i64,
        owner_id: i64,
    ) -> Result<String, Status> {
        // 如果是项目所有者
        if user_id == owner_id {
            return Ok("owner".to_string());
        }

        // 查询项目成员角色
        let role = sqlx::query_scalar::<_, String>(
            "SELECT role::text FROM project_members WHERE project_id = $1 AND user_id = $2"
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(role.unwrap_or_else(|| "none".to_string()))
    }
}
