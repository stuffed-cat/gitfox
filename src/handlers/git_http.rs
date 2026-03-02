// Git HTTP Smart Protocol Handler
// Implements Git's HTTP Smart Protocol for clone, push, pull operations
// Reference: https://git-scm.com/book/en/v2/Git-Internals-Transfer-Protocols

use actix_web::{web, HttpRequest, HttpResponse};
use base64::Engine;
use bcrypt::verify;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::AsyncWriteExt;
use log::{info, error};


use crate::config::AppConfig;
use crate::error::AppError;
use crate::middleware::auth::OptionalAuth;
use crate::models::User;

/// 验证 Git Basic Auth 认证
/// Git 客户端发送 "Authorization: Basic base64(username:password)"
async fn verify_git_basic_auth(
    req: &HttpRequest,
    pool: &sqlx::PgPool,
) -> Option<(i64, String)> {
    let auth_header = req.headers().get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    // 检查是否是 Basic Auth
    if !auth_str.starts_with("Basic ") {
        return None;
    }
    
    let encoded = &auth_str[6..];
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let credentials = String::from_utf8(decoded).ok()?;
    
    // 格式: "username:password"
    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let username = parts[0];
    let password = parts[1];
    
    info!("Git Basic Auth attempt for user: {}", username);
    
    // 从数据库查询用户
    let user: Option<User> = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE username = $1 AND is_active = true"
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .ok()?;
    
    let user = user?;
    
    // 验证密码
    if verify(password, &user.password_hash).ok()? {
        info!("Git Basic Auth successful for user: {}", username);
        Some((user.id, user.username))
    } else {
        info!("Git Basic Auth failed: invalid password for user: {}", username);
        None
    }
}

/// 从完整路径中解析出 namespace 和 project
/// 例如：gitfox/mirror/redis -> (gitfox/mirror, redis)
///      user/project -> (user, project)
fn parse_git_path(full_path: &str) -> Result<(String, String), AppError> {
    let full_path = full_path.trim_end_matches(".git");
    
    match full_path.rfind('/') {
        Some(pos) => {
            let namespace = &full_path[..pos];
            let project = &full_path[pos + 1..];
            if namespace.is_empty() || project.is_empty() {
                Err(AppError::bad_request("Invalid git path"))
            } else {
                Ok((namespace.to_string(), project.to_string()))
            }
        }
        None => Err(AppError::bad_request("Invalid git path: missing namespace")),
    }
}

/// GET /{path:.*}.git/info/refs?service=git-upload-pack
/// GET /{path:.*}.git/info/refs?service=git-receive-pack
pub async fn get_info_refs(
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<InfoRefsQuery>,
    pool: web::Data<sqlx::PgPool>,
    config: web::Data<AppConfig>,
    auth: OptionalAuth,
) -> Result<HttpResponse, AppError> {
    let full_path = path.into_inner();
    let (namespace, project_name) = parse_git_path(&full_path)?;
    
    info!("Git info/refs request: namespace={}, project={}, path={}", namespace, project_name, req.path());
    
    // Validate service parameter
    let service = match query.service.as_deref() {
        Some("git-upload-pack") => "git-upload-pack",
        Some("git-receive-pack") => "git-receive-pack",
        _ => return Ok(HttpResponse::Forbidden().body("Invalid service")),
    };
    
    // Check project access
    let project = get_project_by_path(&pool, &namespace, &project_name).await?;
    
    // For git-receive-pack (push), require authentication and write permission
    if service == "git-receive-pack" {
        // 获取用户ID
        let user_id = match auth.user_id() {
            Some(id) => id,
            None => {
                // 尝试 Basic Auth
                match verify_git_basic_auth(&req, &pool).await {
                    Some((id, _)) => id,
                    None => {
                        return Ok(HttpResponse::Unauthorized()
                            .append_header(("WWW-Authenticate", "Basic realm=\"Git\""))
                            .body("Authentication required"));
                    }
                }
            }
        };

        // 检查用户是否有写权限
        if !check_write_permission(&pool, user_id, project.id, project.owner_id).await? {
            return Ok(HttpResponse::Forbidden()
                .body("You don't have write access to this repository"));
        }
    }
    
    let repo_path = format!("{}/{}/{}.git", config.git_repos_path, namespace, project.name);
    
    // Run git command
    let output = Command::new(service)
        .arg("--stateless-rpc")
        .arg("--advertise-refs")
        .arg(&repo_path)
        .output()
        .await
        .map_err(|e| AppError::internal(format!("Git command failed: {}", e)))?;
    
    if !output.status.success() {
        return Err(AppError::internal("Git command failed"));
    }
    
    // Build response with proper Git protocol headers
    let content_type = format!("application/x-{}-advertisement", service);
    let mut body = format!("# service={}\n", service);
    body = pkt_line(&body);
    body.push_str("0000"); // flush-pkt
    body.push_str(&String::from_utf8_lossy(&output.stdout));
    
    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .append_header(("Cache-Control", "no-cache"))
        .body(body))
}

/// POST /{path:.*}.git/git-upload-pack
pub async fn git_upload_pack(
    path: web::Path<String>,
    body: web::Bytes,
    pool: web::Data<sqlx::PgPool>,
    config: web::Data<AppConfig>,
    _auth: OptionalAuth,
) -> Result<HttpResponse, AppError> {
    let full_path = path.into_inner();
    let (namespace, project_name) = parse_git_path(&full_path)?;
    
    let project = get_project_by_path(&pool, &namespace, &project_name).await?;
    
    let repo_path = format!("{}/{}/{}.git", config.git_repos_path, namespace, project.name);
    
    // Spawn git-upload-pack process
    let mut child = Command::new("git-upload-pack")
        .arg("--stateless-rpc")
        .arg(&repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::internal(format!("Failed to spawn git: {}", e)))?;
    
    // Write request body to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&body).await
            .map_err(|e| AppError::internal(format!("Failed to write to git: {}", e)))?;
    }
    
    // Read output
    let output = child.wait_with_output().await
        .map_err(|e| AppError::internal(format!("Git command failed: {}", e)))?;
    
    Ok(HttpResponse::Ok()
        .content_type("application/x-git-upload-pack-result")
        .append_header(("Cache-Control", "no-cache"))
        .body(output.stdout))
}

