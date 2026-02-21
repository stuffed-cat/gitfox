use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use log::warn;
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::validate_token;
use crate::models::{
    Pipeline, PipelineJob, PipelineJobLog, PipelineListQuery, PipelineStatus,
    PipelineTriggerType, TriggerPipelineRequest,
};
use crate::queue::{messages::PipelineTriggerMessage, RedisMessageQueue, QUEUE_PIPELINE};
use crate::services::{ProjectService, ci_config::CiConfigParser};

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

    // 尝试解析 CI 配置
    let ci_result = CiConfigParser::parse_from_repo(&repo, &commit_sha);
    
    let (status, error_message) = match &ci_result {
        Ok(ci_config) => {
            // 检查是否有 jobs
            if ci_config.jobs.is_empty() {
                (PipelineStatus::Failed, Some("No jobs defined in CI configuration".to_string()))
            } else {
                (PipelineStatus::Pending, None)
            }
        }
        Err(e) => {
            // CI 配置无效
            (PipelineStatus::Failed, Some(format!("CI configuration error: {}", e)))
        }
    };
    
    let pipeline = sqlx::query_as::<_, Pipeline>(
        r#"
        INSERT INTO pipelines 
        (project_id, ref_name, commit_sha, status, trigger_type, triggered_by, error_message, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(&body.ref_name)
    .bind(&commit_sha)
    .bind(&status)
    .bind(PipelineTriggerType::Manual)
    .bind(claims.user_id)
    .bind(&error_message)
    .bind(now)
    .fetch_one(pool.get_ref())
    .await?;
    
    // 如果 CI 配置有效，创建 jobs
    if let Ok(ci_config) = ci_result {
        for (job_name, job_def) in &ci_config.jobs {
            // 检查是否应该运行这个 job
            if !CiConfigParser::should_run_job(job_def, &body.ref_name) {
                continue;
            }
            
            let job_config = serde_json::json!({
                "script": job_def.script,
                "before_script": job_def.before_script.as_ref().or(ci_config.before_script.as_ref()),
                "after_script": job_def.after_script.as_ref().or(ci_config.after_script.as_ref()),
                "variables": job_def.variables.as_ref().or(ci_config.variables.as_ref()),
                "artifacts": job_def.artifacts,
                "cache": job_def.cache,
                "retry": job_def.retry,
                "timeout": job_def.timeout,
            });
            
            sqlx::query(
                r#"
                INSERT INTO jobs
                (pipeline_id, project_id, name, stage, status, config, allow_failure, when_condition, created_at, updated_at)
                VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7, $8, $8)
                "#
            )
            .bind(pipeline.id)
            .bind(project.id)
            .bind(job_name)
            .bind(&job_def.stage)
            .bind(&job_config)
            .bind(job_def.allow_failure)
            .bind(&job_def.when)
            .bind(now)
            .execute(pool.get_ref())
            .await?;
        }
        
        // 只有在有 jobs 的情况下才发布队列消息
        let message = PipelineTriggerMessage {
            pipeline_id: pipeline.id,
            project_id: project.id,
            ref_name: body.ref_name.clone(),
            commit_sha: commit_sha.clone(),
        };
        queue.publish(QUEUE_PIPELINE, &message).await?;
    }
    
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
    
    // 主动检查并修复超时的 jobs（被查询时检测超时）
    let fixed_count = sqlx::query_scalar::<_, i64>(
        "UPDATE jobs 
         SET status = 'failed', 
             finished_at = NOW(), 
             error_message = 'Job exceeded timeout and was detected late on status query'
         WHERE pipeline_id = $1 
           AND status = 'running' 
           AND timeout_at < NOW()
         RETURNING 1"
    )
    .bind(pipeline.id)
    .fetch_all(pool.get_ref())
    .await?
    .len() as i64;
    
    if fixed_count > 0 {
        warn!("Fixed {} timed-out jobs for pipeline {} during get_pipeline query", fixed_count, pipeline.id);
        
        // 为每个被修复的 job 添加日志记录
        let _ = sqlx::query(
            "INSERT INTO job_logs (job_id, log, timestamp) 
             SELECT id, '[System] Job marked as failed: exceeded timeout', NOW()
             FROM jobs 
             WHERE pipeline_id = $1 
               AND status = 'failed' 
               AND error_message = 'Job exceeded timeout and was detected late on status query'"
        )
        .bind(pipeline.id)
        .execute(pool.get_ref())
        .await;
    }
    
    let mut jobs = sqlx::query_as::<_, PipelineJob>(
        "SELECT * FROM jobs WHERE pipeline_id = $1 ORDER BY created_at"
    )
    .bind(pipeline.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    // 计算 duration_seconds
    for job in &mut jobs {
        if let (Some(started), Some(finished)) = (job.started_at, job.finished_at) {
            job.duration_seconds = Some((finished - started).num_seconds() as i32);
        }
    }
    
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
        "UPDATE jobs SET status = 'canceled', updated_at = NOW() WHERE pipeline_id = $1 AND status IN ('pending', 'running')"
    )
    .bind(pipeline.id)
    .execute(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(pipeline))
}

