use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::middleware::validate_token;
use crate::models::CreateTagRequest;
use crate::services::{GitService, ProjectService};

pub async fn list_tags(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    let tags = GitService::get_tags(&repo)?;
    Ok(HttpResponse::Ok().json(tags))
}

pub async fn create_tag(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    body: web::Json<CreateTagRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    
    let user = crate::services::UserService::get_user_by_id(pool.get_ref(), claims.user_id).await?;
    
    GitService::create_tag(
        &repo,
        &body.name,
        &body.ref_name,
        body.message.as_deref(),
        user.display_name.as_deref().unwrap_or(&user.username),
        &user.email,
    )?;
    
    Ok(HttpResponse::Created().json(serde_json::json!({
        "message": "Tag created successfully",
        "name": body.name
    })))
}

pub async fn get_tag(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (slug, tag_name) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    
    let tags = GitService::get_tags(&repo)?;
    let tag = tags.into_iter()
        .find(|t| t.name == tag_name)
        .ok_or_else(|| crate::error::AppError::NotFound("Tag not found".to_string()))?;
    
    Ok(HttpResponse::Ok().json(tag))
}

pub async fn delete_tag(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (slug, tag_name) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    GitService::delete_tag(&repo, &tag_name)?;
    Ok(HttpResponse::NoContent().finish())
}
