// Git HTTP Smart Protocol Handler
// Implements Git's HTTP Smart Protocol for clone, push, pull operations
// Reference: https://git-scm.com/book/en/v2/Git-Internals-Transfer-Protocols

use actix_web::{web, HttpRequest, HttpResponse};
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::AsyncWriteExt;
use log::{info, error};

use crate::config::AppConfig;
use crate::error::AppError;
use crate::middleware::auth::OptionalAuth;

/// GET /{namespace}/{project}.git/info/refs?service=git-upload-pack
/// GET /{namespace}/{project}.git/info/refs?service=git-receive-pack
pub async fn get_info_refs(
    req: HttpRequest,
    path: web::Path<(String, String)>,
    query: web::Query<InfoRefsQuery>,
    pool: web::Data<sqlx::PgPool>,
    config: web::Data<AppConfig>,
    auth: OptionalAuth,
) -> Result<HttpResponse, AppError> {
    let (namespace, project_name) = path.into_inner();
    let project_name = project_name.trim_end_matches(".git");
    
    info!("Git info/refs request: namespace={}, project={}, path={}", namespace, project_name, req.path());
    
    // Validate service parameter
    let service = match query.service.as_deref() {
        Some("git-upload-pack") => "git-upload-pack",
        Some("git-receive-pack") => "git-receive-pack",
        _ => return Ok(HttpResponse::Forbidden().body("Invalid service")),
    };
    
    // Check project access
    let project = get_project_by_path(&pool, &namespace, project_name).await?;
    
    // For git-receive-pack (push), require authentication
    if service == "git-receive-pack" {
        if auth.user_id().is_none() {
            return Ok(HttpResponse::Unauthorized()
                .append_header(("WWW-Authenticate", "Basic realm=\"Git\""))
                .body("Authentication required"));
        }
        // TODO: Check write permission
    }
    
    let repo_path = format!("{}/{}.git", config.git_repos_path, project.slug);
    
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

/// POST /{namespace}/{project}.git/git-upload-pack
pub async fn git_upload_pack(
    path: web::Path<(String, String)>,
    body: web::Bytes,
    pool: web::Data<sqlx::PgPool>,
    config: web::Data<AppConfig>,
    _auth: OptionalAuth,
) -> Result<HttpResponse, AppError> {
    let (namespace, project_name) = path.into_inner();
    let project_name = project_name.trim_end_matches(".git");
    
    let project = get_project_by_path(&pool, &namespace, project_name).await?;
    
    let repo_path = format!("{}/{}.git", config.git_repos_path, project.slug);
    
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

/// POST /{namespace}/{project}.git/git-receive-pack
pub async fn git_receive_pack(
    path: web::Path<(String, String)>,
    body: web::Bytes,
    pool: web::Data<sqlx::PgPool>,
    config: web::Data<AppConfig>,
    auth: OptionalAuth,
) -> Result<HttpResponse, AppError> {
    // Require authentication for push
    let _user_id = auth.user_id().ok_or_else(|| {
        AppError::unauthorized("Authentication required for push")
    })?;
    
    let (namespace, project_name) = path.into_inner();
    let project_name = project_name.trim_end_matches(".git");
    
    let project = get_project_by_path(&pool, &namespace, project_name).await?;
    
    // TODO: Check write permission for user_id on project
    
    let repo_path = format!("{}/{}.git", config.git_repos_path, project.slug);
    
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
async fn get_project_by_path(
    pool: &sqlx::PgPool,
    namespace: &str,
    project_name: &str,
) -> Result<crate::models::project::Project, AppError> {
    info!("Looking for project: namespace='{}', project='{}'", namespace, project_name);
    
    // First try to find by owner username + project slug
    let project = sqlx::query_as::<_, crate::models::project::Project>(
        r#"
        SELECT p.* FROM projects p
        JOIN users u ON p.owner_id = u.id
        WHERE LOWER(u.username) = LOWER($1) AND LOWER(p.slug) = LOWER($2)
        "#
    )
    .bind(namespace)
    .bind(project_name)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;
    
    if let Some(project) = project {
        info!("Found project by username/slug: id={}", project.id);
        return Ok(project);
    }
    
    // Also try matching by project name (not just slug)
    let project = sqlx::query_as::<_, crate::models::project::Project>(
        r#"
        SELECT p.* FROM projects p
        JOIN users u ON p.owner_id = u.id
        WHERE LOWER(u.username) = LOWER($1) AND LOWER(p.name) = LOWER($2)
        "#
    )
    .bind(namespace)
    .bind(project_name)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)?;
    
    if let Some(project) = project {
        info!("Found project by username/name: id={}", project.id);
        return Ok(project);
    }
    
    // TODO: Try finding by group path + project slug
    error!("Project not found: namespace='{}', project='{}'", namespace, project_name);
    Err(AppError::not_found(format!("Project '{}/{}' not found", namespace, project_name)))
}

// Encode a string in Git pkt-line format
fn pkt_line(data: &str) -> String {
    let len = data.len() + 4; // 4 bytes for length prefix
    format!("{:04x}{}", len, data)
}

pub fn configure_git_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Git Smart HTTP Protocol routes
        .route(
            "/git/{namespace}/{project}.git/info/refs",
            web::get().to(get_info_refs)
        )
        .route(
            "/git/{namespace}/{project}.git/git-upload-pack",
            web::post().to(git_upload_pack)
        )
        .route(
            "/git/{namespace}/{project}.git/git-receive-pack",
            web::post().to(git_receive_pack)
        )
        // Alternative paths without /git prefix
        .route(
            "/{namespace}/{project}.git/info/refs",
            web::get().to(get_info_refs)
        )
        .route(
            "/{namespace}/{project}.git/git-upload-pack",
            web::post().to(git_upload_pack)
        )
        .route(
            "/{namespace}/{project}.git/git-receive-pack",
            web::post().to(git_receive_pack)
        );
}
