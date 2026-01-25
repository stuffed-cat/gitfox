use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::models::{CommitListQuery, CompareQuery};
use crate::services::{GitService, ProjectService};

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
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    
    let ref_name = query.ref_name.as_deref().unwrap_or(&project.default_branch);
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    
    let commits = GitService::get_commits(&repo, ref_name, query.path.as_deref(), page, per_page)?;
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
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    
    let commit = GitService::get_commit_detail(&repo, &path.sha)?;
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
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    
    let commits = GitService::compare_refs(&repo, &query.from, &query.to)?;
    Ok(HttpResponse::Ok().json(commits))
}
