use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::path::PathBuf;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::AuthenticatedUser;
use crate::models::{BrowseQuery, FileQuery};
use crate::services::{GitService, GpgKeyService, ProjectService};
use crate::services::gitlayer::{
    OperationServiceClient, Repository as GrpcRepository, Signature as GrpcSignature,
    WriteFileRequest, DeleteFileRequest as GrpcDeleteFileRequest, CreateCommitRequest, FileAction as GrpcFileAction,
};

/// Helper to create a gRPC Signature with current timestamp
fn make_signature(name: &str, email: &str) -> GrpcSignature {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    GrpcSignature {
        name: name.to_string(),
        email: email.to_string(),
        timestamp,
        timezone: "+0000".to_string(),
    }
}

/// 的路径参数
#[derive(Debug, serde::Deserialize)]
pub struct ProjectPath {
    pub namespace: String,
    pub project: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct FilePath {
    pub namespace: String,
    pub project: String,
    pub filepath: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct BlobPath {
    pub namespace: String,
    pub project: String,
    pub sha: String,
}

///  GET /projects/:namespace/:project/repository
pub async fn get_repository_info(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    let info = GitService::get_repository_info(config.get_ref(), &project.owner_name, &project.name).await?;
    Ok(HttpResponse::Ok().json(info))
}

///  GET /projects/:namespace/:project/repository/tree
pub async fn browse_tree(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    query: web::Query<BrowseQuery>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // 如果没指定分支或为空字符串，从 git 仓库获取默认分支
    let ref_name = match query.ref_name.as_deref() {
        Some(r) if !r.is_empty() => r.to_string(),
        _ => GitService::get_default_branch(config.get_ref(), &project.owner_name, &project.name).await?
            .ok_or_else(|| crate::error::AppError::NotFound("Repository is empty".to_string()))?
    };
    
    let entries = GitService::browse_tree(config.get_ref(), &project.owner_name, &project.name, &ref_name, query.path.as_deref()).await?;
    
    Ok(HttpResponse::Ok().json(entries))
}

///  GET /projects/:namespace/:project/repository/files/:filepath
pub async fn get_file(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<FilePath>,
    query: web::Query<FileQuery>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // 如果没指定分支或为空字符串，从 git 仓库获取默认分支
    let ref_name = match query.ref_name.as_deref() {
        Some(r) if !r.is_empty() => r.to_string(),
        _ => GitService::get_default_branch(config.get_ref(), &project.owner_name, &project.name).await?
            .ok_or_else(|| crate::error::AppError::NotFound("Repository is empty".to_string()))?
    };
    
    let content = GitService::get_file_content(config.get_ref(), &project.owner_name, &project.name, &ref_name, &path.filepath).await?;
    
    // 如果请求原始内容，直接返回文件内容
    if query.raw {
        if content.is_binary {
            // 二进制文件需要 base64 解码
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&content.content)
                .map_err(|e| crate::error::AppError::InternalError(format!("Base64 decode error: {}", e)))?;
            
            // 根据文件名猜测 MIME 类型
            let mime_type = mime_guess::from_path(&path.filepath)
                .first_or_octet_stream()
                .to_string();
            
            return Ok(HttpResponse::Ok()
                .content_type(mime_type)
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", 
                    path.filepath.split('/').last().unwrap_or(&path.filepath))))
                .body(bytes));
        } else {
            // 文本文件直接返回
            let mime_type = mime_guess::from_path(&path.filepath)
                .first_or_text_plain()
                .to_string();
            
            return Ok(HttpResponse::Ok()
                .content_type(mime_type)
                .insert_header(("Content-Disposition", format!("inline; filename=\"{}\"", 
                    path.filepath.split('/').last().unwrap_or(&path.filepath))))
                .body(content.content));
        }
    }
    
    Ok(HttpResponse::Ok().json(content))
}

///  GET /projects/:namespace/:project/repository/blobs/:sha
pub async fn get_blob(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<BlobPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    let blob = GitService::get_blob(config.get_ref(), &project.owner_name, &project.name, &path.sha).await?;
    
    Ok(HttpResponse::Ok().json(blob))
}

// ==================== WebIDE File Operations ====================