pub async fn delete_pipeline(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, pipeline_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    // Verify pipeline belongs to project
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pipelines WHERE id = $1 AND project_id = $2)"
    )
    .bind(pipeline_id)
    .bind(project.id)
    .fetch_one(pool.get_ref())
    .await?;
    
    if !exists {
        return Err(AppError::NotFound("Pipeline not found".to_string()));
    }
    
    // Delete pipeline (CASCADE will delete jobs and job_logs)
    sqlx::query("DELETE FROM pipelines WHERE id = $1")
        .bind(pipeline_id)
        .execute(pool.get_ref())
        .await?;
    
    Ok(HttpResponse::NoContent().finish())
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
    
    // 主动检查并修复超时的 jobs（被查询时检测超时）
    let fixed_count = sqlx::query_scalar::<_, i64>(
        "UPDATE jobs 
         SET status = 'failed', 
             finished_at = NOW(), 
             error_message = 'Job exceeded timeout and was detected late on status query'
         WHERE pipeline_id = $1 
           AND status = 'running' 
           AND timeout_at < NOW()
         RETURNING 1"
    )
    .bind(pipeline_id)
    .fetch_all(pool.get_ref())
    .await?
    .len() as i64;
    
    if fixed_count > 0 {
        warn!("Fixed {} timed-out jobs for pipeline {} during list_jobs query", fixed_count, pipeline_id);
        
        // 为每个被修复的 job 添加日志记录
        let _ = sqlx::query(
            "INSERT INTO job_logs (job_id, log, timestamp) 
             SELECT id, '[System] Job marked as failed: exceeded timeout', NOW()
             FROM jobs 
             WHERE pipeline_id = $1 
               AND status = 'failed' 
               AND error_message = 'Job exceeded timeout and was detected late on status query'"
        )
        .bind(pipeline_id)
        .execute(pool.get_ref())
        .await;
    }
    
    let mut jobs = sqlx::query_as::<_, PipelineJob>(
        "SELECT * FROM jobs WHERE pipeline_id = $1 ORDER BY stage, created_at"
    )
    .bind(pipeline_id)
    .fetch_all(pool.get_ref())
    .await?;
    
    // 计算 duration_seconds
    for job in &mut jobs {
        if let (Some(started), Some(finished)) = (job.started_at, job.finished_at) {
            job.duration_seconds = Some((finished - started).num_seconds() as i32);
        }
    }
    
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
        "SELECT * FROM job_logs WHERE job_id = $1 ORDER BY created_at"
    )
    .bind(job_id)
    .fetch_all(pool.get_ref())
    .await?;
    
    let combined_log: String = logs.iter().map(|l| l.output.as_str()).collect::<Vec<_>>().join("\n");
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "job_id": job_id,
        "log": combined_log
    })))
}

pub async fn download_job_log(
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
        "SELECT * FROM job_logs WHERE job_id = $1 ORDER BY created_at"
    )
    .bind(job_id)
    .fetch_all(pool.get_ref())
    .await?;
    
    let combined_log: String = logs.iter().map(|l| l.output.as_str()).collect::<Vec<_>>().join("\n");
    
    Ok(HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .insert_header(("Content-Disposition", format!("attachment; filename=\"job-{}.log\"", job_id)))
        .body(combined_log))
}
