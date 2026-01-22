use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::validate_token;
use crate::models::{AddMemberRequest, CreateProjectRequest, UpdateProjectRequest};
use crate::services::{GitService, ProjectService};

#[derive(Debug, serde::Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub async fn list_projects(
    pool: web::Data<PgPool>,
    query: web::Query<ListQuery>,
) -> AppResult<HttpResponse> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    
    let projects = ProjectService::list_projects(pool.get_ref(), None, page, per_page).await?;
    Ok(HttpResponse::Ok().json(projects))
}

pub async fn create_project(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    body: web::Json<CreateProjectRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let project = ProjectService::create_project(
        pool.get_ref(),
        claims.user_id,
        body.into_inner(),
    ).await?;
    
    // Initialize git repository
    GitService::init_repository(config.get_ref(), &project.slug)?;
    
    Ok(HttpResponse::Created().json(project))
}

pub async fn get_project(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(project))
}

pub async fn update_project(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    body: web::Json<UpdateProjectRequest>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let updated = ProjectService::update_project(pool.get_ref(), project.id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(updated))
}

pub async fn delete_project(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    ProjectService::delete_project(pool.get_ref(), project.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_project_stats(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let stats = ProjectService::get_project_stats(pool.get_ref(), project.id).await?;
    Ok(HttpResponse::Ok().json(stats))
}

pub async fn get_members(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let members = ProjectService::get_project_members(pool.get_ref(), project.id).await?;
    Ok(HttpResponse::Ok().json(members))
}

pub async fn add_member(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    body: web::Json<AddMemberRequest>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let member = ProjectService::add_member(
        pool.get_ref(),
        project.id,
        body.user_id,
        body.role.clone(),
    ).await?;
    Ok(HttpResponse::Created().json(member))
}

pub async fn remove_member(
    pool: web::Data<PgPool>,
    path: web::Path<(String, Uuid)>,
) -> AppResult<HttpResponse> {
    let (slug, user_id) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    ProjectService::remove_member(pool.get_ref(), project.id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
