use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::models::{CommitListQuery, CompareQuery};
use crate::services::{GitService, ProjectService};
use serde::{Deserialize, Serialize};

#[derive(Debug, serde::Deserialize)]
pub struct ProjectPath {
    pub namespace: String,
    pub project: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CommitPath {
    pub namespace: String,
    pub project: String,
    pub sha: String,
}

/// GET /projects/:namespace/:project/repository/commits
pub async fn list_commits(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    query: web::Query<CommitListQuery>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // 从 Git 读取默认分支，不是数据库
    // 空字符串也视为 None
    let ref_name = match query.ref_name.as_deref() {
        Some(r) if !r.is_empty() => r.to_string(),
        _ => GitService::get_default_branch(config.get_ref(), &project.owner_name, &project.name).await?
            .ok_or_else(|| crate::error::AppError::NotFound("Empty repository".to_string()))?
    };
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    
    let commits = GitService::get_commits(config.get_ref(), &project.owner_name, &project.name, &ref_name, query.path.as_deref(), page, per_page).await?;
    Ok(HttpResponse::Ok().json(commits))
}

///  GET /projects/:namespace/:project/repository/commits/:sha
pub async fn get_commit(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<CommitPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    let commit = GitService::get_commit_detail(config.get_ref(), &project.owner_name, &project.name, &path.sha).await?;
    Ok(HttpResponse::Ok().json(commit))
}

///  GET /projects/:namespace/:project/repository/compare
pub async fn compare(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    query: web::Query<CompareQuery>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    let commits = GitService::compare_refs(config.get_ref(), &project.owner_name, &project.name, &query.from, &query.to).await?;
    Ok(HttpResponse::Ok().json(commits))
}

#[derive(Debug, Deserialize)]
pub struct FileDiffPath {
    pub namespace: String,
    pub project: String,
    pub sha: String,
    pub file_path: String,
}

#[derive(Debug, Serialize)]
pub struct FullFileDiff {
    pub original_content: Option<String>,
    pub modified_content: Option<String>,
    pub total_lines: u32,
}

/// GET /projects/:namespace/:project/repository/commits/:sha/files/*file_path
pub async fn get_full_file_diff(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, String, String, String)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, sha, file_path) = path.into_inner();
    
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &namespace, 
        &project_name
    ).await?;
    
    let file_diff = GitService::get_full_file_diff(config.get_ref(), &project.owner_name, &project.name, &sha, &file_path).await?;
    Ok(HttpResponse::Ok().json(file_diff))
}
