use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::models::CreateBranchRequest;
use crate::services::{GitService, ProjectService};

/// 的路径参数
#[derive(Debug, serde::Deserialize)]
pub struct ProjectPath {
    pub namespace: String,
    pub project: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct BranchPath {
    pub namespace: String,
    pub project: String,
    pub branch: String,
}

///  GET /projects/:namespace/:project/repository/branches
pub async fn list_branches(
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
    let branches = GitService::get_branches(&repo)?;
    Ok(HttpResponse::Ok().json(branches))
}

///  POST /projects/:namespace/:project/repository/branches
pub async fn create_branch(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    body: web::Json<CreateBranchRequest>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    GitService::create_branch(&repo, &body.name, &body.ref_name)?;
    Ok(HttpResponse::Created().json(serde_json::json!({
        "name": body.name,
        "commit": null,
        "merged": false,
        "protected": false,
        "default": false
    })))
}

///  GET /projects/:namespace/:project/repository/branches/:branch
pub async fn get_branch(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<BranchPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    let branches = GitService::get_branches(&repo)?;
    
    let branch = branches.into_iter()
        .find(|b| b.name == path.branch)
        .ok_or_else(|| crate::error::AppError::NotFound("Branch not found".to_string()))?;
    
    Ok(HttpResponse::Ok().json(branch))
}

///  DELETE /projects/:namespace/:project/repository/branches/:branch
pub async fn delete_branch(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<BranchPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    GitService::delete_branch(&repo, &path.branch)?;
    Ok(HttpResponse::NoContent().finish())
}