/// POST /{path:.*}.git/git-receive-pack
pub async fn git_receive_pack(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Bytes,
    pool: web::Data<sqlx::PgPool>,
    config: web::Data<AppConfig>,
    auth: OptionalAuth,
) -> Result<HttpResponse, AppError> {
    // Require authentication for push
    // 首先尝试 Bearer Token, 然后尝试 Git Basic Auth
    let _user_id = match auth.user_id() {
        Some(id) => id,
        None => {
            // 尝试 Basic Auth
            match verify_git_basic_auth(&req, &pool).await {
                Some((id, _username)) => id,
                None => {
                    return Ok(HttpResponse::Unauthorized()
                        .append_header(("WWW-Authenticate", "Basic realm=\"Git\""))
                        .body("Authentication required for push"));
                }
            }
        }
    };
    
    let full_path = path.into_inner();
    let (namespace, project_name) = parse_git_path(&full_path)?;
    
    let project = get_project_by_path(&pool, &namespace, &project_name).await?;
    
    // 检查用户是否有写权限
    if !check_write_permission(&pool, _user_id, project.id, project.owner_id).await? {
        return Ok(HttpResponse::Forbidden()
            .body("You don't have write access to this repository"));
    }
    
    let repo_path = format!("{}/{}/{}.git", config.git_repos_path, namespace, project.name);
    
    // Spawn git-receive-pack process
    let mut child = Command::new("git-receive-pack")
        .arg("--stateless-rpc")
        .arg(&repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::internal(format!("Failed to spawn git: {}", e)))?;
    
    // Write request body to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&body).await
            .map_err(|e| AppError::internal(format!("Failed to write to git: {}", e)))?;
    }
    
    // Read output
    let output = child.wait_with_output().await
        .map_err(|e| AppError::internal(format!("Git command failed: {}", e)))?;
    
    // TODO: Trigger webhooks, CI/CD pipelines after successful push
    
    Ok(HttpResponse::Ok()
        .content_type("application/x-git-receive-pack-result")
        .append_header(("Cache-Control", "no-cache"))
        .body(output.stdout))
}

#[derive(Debug, serde::Deserialize)]
pub struct InfoRefsQuery {
    pub service: Option<String>,
}

// Helper function to get project by namespace/project path
// 支持用户 (user) 和组 (group) 两种命名空间
async fn get_project_by_path(
    pool: &sqlx::PgPool,
    namespace: &str,
    project_name: &str,
) -> Result<crate::models::project::Project, AppError> {
    info!("Looking for project: namespace='{}', project='{}'", namespace, project_name);
    
    // 通过 namespaces 表查找，支持用户和组
    let project = sqlx::query_as::<_, crate::models::project::Project>(
        r#"
        SELECT p.* FROM projects p
        JOIN namespaces n ON p.namespace_id = n.id
        WHERE LOWER(n.path) = LOWER($1) AND LOWER(p.name) = LOWER($2)
        "#
    )
    .bind(namespace)
    .bind(project_name)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;
    
    if let Some(project) = project {
        info!("Found project: id={}", project.id);
        return Ok(project);
    }
    
    error!("Project not found: namespace='{}', project='{}'", namespace, project_name);
    Err(AppError::not_found(format!("Project '{}/{}' not found", namespace, project_name)))
}

// Encode a string in Git pkt-line format
fn pkt_line(data: &str) -> String {
    let len = data.len() + 4; // 4 bytes for length prefix
    format!("{:04x}{}", len, data)
}

/// 检查用户是否有项目的写权限
/// 只有 owner、maintainer、developer 角色可以写入
async fn check_write_permission(
    pool: &sqlx::PgPool,
    user_id: i64,
    project_id: i64,
    owner_id: i64,
) -> Result<bool, AppError> {
    // Owner 总是有写权限
    if user_id == owner_id {
        return Ok(true);
    }
    
    // 检查项目成员角色
    let role = sqlx::query_scalar::<_, String>(
        r#"SELECT role::text FROM project_members WHERE project_id = $1 AND user_id = $2"#
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;
    
    match role {
        Some(role) => {
            // 只有 owner、maintainer、developer 有写权限
            Ok(matches!(role.as_str(), "owner" | "maintainer" | "developer"))
        }
        None => Ok(false),
    }
}

pub fn configure_git_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Git Smart HTTP Protocol routes
        // 使用 {path:.*} 来匹配多段 namespace，如 gitfox/mirror/redis
        .route(
            "/git/{path:.*}.git/info/refs",
            web::get().to(get_info_refs)
        )
        .route(
            "/git/{path:.*}.git/git-upload-pack",
            web::post().to(git_upload_pack)
        )
        .route(
            "/git/{path:.*}.git/git-receive-pack",
            web::post().to(git_receive_pack)
        )
        // Alternative paths without /git prefix
        .route(
            "/{path:.*}.git/info/refs",
            web::get().to(get_info_refs)
        )
        .route(
            "/{path:.*}.git/git-upload-pack",
            web::post().to(git_upload_pack)
        )
        .route(
            "/{path:.*}.git/git-receive-pack",
            web::post().to(git_receive_pack)
        );
}
