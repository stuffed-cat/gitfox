use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::models::{CommitListQuery, CompareQuery};
use crate::services::{GitService, ProjectService};

pub async fn list_commits(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    query: web::Query<CommitListQuery>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    
    let ref_name = query.ref_name.as_deref().unwrap_or(&project.default_branch);
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    
    let commits = GitService::get_commits(&repo, ref_name, query.path.as_deref(), page, per_page)?;
    Ok(HttpResponse::Ok().json(commits))
}

pub async fn get_commit(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (slug, sha) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    
    let commit = GitService::get_commit_detail(&repo, &sha)?;
    Ok(HttpResponse::Ok().json(commit))
}

pub async fn compare(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    query: web::Query<CompareQuery>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    
    let commits = GitService::compare_refs(&repo, &query.from, &query.to)?;
    Ok(HttpResponse::Ok().json(commits))
}
