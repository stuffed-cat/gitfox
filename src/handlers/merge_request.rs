use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;


use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::validate_token;
use crate::models::{
    CreateCommentRequest, CreateMergeRequestRequest, CreateReviewRequest, MergeOptions,
    MergeRequest, MergeRequestComment, MergeRequestListQuery, MergeRequestReview,
    MergeRequestStatus, UpdateMergeRequestRequest,
    Pipeline, PipelineStatus, PipelineTriggerType,
};
use crate::services::{GitService, ProjectService, CiConfigParser};

pub async fn list_merge_requests(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String)>,
    query: web::Query<MergeRequestListQuery>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    let offset = (page.saturating_sub(1) * per_page) as i64;
    
    let mrs = sqlx::query_as::<_, MergeRequest>(
        r#"
        SELECT * FROM merge_requests 
        WHERE project_id = $1
        AND ($2::merge_request_status IS NULL OR status = $2)
        AND ($3::bigint IS NULL OR author_id = $3)
        AND ($4::bigint IS NULL OR assignee_id = $4)
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
    path: web::Path<(String, String)>,
    body: web::Json<CreateMergeRequestRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    // Determine source project: use provided or default to target project (same-repo MR)
    let source_project_id = body.source_project_id.unwrap_or(project.id);
    
    // Validate source project exists and user has access
    if source_project_id != project.id {
        let source_project = sqlx::query_as::<_, (String, Option<i64>)>(
            r#"
            SELECT n.path, p.forked_from_id
            FROM projects p
            JOIN namespaces n ON p.namespace_id = n.id
            WHERE p.id = $1
            "#
        )
        .bind(source_project_id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Source project not found".to_string()))?;
        
        let (_, forked_from_id) = source_project;
        
        // Verify this is actually a fork of the target project
        if forked_from_id != Some(project.id) {
            return Err(AppError::BadRequest(
                "Source project must be a fork of the target project".to_string()
            ));
        }
    }
    
    // Get next IID for this project
    let next_iid: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(iid), 0) + 1 FROM merge_requests WHERE project_id = $1"
    )
    .bind(project.id)
    .fetch_one(pool.get_ref())
    .await?;
    
    let now = Utc::now();
    let status = if body.is_draft.unwrap_or(false) {
        MergeRequestStatus::Draft
    } else {
        MergeRequestStatus::Open
    };
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        r#"
        INSERT INTO merge_requests 
        (project_id, source_project_id, iid, title, description, source_branch, target_branch, status, author_id, assignee_id, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(source_project_id)
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
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, iid) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
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
    let can_merge = GitService::can_merge(config.get_ref(), &project.owner_name, &project.name, &mr.source_branch, &mr.target_branch).await.unwrap_or(false);
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
    path: web::Path<(String, String, i64)>,
    body: web::Json<UpdateMergeRequestRequest>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, iid) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
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
    path: web::Path<(String, String, i64)>,
    body: web::Json<MergeOptions>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (namespace, project_name, iid) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
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
    
    // Handle cross-repository merge (from fork)
    if mr.source_project_id != project.id {
        // Get source project info
        let source_project = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT n.path, p.name
            FROM projects p
            JOIN namespaces n ON p.namespace_id = n.id
            WHERE p.id = $1
            "#
        )
        .bind(mr.source_project_id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Source project not found".to_string()))?;
        
        let (source_namespace, source_project_name) = source_project;
        
        // Fetch from the fork and merge
        GitService::fetch_and_merge_from_fork(
            config.get_ref(),
            &project.owner_name,
            &project.name,
            &source_namespace,
            &source_project_name,
            &mr.source_branch,
            &mr.target_branch,
        ).await?;
    } else {
        // Same-repo merge
        if !GitService::can_merge(config.get_ref(), &project.owner_name, &project.name, &mr.source_branch, &mr.target_branch).await? {
            return Err(AppError::Conflict("Cannot merge due to conflicts".to_string()));
        }
    }
    
    // Get user info for the merge commit
    let user = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT username, email, display_name FROM users WHERE id = $1"
    )
    .bind(claims.user_id)
    .fetch_one(pool.get_ref())
    .await?;
    
    let (username, email, display_name) = user;
    let author_name = display_name.unwrap_or(username);
    
    // Generate merge commit message
    let merge_message = format!(
        "Merge branch '{}' into '{}'\n\nMerge request !{}: {}",
        mr.source_branch, mr.target_branch, mr.iid, mr.title
    );
    
    // Perform the actual Git merge
    let merge_commit_sha = GitService::perform_merge(
        config.get_ref(),
        &project.owner_name,
        &project.name,
        &mr.source_branch,
        &mr.target_branch,
        &merge_message,
        &author_name,
        &email,
    ).await?;
    
    // Optionally delete source branch after merge (only for same-repo MRs)
    if body.delete_source_branch.unwrap_or(false) && mr.source_project_id == project.id {
        if let Err(e) = GitService::delete_branch(config.get_ref(), &project.owner_name, &project.name, &mr.source_branch).await {
            // Log but don't fail the merge
            eprintln!("Warning: Failed to delete source branch after merge: {}", e);
        }
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
    
    // Trigger CI/CD pipeline for merged branch
    if let Err(e) = try_trigger_pipeline_for_merge(
        pool.get_ref(),
        config.get_ref(),
        project.id,
        claims.user_id,
        &mr.target_branch,
        &merge_commit_sha,
    ).await {
        // Log error but don't fail the merge
        log::warn!("Failed to trigger CI/CD after merge: {}", e);
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "merge_request": updated_mr,
        "merge_commit_sha": merge_commit_sha
    })))
}