#[derive(Debug, Deserialize)]
pub struct CreateFileRequest {
    pub branch: String,
    pub content: String,
    pub commit_message: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFileRequest {
    pub branch: String,
    pub content: String,
    pub commit_message: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFileRequest {
    pub branch: String,
    pub commit_message: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchCommitRequest {
    pub branch: String,
    pub commit_message: String,
    pub actions: Vec<FileAction>,
}

#[derive(Debug, Deserialize)]
pub struct FileAction {
    pub action: String,  // "create", "update", "delete"
    pub file_path: String,
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommitResponse {
    pub sha: String,
}

/// POST /projects/:namespace/:project/repository/files/:filepath
/// Create a new file
pub async fn create_file(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<FilePath>,
    auth: AuthenticatedUser,
    body: web::Json<CreateFileRequest>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(),
        &path.namespace,
        &path.project,
    ).await?;
    
    // Check write permission
    let has_write_access = check_project_write_permission(
        pool.get_ref(),
        project.id,
        auth.user_id,
        project.owner_id,
    ).await?;
    
    if !has_write_access {
        return Err(AppError::Forbidden(
            "You do not have write access to this repository".to_string()
        ));
    }
    
    // Get user's system GPG key for signing
    let gpg_key = GpgKeyService::get_or_create_system_key(
        pool.get_ref(),
        auth.user_id,
        &format!("{}@gitfox.local", auth.username),
        &auth.username,
    ).await?;
    let gpg_private_key = gpg_key.private_key_encrypted.unwrap_or_default();
    
    // Connect to GitLayer
    let gitlayer_address = std::env::var("GITLAYER_URL")
        .unwrap_or_else(|_| "http://[::1]:50052".to_string());
    
    let mut client = OperationServiceClient::connect(gitlayer_address)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;
    
    // Build repository path
    let base_path = PathBuf::from(&config.git_repos_path);
    let repo_path = base_path.join(format!("{}/{}.git", project.owner_name, project.name));
    
    let request = WriteFileRequest {
        repository: Some(GrpcRepository {
            storage_path: repo_path.to_string_lossy().to_string(),
            relative_path: String::new(),
        }),
        branch: body.branch.clone(),
        path: path.filepath.clone(),
        content: body.content.as_bytes().to_vec(),
        author: Some(make_signature(&auth.username, &format!("{}@gitfox.local", auth.username))),
        committer: Some(make_signature(&auth.username, &format!("{}@gitfox.local", auth.username))),
        message: body.commit_message.clone(),
        create_branch: false,
        gpg_private_key,
    };
    
    let response = client.write_file(request)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to create file: {}", e)))?
        .into_inner();
    
    if !response.success {
        return Err(AppError::InternalError(format!("Create file failed: {}", response.message)));
    }
    
    Ok(HttpResponse::Created().json(CommitResponse { sha: response.commit_id }))
}

/// PUT /projects/:namespace/:project/repository/files/:filepath
/// Update an existing file
pub async fn update_file(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<FilePath>,
    auth: AuthenticatedUser,
    body: web::Json<UpdateFileRequest>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(),
        &path.namespace,
        &path.project,
    ).await?;
    
    // Get user's system GPG key for signing
    let gpg_key = GpgKeyService::get_or_create_system_key(
        pool.get_ref(),
        auth.user_id,
        &format!("{}@gitfox.local", auth.username),
        &auth.username,
    ).await?;
    let gpg_private_key = gpg_key.private_key_encrypted.unwrap_or_default();
    
    // Connect to GitLayer
    let gitlayer_address = std::env::var("GITLAYER_URL")
        .unwrap_or_else(|_| "http://[::1]:50052".to_string());
    
    let mut client = OperationServiceClient::connect(gitlayer_address)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;
    
    // Build repository path
    let base_path = PathBuf::from(&config.git_repos_path);
    let repo_path = base_path.join(format!("{}/{}.git", project.owner_name, project.name));
    
    let request = WriteFileRequest {
        repository: Some(GrpcRepository {
            storage_path: repo_path.to_string_lossy().to_string(),
            relative_path: String::new(),
        }),
        branch: body.branch.clone(),
        path: path.filepath.clone(),
        content: body.content.as_bytes().to_vec(),
        author: Some(make_signature(&auth.username, &format!("{}@gitfox.local", auth.username))),
        committer: Some(make_signature(&auth.username, &format!("{}@gitfox.local", auth.username))),
        message: body.commit_message.clone(),
        create_branch: false,
        gpg_private_key,
    };
    
    let response = client.write_file(request)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to update file: {}", e)))?
        .into_inner();
    
    if !response.success {
        return Err(AppError::InternalError(format!("Update file failed: {}", response.message)));
    }
    
    Ok(HttpResponse::Ok().json(CommitResponse { sha: response.commit_id }))
}

/// DELETE /projects/:namespace/:project/repository/files/:filepath
/// Delete a file
pub async fn delete_file(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<FilePath>,
    auth: AuthenticatedUser,
    query: web::Query<DeleteFileRequest>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(),
        &path.namespace,
        &path.project,
    ).await?;
    
    // Get user's system GPG key for signing
    let gpg_key = GpgKeyService::get_or_create_system_key(
        pool.get_ref(),
        auth.user_id,
        &format!("{}@gitfox.local", auth.username),
        &auth.username,
    ).await?;
    let gpg_private_key = gpg_key.private_key_encrypted.unwrap_or_default();
    
    // Connect to GitLayer
    let gitlayer_address = std::env::var("GITLAYER_URL")
        .unwrap_or_else(|_| "http://[::1]:50052".to_string());
    
    let mut client = OperationServiceClient::connect(gitlayer_address)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;
    
    // Build repository path
    let base_path = PathBuf::from(&config.git_repos_path);
    let repo_path = base_path.join(format!("{}/{}.git", project.owner_name, project.name));
    
    let request = GrpcDeleteFileRequest {
        repository: Some(GrpcRepository {
            storage_path: repo_path.to_string_lossy().to_string(),
            relative_path: String::new(),
        }),
        branch: query.branch.clone(),
        path: path.filepath.clone(),
        author: Some(make_signature(&auth.username, &format!("{}@gitfox.local", auth.username))),
        committer: Some(make_signature(&auth.username, &format!("{}@gitfox.local", auth.username))),
        message: query.commit_message.clone(),
        gpg_private_key,
    };
    
    let response = client.delete_file(request)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete file: {}", e)))?
        .into_inner();
    
    if !response.success {
        return Err(AppError::InternalError(format!("Delete file failed: {}", response.message)));
    }
    
    Ok(HttpResponse::Ok().json(CommitResponse { sha: response.commit_id }))
}

/// POST /projects/:namespace/:project/repository/commits/batch
/// Batch commit multiple file changes (for WebIDE)
pub async fn batch_commit(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    auth: AuthenticatedUser,
    body: web::Json<BatchCommitRequest>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(),
        &path.namespace,
        &path.project,
    ).await?;
    
