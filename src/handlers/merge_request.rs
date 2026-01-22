use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::validate_token;
use crate::models::{
    CreateCommentRequest, CreateMergeRequestRequest, CreateReviewRequest, MergeOptions,
    MergeRequest, MergeRequestComment, MergeRequestListQuery, MergeRequestReview,
    MergeRequestStatus, ReviewStatus, UpdateMergeRequestRequest,
};
use crate::services::{GitService, ProjectService};

pub async fn list_merge_requests(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    query: web::Query<MergeRequestListQuery>,
) -> AppResult<HttpResponse> {
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    let offset = (page.saturating_sub(1) * per_page) as i64;
    
    let mrs = sqlx::query_as::<_, MergeRequest>(
        r#"
        SELECT * FROM merge_requests 
        WHERE project_id = $1
        AND ($2::merge_request_status IS NULL OR status = $2)
        AND ($3::uuid IS NULL OR author_id = $3)
        AND ($4::uuid IS NULL OR assignee_id = $4)
        ORDER BY updated_at DESC
        LIMIT $5 OFFSET $6
        "#
    )
    .bind(project.id)
    .bind(&query.status)
    .bind(&query.author_id)
    .bind(&query.assignee_id)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(mrs))
}

pub async fn create_merge_request(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    body: web::Json<CreateMergeRequestRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &path.into_inner()).await?;
    
    // Get next IID for this project
    let next_iid: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(iid), 0) + 1 FROM merge_requests WHERE project_id = $1"
    )
    .bind(project.id)
    .fetch_one(pool.get_ref())
    .await?;
    
    let id = Uuid::new_v4();
    let now = Utc::now();
    let status = if body.is_draft.unwrap_or(false) {
        MergeRequestStatus::Draft
    } else {
        MergeRequestStatus::Open
    };
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        r#"
        INSERT INTO merge_requests 
        (id, project_id, iid, title, description, source_branch, target_branch, status, author_id, assignee_id, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(project.id)
    .bind(next_iid)
    .bind(&body.title)
    .bind(&body.description)
    .bind(&body.source_branch)
    .bind(&body.target_branch)
    .bind(status)
    .bind(claims.user_id)
    .bind(&body.assignee_id)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(mr))
}

pub async fn get_merge_request(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, i64)>,
) -> AppResult<HttpResponse> {
    let (slug, iid) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        "SELECT * FROM merge_requests WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    let comments = sqlx::query_as::<_, MergeRequestComment>(
        "SELECT * FROM merge_request_comments WHERE merge_request_id = $1 ORDER BY created_at"
    )
    .bind(mr.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    let reviews = sqlx::query_as::<_, MergeRequestReview>(
        "SELECT * FROM merge_request_reviews WHERE merge_request_id = $1 ORDER BY created_at"
    )
    .bind(mr.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    // Check if can merge
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    let can_merge = GitService::can_merge(&repo, &mr.source_branch, &mr.target_branch).unwrap_or(false);
    let has_conflicts = !can_merge;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "merge_request": mr,
        "comments": comments,
        "reviews": reviews,
        "can_merge": can_merge,
        "has_conflicts": has_conflicts
    })))
}

pub async fn update_merge_request(
    pool: web::Data<PgPool>,
    path: web::Path<(String, i64)>,
    body: web::Json<UpdateMergeRequestRequest>,
) -> AppResult<HttpResponse> {
    let (slug, iid) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        r#"
        UPDATE merge_requests
        SET title = COALESCE($3, title),
            description = COALESCE($4, description),
            target_branch = COALESCE($5, target_branch),
            assignee_id = COALESCE($6, assignee_id),
            updated_at = NOW()
        WHERE project_id = $1 AND iid = $2
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(iid)
    .bind(&body.title)
    .bind(&body.description)
    .bind(&body.target_branch)
    .bind(&body.assignee_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    Ok(HttpResponse::Ok().json(mr))
}

pub async fn merge(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, i64)>,
    body: web::Json<MergeOptions>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (slug, iid) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        "SELECT * FROM merge_requests WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    if mr.status != MergeRequestStatus::Open {
        return Err(AppError::BadRequest("Merge request is not open".to_string()));
    }
    
    // Check if can merge
    let repo = GitService::open_repository(config.get_ref(), &project.slug)?;
    if !GitService::can_merge(&repo, &mr.source_branch, &mr.target_branch)? {
        return Err(AppError::Conflict("Cannot merge due to conflicts".to_string()));
    }
    
    // Update MR status
    let now = Utc::now();
    let updated_mr = sqlx::query_as::<_, MergeRequest>(
        r#"
        UPDATE merge_requests
        SET status = 'merged', merged_by = $3, merged_at = $4, updated_at = $4
        WHERE project_id = $1 AND iid = $2
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(iid)
    .bind(claims.user_id)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(updated_mr))
}

pub async fn close(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, i64)>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (slug, iid) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let now = Utc::now();
    let mr = sqlx::query_as::<_, MergeRequest>(
        r#"
        UPDATE merge_requests
        SET status = 'closed', closed_by = $3, closed_at = $4, updated_at = $4
        WHERE project_id = $1 AND iid = $2
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(iid)
    .bind(claims.user_id)
    .bind(now)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    Ok(HttpResponse::Ok().json(mr))
}

pub async fn list_comments(
    pool: web::Data<PgPool>,
    path: web::Path<(String, i64)>,
) -> AppResult<HttpResponse> {
    let (slug, iid) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        "SELECT * FROM merge_requests WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    let comments = sqlx::query_as::<_, MergeRequestComment>(
        "SELECT * FROM merge_request_comments WHERE merge_request_id = $1 ORDER BY created_at"
    )
    .bind(mr.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(comments))
}

pub async fn add_comment(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, i64)>,
    body: web::Json<CreateCommentRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (slug, iid) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        "SELECT * FROM merge_requests WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    let comment = sqlx::query_as::<_, MergeRequestComment>(
        r#"
        INSERT INTO merge_request_comments 
        (id, merge_request_id, author_id, content, line_number, file_path, parent_id, is_resolved, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, false, $8, $8)
        RETURNING *
        "#
    )
    .bind(id)
    .bind(mr.id)
    .bind(claims.user_id)
    .bind(&body.content)
    .bind(body.line_number)
    .bind(&body.file_path)
    .bind(&body.parent_id)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(comment))
}

pub async fn add_review(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, i64)>,
    body: web::Json<CreateReviewRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (slug, iid) = path.into_inner();
    let project = ProjectService::get_project_by_slug(pool.get_ref(), &slug).await?;
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        "SELECT * FROM merge_requests WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    let id = Uuid::new_v4();
    let now = Utc::now();
    
    let review = sqlx::query_as::<_, MergeRequestReview>(
        r#"
        INSERT INTO merge_request_reviews 
        (id, merge_request_id, reviewer_id, status, comment, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        ON CONFLICT (merge_request_id, reviewer_id) 
        DO UPDATE SET status = $4, comment = $5, updated_at = $6
        RETURNING *
        "#
    )
    .bind(id)
    .bind(mr.id)
    .bind(claims.user_id)
    .bind(&body.status)
    .bind(&body.comment)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(review))
}
