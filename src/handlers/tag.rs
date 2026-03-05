use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::middleware::validate_token;
use crate::models::CreateTagRequest;
use crate::services::{GitService, ProjectService};

/// 的路径参数
#[derive(Debug, serde::Deserialize)]
pub struct ProjectPath {
    pub namespace: String,
    pub project: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct TagPath {
    pub namespace: String,
    pub project: String,
    pub tag_name: String,
}

///  GET /projects/:namespace/:project/repository/tags
pub async fn list_tags(
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
    let tags = GitService::get_tags(config.get_ref(), &project.owner_name, &project.name).await?;
    Ok(HttpResponse::Ok().json(tags))
}

///  POST /projects/:namespace/:project/repository/tags
pub async fn create_tag(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    body: web::Json<CreateTagRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    let user = crate::services::UserService::get_user_by_id(pool.get_ref(), claims.user_id).await?;
    
    GitService::create_tag(
        config.get_ref(),
        &project.owner_name,
        &project.name,
        &body.name,
        &body.ref_name,
        body.message.as_deref(),
        user.display_name.as_deref().unwrap_or(&user.username),
        &user.email,
    ).await?;
    
    Ok(HttpResponse::Created().json(serde_json::json!({
        "name": body.name,
        "message": body.message,
        "target": body.ref_name
    })))
}

///  GET /projects/:namespace/:project/repository/tags/:tag_name
pub async fn get_tag(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<TagPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    let tags = GitService::get_tags(config.get_ref(), &project.owner_name, &project.name).await?;
    let tag = tags.into_iter()
        .find(|t| t.name == path.tag_name)
        .ok_or_else(|| crate::error::AppError::NotFound("Tag not found".to_string()))?;
    
    Ok(HttpResponse::Ok().json(tag))
}

///  DELETE /projects/:namespace/:project/repository/tags/:tag_name
pub async fn delete_tag(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<TagPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    GitService::delete_tag(config.get_ref(), &project.owner_name, &project.name, &path.tag_name).await?;
    Ok(HttpResponse::NoContent().finish())
}
