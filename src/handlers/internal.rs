//! Internal API handlers for GitFox Shell
//!
//! These endpoints are used by gitfox-shell for SSH authentication and authorization.
//! They should be protected by a secret token and not exposed publicly.

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use log::{debug, info, warn};

use crate::config::Config;
use crate::error::AppError;
use crate::models::{
    AccessCheckRequest, AccessCheckResponse, CheckRefUpdateRequest, CheckRefUpdateResponse,
    FindKeyRequest, PostReceiveRequest, SshKey, SshKeyInternalInfo,
    Pipeline, PipelineStatus, PipelineTriggerType,
};
use crate::services::{GitService, CiConfigParser};

/// Verify the internal API token
fn verify_shell_token(req: &HttpRequest, config: &Config) -> Result<(), AppError> {
    let token = req
        .headers()
        .get("X-GitFox-Shell-Token")
        .and_then(|v| v.to_str().ok());

    match token {
        Some(t) if t == config.shell_secret => Ok(()),
        _ => {
            warn!("Invalid or missing shell token");
            Err(AppError::Unauthorized("Invalid shell token".to_string()))
        }
    }
}

/// Check if a key has access to a repository
/// POST /api/internal/allowed
pub async fn check_allowed(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<AccessCheckRequest>,
) -> Result<HttpResponse, AppError> {
    verify_shell_token(&req, &config)?;

    debug!(
        "Checking access for key_id={}, repo={}, action={}",
        body.key_id, body.repo_path, body.action
    );

    // Parse key_id (format: "key-123")
    let key_id: i64 = body
        .key_id
        .strip_prefix("key-")
        .and_then(|id| id.parse().ok())
        .ok_or_else(|| AppError::BadRequest("Invalid key_id format".to_string()))?;

    // Get the SSH key and user
    let key = sqlx::query_as::<_, SshKey>(
        r#"SELECT * FROM ssh_keys WHERE id = $1"#
    )
    .bind(key_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| {
        info!("SSH key not found: {}", key_id);
        AppError::Unauthorized("SSH key not found".to_string())
    })?;

    // Check if key is expired
    if let Some(expires_at) = key.expires_at {
        if expires_at < chrono::Utc::now() {
            return Ok(HttpResponse::Forbidden().json(AccessCheckResponse::denied(
                "SSH key has expired",
            )));
        }
    }

    // Get user info
    let user = sqlx::query_as::<_, (i64, String, bool)>(
        r#"SELECT id, username, is_active FROM users WHERE id = $1"#
    )
    .bind(key.user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::Unauthorized("User not found".to_string()))?;

    let (user_id, username, is_active) = user;

    if !is_active {
        return Ok(HttpResponse::Forbidden().json(AccessCheckResponse::denied(
            "User account is deactivated",
        )));
    }

    // Parse repository path (format: "owner/project")
    let repo_path = &body.repo_path;
    
    // Split into owner and project name
    let parts: Vec<&str> = repo_path.split('/').collect();
    if parts.len() != 2 {
        return Ok(HttpResponse::BadRequest().json(AccessCheckResponse::denied(
            "Invalid repository path format. Expected: owner/project",
        )));
    }
    let (owner_name, project_name) = (parts[0], parts[1]);

    // Find the project by owner username and project name
    let project = sqlx::query_as::<_, (i64, String, i64)>(
        r#"
        SELECT p.id, p.visibility::text, p.owner_id
        FROM projects p
        JOIN users u ON p.owner_id = u.id
        WHERE LOWER(u.username) = LOWER($1) AND LOWER(p.name) = LOWER($2)
        "#,
    )
    .bind(owner_name)
    .bind(project_name)
    .fetch_optional(pool.get_ref())
    .await?;

    let (project_id, visibility, owner_id) = match project {
        Some(p) => p,
        None => {
            info!("Repository not found: {}", repo_path);
            return Ok(HttpResponse::NotFound().json(AccessCheckResponse::denied(
                "Repository not found",
            )));
        }
    };

    // Check access based on visibility and membership
    let needs_write = body.action == "git-receive-pack";
    let can_access: bool;
    let can_write: bool;

    // Check if user is owner
    if user_id == owner_id {
        can_access = true;
        can_write = true;
    } else {
        // Check project membership
        let membership = sqlx::query_as::<_, (String,)>(
            r#"SELECT role::text FROM project_members WHERE project_id = $1 AND user_id = $2"#
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(pool.get_ref())
        .await?;

        match membership {
            Some((role,)) => {
                can_access = true;
                // Check write permission based on role
                can_write = matches!(
                    role.as_str(),
                    "owner" | "maintainer" | "developer"
                );
            }
            None => {
                // No membership, check if project is public
                if visibility == "public" || visibility == "internal" {
                    can_access = true;
                    can_write = false;
                } else {
                    can_access = false;
                    can_write = false;
                }
            }
        }
    }

    if !can_access {
        info!(
            "Access denied for user {} on repo {}",
            username, repo_path
        );
        return Ok(HttpResponse::Forbidden().json(AccessCheckResponse::denied(
            "You don't have access to this repository",
        )));
    }

    if needs_write && !can_write {
        info!(
            "Write access denied for user {} on repo {}",
            username, repo_path
        );
        return Ok(HttpResponse::Forbidden().json(AccessCheckResponse::denied(
            "You don't have write access to this repository",
        )));
    }

    // Update last_used_at for the SSH key
    let _ = sqlx::query("UPDATE ssh_keys SET last_used_at = NOW() WHERE id = $1")
        .bind(key_id)
        .execute(pool.get_ref())
        .await;

    info!(
        "Access granted for user {} on repo {} (write: {})",
        username, repo_path, can_write
    );

    Ok(HttpResponse::Ok().json(AccessCheckResponse::allowed(
        user_id,
        username,
        can_write,
        Some(project_id),
        Some(config.base_url.clone()),
    )))
}

/// Get SSH key by ID
/// GET /api/internal/keys/{id}
pub async fn get_key(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    verify_shell_token(&req, &config)?;

    let key_id = path.into_inner();

    let result = sqlx::query_as::<_, (i64, i64, String, String, String)>(
        r#"
        SELECT k.id, k.user_id, u.username, k.key_type, k.public_key
        FROM ssh_keys k
        JOIN users u ON k.user_id = u.id
        WHERE k.id = $1
        "#,
    )
    .bind(key_id)
    .fetch_optional(pool.get_ref())
    .await?;

    match result {
        Some((id, user_id, username, key_type, key)) => {
            Ok(HttpResponse::Ok().json(SshKeyInternalInfo {
                id,
                user_id,
                username,
                key_type,
                key,
            }))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "SSH key not found"
        }))),
    }
}

