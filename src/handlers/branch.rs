use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::models::CreateBranchRequest;
use crate::services::{GitService, ProjectService};

pub async fn list_branches(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    let branches = GitService::get_branches(&repo, &project.default_branch)?;
    Ok(HttpResponse::Ok().json(branches))
}

pub async fn create_branch(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    body: web::Json<CreateBranchRequest>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    GitService::create_branch(&repo, &body.name, &body.ref_name)?;
    Ok(HttpResponse::Created().json(serde_json::json!({
        "message": "Branch created successfully",
        "name": body.name
    })))
}

pub async fn get_branch(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (slug, branch_name) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    let branches = GitService::get_branches(&repo, &project.default_branch)?;
    
    let branch = branches.into_iter()
        .find(|b| b.name == branch_name)
        .ok_or_else(|| crate::error::AppError::NotFound("Branch not found".to_string()))?;
    
    Ok(HttpResponse::Ok().json(branch))
}

pub async fn delete_branch(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (slug, branch_name) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    GitService::delete_branch(&repo, &branch_name)?;
    Ok(HttpResponse::NoContent().finish())
}
