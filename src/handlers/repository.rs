use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::models::{BrowseQuery, FileQuery};
use crate::services::{GitService, ProjectService};

pub async fn get_repository_info(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    let info = GitService::get_repository_info(&repo, &project.default_branch)?;
    Ok(HttpResponse::Ok().json(info))
}

pub async fn browse_tree(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    query: web::Query<BrowseQuery>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    
    let ref_name = query.ref_name.as_deref().unwrap_or(&project.default_branch);
    let entries = GitService::browse_tree(&repo, ref_name, query.path.as_deref())?;
    
    Ok(HttpResponse::Ok().json(entries))
}

pub async fn get_file(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    query: web::Query<FileQuery>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    
    let ref_name = query.ref_name.as_deref().unwrap_or(&project.default_branch);
    let content = GitService::get_file_content(&repo, ref_name, &query.path)?;
    
    Ok(HttpResponse::Ok().json(content))
}