/// Find SSH key by fingerprint
/// POST /api/internal/keys/find
pub async fn find_key_by_fingerprint(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<FindKeyRequest>,
) -> Result<HttpResponse, AppError> {
    verify_shell_token(&req, &config)?;

    debug!("Looking up key by fingerprint: {}", body.fingerprint);

    let result = sqlx::query_as::<_, (i64, i64, String, String, String)>(
        r#"
        SELECT k.id, k.user_id, u.username, k.key_type, k.public_key
        FROM ssh_keys k
        JOIN users u ON k.user_id = u.id
        WHERE k.fingerprint = $1
          AND (k.expires_at IS NULL OR k.expires_at > NOW())
          AND u.is_active = true
        "#,
    )
    .bind(&body.fingerprint)
    .fetch_optional(pool.get_ref())
    .await?;

    match result {
        Some((id, user_id, username, key_type, key)) => {
            info!("Found key {} for user {}", id, username);
            Ok(HttpResponse::Ok().json(SshKeyInternalInfo {
                id,
                user_id,
                username,
                key_type,
                key,
            }))
        }
        None => {
            debug!("Key not found for fingerprint: {}", body.fingerprint);
            Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "SSH key not found"
            })))
        }
    }
}

/// Handle post-receive hook notification
/// POST /api/internal/post-receive
pub async fn post_receive(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<PostReceiveRequest>,
) -> Result<HttpResponse, AppError> {
    verify_shell_token(&req, &config)?;

    info!(
        "Post-receive for repo {} from user {}: {} changes",
        body.repository,
        body.user_id,
        body.changes.len()
    );

    // Get project_id from body or find it from repository path
    let project_id: i64 = if let Some(ref pid) = body.project_id {
        pid.parse().unwrap_or(0)
    } else {
        // Find project from repository path
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT p.id
            FROM projects p
            JOIN namespaces n ON p.namespace_id = n.id
            WHERE CONCAT(n.path, '/', p.name) = $1
            "#,
        )
        .bind(&body.repository)
        .fetch_optional(pool.get_ref())
        .await?
        .unwrap_or(0)
    };

    let user_id: i64 = body.user_id.parse().unwrap_or(0);

    // Process each ref change
    for change in &body.changes {
        debug!(
            "Ref update: {} -> {} ({})",
            change.old_sha, change.new_sha, change.ref_name
        );

        // Skip if branch/tag is deleted
        if change.new_sha == "0000000000000000000000000000000000000000" {
            debug!("Ref {} deleted, skipping CI", change.ref_name);
            continue;
        }

        // Check if this is a branch or tag update
        if change.ref_name.starts_with("refs/heads/") || change.ref_name.starts_with("refs/tags/") {
            // Try to trigger CI/CD pipeline
            if project_id > 0 {
                match try_trigger_pipeline(
                    pool.get_ref(),
                    config.get_ref(),
                    project_id,
                    user_id,
                    &change.ref_name,
                    &change.new_sha,
                ).await {
                    Ok(_) => info!("Pipeline triggered for {} at {}", change.ref_name, change.new_sha),
                    Err(e) => warn!("Failed to trigger pipeline: {}", e),
                }
            }
            
            // Update branch record if it's a branch
            if change.ref_name.starts_with("refs/heads/") {
                let branch_name = change.ref_name.strip_prefix("refs/heads/").unwrap();
                debug!("Branch {} updated to {}", branch_name, change.new_sha);
            }
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "ok"
    })))
}

