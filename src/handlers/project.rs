use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;


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

/// 的路径参数: /{namespace}/{project}
#[derive(Debug, serde::Deserialize)]
pub struct ProjectPath {
    pub namespace: String,
    pub project: String,
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
    GitService::init_repository(config.get_ref(), &project.owner_name, &project.name)?;
    
    Ok(HttpResponse::Created().json(project))
}

///  GET /projects/:namespace/:project
pub async fn get_project(
    pool: web::Data<PgPool>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    Ok(HttpResponse::Ok().json(project))
}

///  PUT /projects/:namespace/:project
pub async fn update_project(
    pool: web::Data<PgPool>,
    path: web::Path<ProjectPath>,
    body: web::Json<UpdateProjectRequest>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    let updated = ProjectService::update_project(pool.get_ref(), project.id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(updated))
}

///  DELETE /projects/:namespace/:project
pub async fn delete_project(
    pool: web::Data<PgPool>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    ProjectService::delete_project(pool.get_ref(), project.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_project_stats(
    pool: web::Data<PgPool>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    let stats = ProjectService::get_project_stats(pool.get_ref(), project.id).await?;
    Ok(HttpResponse::Ok().json(stats))
}

pub async fn get_members(
    pool: web::Data<PgPool>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    let members = ProjectService::get_project_members(pool.get_ref(), project.id).await?;
    Ok(HttpResponse::Ok().json(members))
}

#[derive(Debug, serde::Deserialize)]
pub struct MemberPath {
    pub namespace: String,
    pub project: String,
    pub user_id: i64,
}

pub async fn add_member(
    pool: web::Data<PgPool>,
    path: web::Path<ProjectPath>,
    body: web::Json<AddMemberRequest>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
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
    path: web::Path<MemberPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    ProjectService::remove_member(pool.get_ref(), project.id, path.user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
