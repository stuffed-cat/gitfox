use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;


use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::validate_token;
use crate::models::{
    Pipeline, PipelineJob, PipelineJobLog, PipelineListQuery, PipelineStatus,
    PipelineTriggerType, TriggerPipelineRequest,
};
use crate::queue::{messages::PipelineTriggerMessage, RedisMessageQueue, QUEUE_PIPELINE};
use crate::services::ProjectService;

pub async fn list_pipelines(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String)>,
    query: web::Query<PipelineListQuery>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    let offset = (page.saturating_sub(1) * per_page) as i64;
    
    let pipelines = sqlx::query_as::<_, Pipeline>(
        r#"
        SELECT * FROM pipelines 
        WHERE project_id = $1
        AND ($2::pipeline_status IS NULL OR status = $2)
        AND ($3::text IS NULL OR ref_name = $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
        "#
    )
    .bind(project.id)
    .bind(&query.status)
    .bind(&query.ref_name)
    .bind(per_page as i64)
    .bind(offset)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(pipelines))
}

pub async fn trigger_pipeline(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    queue: web::Data<RedisMessageQueue>,
    path: web::Path<(String, String)>,
    body: web::Json<TriggerPipelineRequest>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    let now = Utc::now();
    
    // Get commit SHA from ref
    let repo = crate::services::GitService::open_repository(config.get_ref(), &project.owner_name, &project.name)?;
    let commits = crate::services::GitService::get_commits(&repo, &body.ref_name, None, 1, 1)?;
    let commit_sha = commits.first()
        .map(|c| c.sha.clone())
        .ok_or_else(|| AppError::NotFound("Reference not found".to_string()))?;
    
    let pipeline = sqlx::query_as::<_, Pipeline>(
        r#"
        INSERT INTO pipelines 
        (project_id, ref_name, commit_sha, status, trigger_type, triggered_by, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(&body.ref_name)
    .bind(&commit_sha)
    .bind(PipelineStatus::Pending)
    .bind(PipelineTriggerType::Manual)
    .bind(claims.user_id)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    // Queue pipeline execution
    let message = PipelineTriggerMessage {
        pipeline_id: pipeline.id,
        project_id: project.id,
        ref_name: body.ref_name.clone(),
        commit_sha,
    };
    queue.publish(QUEUE_PIPELINE, &message).await?;
    
    Ok(HttpResponse::Created().json(pipeline))
}

pub async fn get_pipeline(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, pipeline_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    let pipeline = sqlx::query_as::<_, Pipeline>(
        "SELECT * FROM pipelines WHERE id = $1 AND project_id = $2"
    )
    .bind(pipeline_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Pipeline not found".to_string()))?;
    
    let jobs = sqlx::query_as::<_, PipelineJob>(
        "SELECT * FROM pipeline_jobs WHERE pipeline_id = $1 ORDER BY created_at"
    )
    .bind(pipeline.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "pipeline": pipeline,
        "jobs": jobs
    })))
}

pub async fn cancel_pipeline(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, pipeline_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    let pipeline = sqlx::query_as::<_, Pipeline>(
        r#"
        UPDATE pipelines
        SET status = 'canceled', updated_at = NOW()
        WHERE id = $1 AND project_id = $2 AND status IN ('pending', 'running')
        RETURNING *
        "#
    )
    .bind(pipeline_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Pipeline not found or cannot be canceled".to_string()))?;
    
    // Cancel all pending/running jobs
    sqlx::query(
        "UPDATE pipeline_jobs SET status = 'canceled', updated_at = NOW() WHERE pipeline_id = $1 AND status IN ('pending', 'running')"
    )
    .bind(pipeline.id)
    .execute(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(pipeline))
}

pub async fn retry_pipeline(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    queue: web::Data<RedisMessageQueue>,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    
    let (namespace, project_name, pipeline_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    let old_pipeline = sqlx::query_as::<_, Pipeline>(
        "SELECT * FROM pipelines WHERE id = $1 AND project_id = $2"
    )
    .bind(pipeline_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Pipeline not found".to_string()))?;
    
    // Create new pipeline
    let now = Utc::now();
    
    let pipeline = sqlx::query_as::<_, Pipeline>(
        r#"
        INSERT INTO pipelines 
        (project_id, ref_name, commit_sha, status, trigger_type, triggered_by, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(&old_pipeline.ref_name)
    .bind(&old_pipeline.commit_sha)
    .bind(PipelineStatus::Pending)
    .bind(PipelineTriggerType::Manual)
    .bind(claims.user_id)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    // Queue pipeline execution
    let message = PipelineTriggerMessage {
        pipeline_id: pipeline.id,
        project_id: project.id,
        ref_name: old_pipeline.ref_name.clone(),
        commit_sha: old_pipeline.commit_sha.clone(),
    };
    queue.publish(QUEUE_PIPELINE, &message).await?;
    
    Ok(HttpResponse::Created().json(pipeline))
}

pub async fn list_jobs(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, pipeline_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    // Verify pipeline belongs to project
    let _ = sqlx::query_as::<_, Pipeline>(
        "SELECT * FROM pipelines WHERE id = $1 AND project_id = $2"
    )
    .bind(pipeline_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Pipeline not found".to_string()))?;
    
    let jobs = sqlx::query_as::<_, PipelineJob>(
        "SELECT * FROM pipeline_jobs WHERE pipeline_id = $1 ORDER BY stage, created_at"
    )
    .bind(pipeline_id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(jobs))
}

pub async fn get_job_log(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, pipeline_id, job_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    // Verify pipeline belongs to project
    let _ = sqlx::query_as::<_, Pipeline>(
        "SELECT * FROM pipelines WHERE id = $1 AND project_id = $2"
    )
    .bind(pipeline_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Pipeline not found".to_string()))?;
    
    let logs = sqlx::query_as::<_, PipelineJobLog>(
        "SELECT * FROM pipeline_job_logs WHERE job_id = $1 ORDER BY created_at"
    )
    .bind(job_id)
    .fetch_all(pool.get_ref())
    .await?;
    
    let combined_log: String = logs.iter().map(|l| l.content.as_str()).collect::<Vec<_>>().join("\n");
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "job_id": job_id,
        "log": combined_log
    })))
}