/// Check if a ref update is allowed
/// POST /api/internal/check-ref-update
pub async fn check_ref_update(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<CheckRefUpdateRequest>,
) -> Result<HttpResponse, AppError> {
    verify_shell_token(&req, &config)?;

    debug!(
        "Checking ref update: {} ({}) for repo {}",
        body.ref_name, body.change_type, body.repository
    );

    let user_id: i64 = body.user_id.parse().unwrap_or(0);

    // Find project
    let project = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT p.id, p.owner_id
        FROM projects p
        JOIN namespaces n ON p.namespace_id = n.id
        WHERE CONCAT(n.path, '/', p.name) = $1
        "#,
    )
    .bind(&body.repository)
    .fetch_optional(pool.get_ref())
    .await?;

    let (project_id, owner_id) = match project {
        Some(p) => p,
        None => {
            return Ok(HttpResponse::Ok().json(CheckRefUpdateResponse {
                allowed: true,
                message: None,
            }));
        }
    };

    // Check if this is a protected branch/tag
    let ref_name = &body.ref_name;
    let is_branch = ref_name.starts_with("refs/heads/");
    let _is_tag = ref_name.starts_with("refs/tags/");

    if is_branch {
        let branch_name = ref_name.strip_prefix("refs/heads/").unwrap();

        // Check branch protection rules
        let protection = sqlx::query_as::<_, (bool, bool)>(
            r#"
            SELECT allow_force_push, allow_deletion
            FROM branch_protection_rules
            WHERE project_id = $1 AND $2 LIKE REPLACE(branch_pattern, '*', '%')
            ORDER BY LENGTH(branch_pattern) DESC
            LIMIT 1
            "#,
        )
        .bind(project_id)
        .bind(branch_name)
        .fetch_optional(pool.get_ref())
        .await?;

        if let Some((allow_force_push, allow_deletion)) = protection {
            // Check if user is owner or maintainer
            let is_owner = user_id == owner_id;
            let is_maintainer = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT role::text IN ('owner', 'maintainer')
                FROM project_members
                WHERE project_id = $1 AND user_id = $2
                "#,
            )
            .bind(project_id)
            .bind(user_id)
            .fetch_optional(pool.get_ref())
            .await?
            .unwrap_or(false);

            let has_admin_access = is_owner || is_maintainer;

            // Check deletion
            if body.change_type == "delete" {
                if !allow_deletion && !has_admin_access {
                    return Ok(HttpResponse::Ok().json(CheckRefUpdateResponse {
                        allowed: false,
                        message: Some(format!(
                            "Branch '{}' is protected: deletion is not allowed",
                            branch_name
                        )),
                    }));
                }
            }

            // Check force push (would need to verify if old_sha is ancestor of new_sha)
            if body.change_type == "update" && !allow_force_push && !has_admin_access {
                // For simplicity, we'll skip the force push check here
                // In production, you'd verify if it's actually a force push
            }
        }
    }

    Ok(HttpResponse::Ok().json(CheckRefUpdateResponse {
        allowed: true,
        message: None,
    }))
}

