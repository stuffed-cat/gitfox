use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;


use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::{validate_token, try_validate_token};
use crate::models::{AddMemberRequest, CreateProjectRequest, UpdateProjectRequest, ForkProjectRequest, ProjectStar};
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
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    query: web::Query<ListQuery>,
) -> AppResult<HttpResponse> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    
    // Try to get current user ID if authenticated
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let user_id = try_validate_token(&service_req, config.get_ref()).await.map(|c| c.user_id);
    
    let projects = ProjectService::list_projects(pool.get_ref(), user_id, page, per_page).await?;
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
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    body: web::Json<UpdateProjectRequest>,
) -> AppResult<HttpResponse> {
    // 验证用户认证
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // 检查用户是否有管理权限（owner 或 maintainer）
    if !check_admin_permission(pool.get_ref(), claims.user_id, project.id, project.owner_id).await? {
        return Err(AppError::Forbidden("You don't have permission to update this project".to_string()));
    }
    
    let updated = ProjectService::update_project(pool.get_ref(), project.id, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(updated))
}

///  DELETE /projects/:namespace/:project
pub async fn delete_project(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    // 验证用户认证
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // 只有 owner 可以删除项目
    if claims.user_id != project.owner_id {
        return Err(AppError::Forbidden("Only the project owner can delete this project".to_string()));
    }
    
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
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    body: web::Json<AddMemberRequest>,
) -> AppResult<HttpResponse> {
    // 验证用户认证
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // 检查用户是否有管理权限（owner 或 maintainer）
    if !check_admin_permission(pool.get_ref(), claims.user_id, project.id, project.owner_id).await? {
        return Err(AppError::Forbidden("You don't have permission to manage members".to_string()));
    }
    
    let member = ProjectService::add_member(
        pool.get_ref(),
        project.id,
        body.user_id,
        body.role.clone(),
    ).await?;
    Ok(HttpResponse::Created().json(member))
}

pub async fn remove_member(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<MemberPath>,
) -> AppResult<HttpResponse> {
    // 验证用户认证
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // 检查用户是否有管理权限（owner 或 maintainer）
    if !check_admin_permission(pool.get_ref(), claims.user_id, project.id, project.owner_id).await? {
        return Err(AppError::Forbidden("You don't have permission to manage members".to_string()));
    }
    
    // 不允许移除 owner
    if path.user_id == project.owner_id {
        return Err(AppError::BadRequest("Cannot remove the project owner".to_string()));
    }
    
    ProjectService::remove_member(pool.get_ref(), project.id, path.user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// 检查用户是否有项目的管理权限
/// 只有 owner 和 maintainer 可以管理项目
async fn check_admin_permission(
    pool: &PgPool,
    user_id: i64,
    project_id: i64,
    owner_id: i64,
) -> AppResult<bool> {
    // Owner 总是有管理权限
    if user_id == owner_id {
        return Ok(true);
    }
    
    // 检查项目成员角色
    let role = sqlx::query_scalar::<_, String>(
        r#"SELECT role::text FROM project_members WHERE project_id = $1 AND user_id = $2"#
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    
    match role {
        Some(role) => {
            // 只有 owner 和 maintainer 有管理权限
            Ok(matches!(role.as_str(), "owner" | "maintainer"))
        }
        None => Ok(false),
    }
}

// ============== Star APIs ==============

/// Check if current user has starred the project
/// GET /projects/:namespace/:project/starred
pub async fn check_starred(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = match try_validate_token(&service_req, config.get_ref()).await {
        Some(c) => c,
        None => return Ok(HttpResponse::Ok().json(serde_json::json!({ "starred": false }))),
    };
    
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    let star = sqlx::query_as::<_, ProjectStar>(
        "SELECT * FROM project_stars WHERE project_id = $1 AND user_id = $2"
    )
    .bind(project.id)
    .bind(claims.user_id)
    .fetch_optional(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "starred": star.is_some()
    })))
}

/// Star a project
/// POST /projects/:namespace/:project/star
pub async fn star_project(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // Insert star (ignore if already exists)
    sqlx::query(
        r#"
        INSERT INTO project_stars (project_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (project_id, user_id) DO NOTHING
        "#
    )
    .bind(project.id)
    .bind(claims.user_id)
    .execute(pool.get_ref())
    .await?;
    
    // Get updated stars count
    let stars_count = sqlx::query_scalar::<_, i32>(
        "SELECT stars_count FROM projects WHERE id = $1"
    )
    .bind(project.id)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "starred": true,
        "stars_count": stars_count
    })))
}

/// Unstar a project
/// DELETE /projects/:namespace/:project/star
pub async fn unstar_project(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    sqlx::query(
        "DELETE FROM project_stars WHERE project_id = $1 AND user_id = $2"
    )
    .bind(project.id)
    .bind(claims.user_id)
    .execute(pool.get_ref())
    .await?;
    
    // Get updated stars count
    let stars_count = sqlx::query_scalar::<_, i32>(
        "SELECT stars_count FROM projects WHERE id = $1"
    )
    .bind(project.id)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "starred": false,
        "stars_count": stars_count
    })))
}

// ============== Fork APIs ==============

/// Fork a project
/// POST /projects/:namespace/:project/fork
pub async fn fork_project(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<ProjectPath>,
    body: web::Json<ForkProjectRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let path = path.into_inner();
    let source_project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    // Create the forked project
    let fork_name = body.name.clone().unwrap_or_else(|| source_project.name.clone());
    let fork_description = body.description.clone().or_else(|| source_project.description.clone());
    let fork_visibility = body.visibility.clone().unwrap_or_else(|| source_project.visibility.clone());
    
    let forked_project = ProjectService::create_fork(
        pool.get_ref(),
        claims.user_id,
        source_project.id,
        body.namespace_id,
        &fork_name,
        fork_description,
        fork_visibility,
    ).await?;
    
    // Copy the git repository
    let only_default_branch = body.branches.as_deref() == Some("default");
    GitService::fork_repository(
        config.get_ref(),
        &source_project.owner_name,
        &source_project.name,
        &forked_project.owner_name,
        &forked_project.name,
        only_default_branch,
    )?;
    
    // Create fork record
    sqlx::query(
        r#"
        INSERT INTO project_forks (source_project_id, forked_project_id, forked_by)
        VALUES ($1, $2, $3)
        "#
    )
    .bind(source_project.id)
    .bind(forked_project.id)
    .bind(claims.user_id)
    .execute(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(forked_project))
}

/// Get fork info for a project
/// GET /projects/:namespace/:project/forks
pub async fn list_forks(
    pool: web::Data<PgPool>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    let forks = ProjectService::list_forks(pool.get_ref(), project.id).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "forks_count": project.forks_count,
        "forks": forks
    })))
}

/// GET /projects/:namespace/:project/fork_network
/// Returns all projects in the same fork network (the entire fork tree)
pub async fn get_fork_network(
    pool: web::Data<PgPool>,
    path: web::Path<ProjectPath>,
) -> AppResult<HttpResponse> {
    let path = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(
        pool.get_ref(), 
        &path.namespace, 
        &path.project
    ).await?;
    
    let network = ProjectService::get_fork_network(pool.get_ref(), project.id).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "current_project_id": project.id,
        "projects": network
    })))
}