    // Get user's system GPG key for signing
    let gpg_key = GpgKeyService::get_or_create_system_key(
        pool.get_ref(),
        auth.user_id,
        &format!("{}@gitfox.local", auth.username),
        &auth.username,
    ).await?;
    let gpg_private_key = gpg_key.private_key_encrypted.unwrap_or_default();
    
    // Connect to GitLayer
    let gitlayer_address = std::env::var("GITLAYER_URL")
        .unwrap_or_else(|_| "http://[::1]:50052".to_string());
    
    let mut client = OperationServiceClient::connect(gitlayer_address)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to connect to GitLayer: {}", e)))?;
    
    // Build repository path
    let base_path = PathBuf::from(&config.git_repos_path);
    let repo_path = base_path.join(format!("{}/{}.git", project.owner_name, project.name));
    
    // Convert FileAction to gRPC FileAction
    let actions: Vec<GrpcFileAction> = body.actions.iter().map(|action| {
        GrpcFileAction {
            action: action.action.clone(),
            path: action.file_path.clone(),
            content: action.content.as_ref().map(|c| c.as_bytes().to_vec()).unwrap_or_default(),
            previous_path: String::new(),
            mode: 0o100644,
            expected_blob_id: String::new(),
        }
    }).collect();
    
    let request = CreateCommitRequest {
        repository: Some(GrpcRepository {
            storage_path: repo_path.to_string_lossy().to_string(),
            relative_path: String::new(),
        }),
        branch: body.branch.clone(),
        author: Some(make_signature(&auth.username, &format!("{}@gitfox.local", auth.username))),
        committer: Some(make_signature(&auth.username, &format!("{}@gitfox.local", auth.username))),
        message: body.commit_message.clone(),
        actions,
        create_branch: false,
        expected_parent: String::new(),
        gpg_private_key,
    };
    
    let response = client.create_commit(request)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to create commit: {}", e)))?
        .into_inner();
    
    if !response.success {
        return Err(AppError::InternalError(format!("Batch commit failed: {}", response.message)));
    }
    
    Ok(HttpResponse::Created().json(CommitResponse { sha: response.commit_id }))
}

/// Check if user has write permission to a project
/// Returns true if user is owner, maintainer, or developer
async fn check_project_write_permission(
    pool: &PgPool,
    project_id: i64,
    user_id: i64,
    owner_id: i64,
) -> Result<bool, crate::error::AppError> {
    use crate::models::MemberRole;
    
    // Owner always has write access
    if user_id == owner_id {
        return Ok(true);
    }
    
    // Check project membership with write-level roles
    let role = ProjectService::get_member_role(pool, project_id, user_id).await?;
    
    match role {
        Some(MemberRole::Owner) | Some(MemberRole::Maintainer) | Some(MemberRole::Developer) => Ok(true),
        _ => {
            // Check if user is member of the namespace (group) with write access
            let has_group_write: bool = sqlx::query_scalar(
                r#"
                SELECT EXISTS(
                    SELECT 1 
                    FROM group_members gm
                    JOIN groups g ON gm.group_id = g.id
                    JOIN projects p ON p.namespace_id = g.namespace_id
                    WHERE p.id = $1 
                    AND gm.user_id = $2
                    AND gm.access_level >= 30  -- Developer level or higher
                )
                "#
            )
            .bind(project_id)
            .bind(user_id)
            .fetch_one(pool)
            .await?;
            
            Ok(has_group_write)
        }
    }
}