/// Try to trigger CI/CD pipeline for a push
async fn try_trigger_pipeline(
    pool: &PgPool,
    config: &Config,
    project_id: i64,
    user_id: i64,
    ref_name: &str,
    commit_sha: &str,
) -> Result<(), AppError> {
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
    .await?;

    let (namespace_path, project_name) = match project {
        Some(p) => p,
        None => {
            warn!("Project {} not found", project_id);
            return Ok(());
        }
    };

    // Open repository and parse CI config
    let repo = match GitService::open_repository(config, &namespace_path, &project_name) {
        Ok(r) => r,
        Err(e) => {
            debug!("Failed to open repository: {}", e);
            return Ok(()); // Not an error, just skip CI
        }
    };

    // Try to parse CI configuration
    let ci_config = match CiConfigParser::parse_from_repo(&repo, commit_sha) {
        Ok(config) => config,
        Err(e) => {
            debug!("No CI config or parse error: {}", e);
            return Ok(()); // No CI config is not an error
        }
    };

    // Check if there are any jobs
    if ci_config.jobs.is_empty() {
        debug!("No jobs defined in CI config");
        return Ok(());
    }

    let now = Utc::now();

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
    .bind(ref_name)
    .bind(commit_sha)
    .bind(PipelineStatus::Pending)
    .bind(PipelineTriggerType::Push)
    .bind(user_id)
    .bind(now)
    .fetch_one(pool)
    .await?;

    info!("Created pipeline {} for project {} on {}", pipeline.id, project_id, ref_name);

    // Create jobs for this pipeline
    let mut jobs_created = 0;
    for (job_name, job_def) in &ci_config.jobs {
        // Check if job should run on this ref
        if !CiConfigParser::should_run_job(job_def, ref_name) {
            debug!("Job {} skipped due to only/except rules", job_name);
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
            INSERT INTO pipeline_jobs
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

    info!("Created {} jobs for pipeline {}", jobs_created, pipeline.id);

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
        
        debug!("Pipeline {} marked as skipped (no matching jobs)", pipeline.id);
    }

    Ok(())
}

/// Configure internal API routes
pub fn configure_internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/internal")
            .route("/allowed", web::post().to(check_allowed))
            .route("/keys/{id}", web::get().to(get_key))
            .route("/keys/find", web::post().to(find_key_by_fingerprint))
            .route("/post-receive", web::post().to(post_receive))
            .route("/check-ref-update", web::post().to(check_ref_update)),
    );
}