/// Try to trigger CI/CD pipeline after merge request is merged
async fn try_trigger_pipeline_for_merge(
    pool: &PgPool,
    config: &AppConfig,
    project_id: i64,
    user_id: i64,
    target_branch: &str,
    merge_commit_sha: &str,
) -> AppResult<()> {
    // Get project info
    let project = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT n.path, p.name
        FROM projects p
        JOIN namespaces n ON p.namespace_id = n.id
        WHERE p.id = $1
        "#
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    let (namespace_path, project_name) = project;

    // Try to parse CI configuration
    let ci_config = match CiConfigParser::parse_from_repo(config, &namespace_path, &project_name, merge_commit_sha).await {
        Ok(config) => config,
        Err(_) => return Ok(()), // No CI config is not an error
    };

    // Check if there are any jobs
    if ci_config.jobs.is_empty() {
        return Ok(());
    }

    let now = Utc::now();
    let ref_name = target_branch.to_string();

    // Create pipeline
    let pipeline = sqlx::query_as::<_, Pipeline>(
        r#"
        INSERT INTO pipelines
        (project_id, ref_name, commit_sha, status, trigger_type, triggered_by, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        RETURNING *
        "#
    )
    .bind(project_id)
    .bind(&ref_name)
    .bind(merge_commit_sha)
    .bind(PipelineStatus::Pending)
    .bind(PipelineTriggerType::MergeRequest)
    .bind(user_id)
    .bind(now)
    .fetch_one(pool)
    .await?;

    log::info!("Created pipeline {} for MR merge on {}", pipeline.id, ref_name);

    // Create jobs for this pipeline
    let mut jobs_created = 0;
    for (job_name, job_def) in &ci_config.jobs {
        // Check if job should run on this ref
        if !CiConfigParser::should_run_job(job_def, &ref_name) {
            continue;
        }

        // Build job config JSON
        let job_config = serde_json::json!({
            "script": job_def.script,
            "before_script": job_def.before_script.as_ref().or(ci_config.before_script.as_ref()),
            "after_script": job_def.after_script.as_ref().or(ci_config.after_script.as_ref()),
            "variables": job_def.variables.as_ref().or(ci_config.variables.as_ref()),
            "artifacts": job_def.artifacts,
            "cache": job_def.cache,
            "retry": job_def.retry,
            "timeout": job_def.timeout,
            "tags": job_def.tags,
            "needs": job_def.needs,
        });

        // Insert job into database
        sqlx::query(
            r#"
            INSERT INTO jobs
            (pipeline_id, project_id, name, stage, status, config, allow_failure, when_condition, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7, $8, $8)
            "#
        )
        .bind(pipeline.id)
        .bind(project_id)
        .bind(job_name)
        .bind(&job_def.stage)
        .bind(&job_config)
        .bind(job_def.allow_failure)
        .bind(&job_def.when)
        .bind(now)
        .execute(pool)
        .await?;

        jobs_created += 1;
    }

    log::info!("Created {} jobs for pipeline {}", jobs_created, pipeline.id);

    // If no jobs were created, mark pipeline as skipped
    if jobs_created == 0 {
        sqlx::query(
            r#"
            UPDATE pipelines
            SET status = 'skipped', updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(pipeline.id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn close(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (namespace, project_name, iid) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
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
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, iid) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
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
    path: web::Path<(String, String, i64)>,
    body: web::Json<CreateCommentRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (namespace, project_name, iid) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        "SELECT * FROM merge_requests WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    let now = Utc::now();
    
    let comment = sqlx::query_as::<_, MergeRequestComment>(
        r#"
        INSERT INTO merge_request_comments 
        (merge_request_id, author_id, content, line_number, file_path, parent_id, is_resolved, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, false, $7, $7)
        RETURNING *
        "#
    )
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
    path: web::Path<(String, String, i64)>,
    body: web::Json<CreateReviewRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (namespace, project_name, iid) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    let mr = sqlx::query_as::<_, MergeRequest>(
        "SELECT * FROM merge_requests WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Merge request not found".to_string()))?;
    
    let now = Utc::now();
    
    let review = sqlx::query_as::<_, MergeRequestReview>(
        r#"
        INSERT INTO merge_request_reviews 
        (merge_request_id, reviewer_id, status, comment, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $5)
        ON CONFLICT (merge_request_id, reviewer_id) 
        DO UPDATE SET status = $3, comment = $4, updated_at = $5
        RETURNING *
        "#
    )
    .bind(mr.id)
    .bind(claims.user_id)
    .bind(&body.status)
    .bind(&body.comment)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(review))
}
