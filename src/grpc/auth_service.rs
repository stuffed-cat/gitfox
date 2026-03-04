//! gRPC Auth Service implementation
//! 
//! 主应用作为 gRPC 服务端，为 Workhorse 和 Shell 提供权限认证服务。
//! 这符合 GitLab 的架构模式：主应用只负责权限管理，不直接操作 Git。

use tonic::{Request, Response, Status};
use sqlx::PgPool;
use log::{debug, info, warn};
use std::sync::Arc;

use crate::config::Config;

// 导入生成的 proto 代码
pub mod auth_proto {
    tonic::include_proto!("gitfox.auth");
}

use auth_proto::auth_service_server::{AuthService, AuthServiceServer};
use auth_proto::*;

/// Auth gRPC 服务实现
pub struct AuthServiceImpl {
    pool: PgPool,
    config: Arc<Config>,
}

impl AuthServiceImpl {
    pub fn new(pool: PgPool, config: Arc<Config>) -> Self {
        Self { pool, config }
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

        // 验证用户身份
        let (user_id, username) = match req.auth {
            Some(http_access_request::Auth::JwtAuth(jwt)) => {
                self.verify_jwt(&jwt.token).await?
            }
            Some(http_access_request::Auth::BasicAuth(basic)) => {
                self.verify_basic_auth(&basic.username, &basic.password).await?
            }
            None => {
                // 未认证 - 可能是公开仓库的读取
                (0, String::new())
            }
        };

        // 检查仓库访问权限
        let (can_access, can_write, project_id, repo_path) = 
            self.check_repo_access(user_id, &req.repo_path, &req.action).await?;

        let needs_write = req.action == "git-receive-pack";

        // 写操作必须认证
        if needs_write && user_id == 0 {
            return Ok(Response::new(HttpAccessResponse {
                status: false,
                message: "Authentication required for push".to_string(),
                ..Default::default()
            }));
        }

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

        info!(
            "HTTP access granted for user {} on repo {} (write: {})",
            if user_id == 0 { "anonymous" } else { &username },
            req.repo_path,
            can_write
        );

        Ok(Response::new(HttpAccessResponse {
            status: true,
            message: String::new(),
            user_id,
            username,
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
    async fn notify_post_receive(
        &self,
        request: Request<PostReceiveRequest>,
    ) -> Result<Response<PostReceiveResponse>, Status> {
        self.verify_internal_token(&request)?;

        let req = request.into_inner();
        info!(
            "PostReceive notification: user_id={}, repo={}, changes={}",
            req.user_id,
            req.repository,
            req.changes.len()
        );

        let mut pipeline_ids = Vec::new();

        // 触发 CI/CD pipeline
        for change in &req.changes {
            if let Ok(Some(pipeline_id)) = self
                .trigger_pipeline(req.user_id, req.project_id, &change.ref_name, &change.new_sha)
                .await
            {
                pipeline_ids.push(pipeline_id);
            }
        }

        Ok(Response::new(PostReceiveResponse {
            success: true,
            message: format!("Triggered {} pipelines", pipeline_ids.len()),
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

        // TODO: 实现分支保护规则检查
        // 目前默认允许所有更新
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
}

impl AuthServiceImpl {
    /// 检查仓库访问权限
    async fn check_repo_access(
        &self,
        user_id: i64,
        repo_path: &str,
        _action: &str,
    ) -> Result<(bool, bool, i64, String), Status> {
        // 解析仓库路径
        let parts: Vec<&str> = repo_path.rsplitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(Status::invalid_argument("Invalid repository path format"));
        }
        let (project_name, namespace) = (parts[0], parts[1]);

        // 查找项目
        let project = sqlx::query_as::<_, (i64, String, i64, String)>(
            r#"
            SELECT p.id, p.name, p.owner_id, p.visibility::text
            FROM projects p
            JOIN namespaces n ON p.namespace_id = n.id
            WHERE LOWER(n.path) = LOWER($1) AND LOWER(p.name) = LOWER($2)
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

        // 未认证用户只能访问公开仓库
        if user_id == 0 {
            let can_access = visibility == "public" || visibility == "internal";
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
                let can_access = visibility == "public" || visibility == "internal";
                Ok((can_access, false, project_id, repo_disk_path))
            }
        }
    }

    /// 验证 JWT token
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

    /// 触发 CI/CD Pipeline
    /// 
    /// 注意：这是一个简化实现，只创建 pipeline 记录。
    /// 实际的 CI 配置解析和 job 创建由 pipeline 处理服务（如 handlers/internal.rs 中的 post_receive）完成。
    /// 这避免了在 gRPC 服务中直接使用 git2 库。
    async fn trigger_pipeline(
        &self,
        user_id: i64,
        project_id: i64,
        ref_name: &str,
        sha: &str,
    ) -> Result<Option<i64>, Status> {
        // 只在分支推送时触发（不触发 tag）
        if !ref_name.starts_with("refs/heads/") {
            return Ok(None);
        }

        let branch_name = ref_name.strip_prefix("refs/heads/").unwrap_or(ref_name);

        // 创建 pipeline 记录（状态为 created，等待后续处理）
        // 实际的 CI 配置解析和 job 创建由其他服务完成
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO pipelines (project_id, ref, sha, status, trigger_type, triggered_by)
            VALUES ($1, $2, $3, 'created', 'push', $4)
            RETURNING id
            "#,
        )
        .bind(project_id)
        .bind(branch_name)
        .bind(sha)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(format!("Failed to create pipeline: {}", e)))?;

        if let Some(pipeline_id) = result {
            info!("Created pipeline {} for project {} (ref: {})", pipeline_id, project_id, branch_name);
        }

        Ok(result)
    }
}
