use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::models::{BrowseQuery, FileQuery};
use crate::services::{GitService, ProjectService};

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
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    let info = GitService::get_repository_info(&repo)?;
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
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    
    // 如果没指定分支，从git仓库获取默认分支
    let ref_name = if let Some(ref r) = query.ref_name {
        r.clone()
    } else {
        GitService::get_default_branch(&repo)?
            .ok_or_else(|| crate::error::AppError::NotFound("Repository is empty".to_string()))?
    };
    
    let entries = GitService::browse_tree(&repo, &ref_name, query.path.as_deref())?;
    
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
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    
    // 如果没指定分支，从git仓库获取默认分支
    let ref_name = if let Some(ref r) = query.ref_name {
        r.clone()
    } else {
        GitService::get_default_branch(&repo)?
            .ok_or_else(|| crate::error::AppError::NotFound("Repository is empty".to_string()))?
    };
    
    let content = GitService::get_file_content(&repo, &ref_name, &path.filepath)?;
    
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
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    
    let blob = GitService::get_blob(&repo, &path.sha)?;
    
    Ok(HttpResponse::Ok().json(blob))
}
