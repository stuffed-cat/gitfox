use crate::error::{AppError, AppResult};
use crate::config::Config;
use crate::middleware::auth::{AdminUser, AuthenticatedUser};
use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Message as ActixMessage, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(90);

/// WebSocket Actor for Runner connection
pub struct RunnerWebSocket {
    id: Option<i64>,
    pool: web::Data<PgPool>,
    redis: web::Data<deadpool_redis::Pool>,
    config: web::Data<Config>,
    hb: Instant,
    runner_manager: Arc<RwLock<RunnerManager>>,
}

impl RunnerWebSocket {
    pub fn new(pool: web::Data<PgPool>, redis: web::Data<deadpool_redis::Pool>, config: web::Data<Config>, runner_manager: Arc<RwLock<RunnerManager>>) -> Self {
        Self {
            id: None,
            pool,
            redis,
            config,
            hb: Instant::now(),
            runner_manager,
        }
    }

    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                warn!("Runner heartbeat timeout, disconnecting");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

impl Actor for RunnerWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Runner WebSocket connection established");
        self.start_heartbeat(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("Runner WebSocket connection closed");
        if let Some(runner_id) = self.id {
            // Mark runner as offline
            let pool = self.pool.clone();
            actix_web::rt::spawn(async move {
                let _ = sqlx::query("UPDATE runners SET status = 'offline' WHERE id = $1")
                    .bind(runner_id)
                    .execute(pool.get_ref())
                    .await;
            });

            // Remove from manager
            let manager = self.runner_manager.clone();
            actix_web::rt::spawn(async move {
                manager.write().await.remove_runner(runner_id);
            });
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for RunnerWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                debug!("Received message: {}", text);
                match serde_json::from_str::<RunnerMessage>(&text) {
                    Ok(runner_msg) => {
                        ctx.address().do_send(ProcessMessage(runner_msg));
                    }
                    Err(e) => {
                        error!("Failed to parse runner message: {}", e);
                    }
                }
            }
            Ok(ws::Message::Binary(_)) => {
                warn!("Unexpected binary message from runner");
            }
            Ok(ws::Message::Close(reason)) => {
                info!("Runner closed connection: {:?}", reason);
                ctx.stop();
            }
            Err(e) => {
                error!("WebSocket protocol error: {}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

// Actor message to send text to WebSocket
#[derive(ActixMessage)]
#[rtype(result = "()")]
pub struct SendText(pub String);

impl Handler<SendText> for RunnerWebSocket {
    type Result = ();

    fn handle(&mut self, msg: SendText, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

// Actor message to process runner messages
#[derive(ActixMessage)]
#[rtype(result = "()")]
struct ProcessMessage(RunnerMessage);

impl Handler<ProcessMessage> for RunnerWebSocket {
    type Result = ();

    fn handle(&mut self, msg: ProcessMessage, ctx: &mut Self::Context) {
        let runner_msg = msg.0;
        let pool = self.pool.clone();
        let runner_manager = self.runner_manager.clone();
        let addr = ctx.address();
        let current_runner_id = self.id;

        match runner_msg {
            RunnerMessage::Register { token, name, tags, executor } => {
                actix_web::rt::spawn(async move {
                    match register_runner(pool.get_ref(), &token, &name, tags, &executor).await {
                        Ok(runner_id) => {
                            info!("Runner '{}' registered with ID: {}", name, runner_id);
                            
                            // Add to manager
                            runner_manager.write().await.add_runner(runner_id, addr.clone());

                            // Send success response
                            let response = ServerMessage::Registered { runner_id };
                            if let Ok(json) = serde_json::to_string(&response) {
                                addr.do_send(SendText(json));
                            }

                            // Store runner ID
                            addr.do_send(SetRunnerId(runner_id));
                        }
                        Err(e) => {
                            error!("Runner registration failed: {}", e);
                            let response = ServerMessage::Error {
                                message: format!("Registration failed: {}", e),
                            };
                            if let Ok(json) = serde_json::to_string(&response) {
                                addr.do_send(SendText(json));
                            }
                        }
                    }
                });
            }
            RunnerMessage::JobUpdate { job_id, status, exit_code, error_message } => {
                let redis_clone = self.redis.clone();
                let pool_clone = self.pool.clone();
                let instance_id = self.config.instance_id.clone();
                actix_web::rt::spawn(async move {
                    // Update job status in database
                    let instance_id_ref = if status == "running" { Some(instance_id.as_str()) } else { None };
                    if let Err(e) = update_job_status(pool.get_ref(), &redis_clone, job_id, &status, exit_code, error_message.as_deref(), instance_id_ref).await {
                        error!("Failed to update job status: {}", e);
                    }
                    
                    // Update pipeline status based on all jobs
                    if let Err(e) = update_pipeline_status(pool.get_ref(), job_id).await {
                        error!("Failed to update pipeline status: {}", e);
                    }
                    
                    // Handle Redis timeout key based on status
                    if status == "running" {
                        // Job started: set Redis timeout key
                        match get_job_timeout(&pool_clone, &redis_clone, job_id).await {
                            Ok(timeout) => {
                                if let Err(e) = set_job_timeout_in_redis(&redis_clone, job_id, timeout).await {
                                    error!("Failed to set Redis timeout for job {}: {}", job_id, e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to get timeout for job {}: {}", job_id, e);
                            }
                        }
                    } else if status == "success" || status == "failed" || status == "canceled" || status == "skipped" {
                        // Job finished: clear Redis timeout key
                        if let Err(e) = clear_job_timeout_in_redis(&redis_clone, job_id).await {
                            error!("Failed to clear Redis timeout for job {}: {}", job_id, e);
                        }
                    }
                    
                    let ack = ServerMessage::Ack;
                    if let Ok(json) = serde_json::to_string(&ack) {
                        addr.do_send(SendText(json));
                    }
                });
            }
            RunnerMessage::JobLog { job_id, output } => {
                actix_web::rt::spawn(async move {
                    if let Err(e) = append_job_log(pool.get_ref(), job_id, &output).await {
                        error!("Failed to append job log: {}", e);
                    }
                });
            }
            RunnerMessage::Heartbeat => {
                self.hb = Instant::now();
                if let Some(runner_id) = current_runner_id {
                    actix_web::rt::spawn(async move {
                        let _ = update_runner_contact(pool.get_ref(), runner_id).await;
                    });
                }
            }
            RunnerMessage::RequestJob => {
                if let Some(runner_id) = current_runner_id {
                    actix_web::rt::spawn(async move {
                        match get_pending_job(pool.get_ref(), runner_id).await {
                            Ok(Some(job)) => {
                                info!("Assigning job {} to runner {}", job.id, runner_id);
                                let response = ServerMessage::JobAssigned { job };
                                if let Ok(json) = serde_json::to_string(&response) {
                                    addr.do_send(SendText(json));
                                }
                            }
                            Ok(None) => {
                                let response = ServerMessage::NoJobs;
                                if let Ok(json) = serde_json::to_string(&response) {
                                    addr.do_send(SendText(json));
                                }
                            }
                            Err(e) => {
                                warn!("Failed to get pending job: {}", e);
                            }
                        }
                    });
                }
            }
        }
    }
}

// Message to set runner ID
#[derive(ActixMessage)]
#[rtype(result = "()")]
struct SetRunnerId(i64);

impl Handler<SetRunnerId> for RunnerWebSocket {
    type Result = ();

    fn handle(&mut self, msg: SetRunnerId, _ctx: &mut Self::Context) {
        self.id = Some(msg.0);
    }
}

// Database operations
async fn register_runner(
    pool: &PgPool,
    token: &str,
    name: &str,
    tags: Vec<String>,
    executor: &str,
) -> AppResult<i64> {
    // 使用认证 token 查找并更新 runner 状态
    let result = sqlx::query_scalar::<_, i64>(
        r#"
        UPDATE runners
        SET name = $1,
            tags = $2,
            executor = $3,
            status = 'online',
            last_contact = NOW(),
            updated_at = NOW()
        WHERE token = $4 AND is_active = true
        RETURNING id
        "#
    )
    .bind(name)
    .bind(&tags)
    .bind(executor)
    .bind(token)
    .fetch_optional(pool)
    .await?;

    if let Some(runner_id) = result {
        Ok(runner_id)
    } else {
        Err(crate::error::AppError::Unauthorized(
            "Invalid authentication token".to_string()
        ))
    }
}

async fn update_job_status(
    pool: &PgPool,
    redis: &deadpool_redis::Pool,
    job_id: i64,
    status: &str,
    exit_code: Option<i32>,
    error_message: Option<&str>,
    instance_id: Option<&str>,
) -> AppResult<()> {
    if status == "running" {
        // Calculate timeout_at when job starts
        let timeout_seconds = get_job_timeout(pool, redis, job_id).await.unwrap_or(3600);
        
        sqlx::query(
            r#"
            UPDATE jobs 
            SET status = $1, 
                started_at = NOW(),
                timeout_at = NOW() + ($2 || ' seconds')::interval,
                watcher_instance = $3
            WHERE id = $4
            "#
        )
        .bind(status)
        .bind(timeout_seconds.to_string())
        .bind(instance_id)
        .bind(job_id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            r#"
            UPDATE jobs 
            SET status = $1, 
                finished_at = NOW(),
                updated_at = NOW(),
                error_message = $2,
                watcher_instance = NULL
            WHERE id = $3
            "#
        )
        .bind(status)
        .bind(error_message)
        .bind(job_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Update pipeline status based on the status of all its jobs
async fn update_pipeline_status(pool: &PgPool, job_id: i64) -> AppResult<()> {
    // Get pipeline_id for this job
    let pipeline_id: i64 = sqlx::query_scalar("SELECT pipeline_id FROM jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(pool)
        .await?;
    
    // Get all jobs for this pipeline
    #[derive(sqlx::FromRow)]
    struct JobStatus {
        status: String,
        allow_failure: bool,
    }
    
    let jobs: Vec<JobStatus> = sqlx::query_as(
        "SELECT status, allow_failure FROM jobs WHERE pipeline_id = $1"
    )
    .bind(pipeline_id)
    .fetch_all(pool)
    .await?;
    
    if jobs.is_empty() {
        return Ok(());
    }
    
    // Determine overall pipeline status
    let new_status = if jobs.iter().any(|j| j.status == "running") {
        "running"
    } else if jobs.iter().any(|j| j.status == "pending") {
        "pending"
    } else if jobs.iter().any(|j| j.status == "canceled") {
        "canceled"
    } else if jobs.iter().any(|j| j.status == "failed" && !j.allow_failure) {
        "failed"
    } else if jobs.iter().all(|j| matches!(j.status.as_str(), "success" | "skipped") || (j.status == "failed" && j.allow_failure)) {
        "success"
    } else {
        // Some jobs are still pending or unknown state
        "pending"
    };
    
    // Calculate duration if pipeline is complete
    if matches!(new_status, "success" | "failed" | "canceled") {
        sqlx::query(
            r#"
            UPDATE pipelines 
            SET status = $1,
                finished_at = NOW(),
                duration_seconds = EXTRACT(EPOCH FROM (NOW() - started_at))::INTEGER,
                updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(new_status)
        .bind(pipeline_id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE pipelines SET status = $1, updated_at = NOW() WHERE id = $2"
        )
        .bind(new_status)
        .bind(pipeline_id)
        .execute(pool)
        .await?;
    }
    
    info!("Updated pipeline {} status to {}", pipeline_id, new_status);
    Ok(())
}

/// Set job timeout in Redis when job starts running
/// Redis will expire the key after timeout seconds, triggering timeout check
async fn set_job_timeout_in_redis(
    redis: &deadpool_redis::Pool,
    job_id: i64,
    timeout_seconds: i32,
) -> AppResult<()> {
    use deadpool_redis::redis::AsyncCommands;
    
    let mut conn = redis.get().await.map_err(|e| {
        AppError::InternalError(format!("Redis connection failed: {}", e))
    })?;
    
    // Set key with expiration (EX = seconds)
    // Key format: job:timeout:{job_id}
    // Value: job_id (for easy parsing in expiration listener)
    let key = format!("job:timeout:{}", job_id);
    let _: () = conn.set_ex(&key, job_id, timeout_seconds as u64).await.map_err(|e| {
        AppError::InternalError(format!("Failed to set Redis timeout key: {}", e))
    })?;
    
    info!("Set Redis timeout for job {} ({} seconds)", job_id, timeout_seconds);
    Ok(())
}

/// Remove job timeout key from Redis when job completes
async fn clear_job_timeout_in_redis(
    redis: &deadpool_redis::Pool,
    job_id: i64,
) -> AppResult<()> {
    use deadpool_redis::redis::AsyncCommands;
    
    let mut conn = redis.get().await.map_err(|e| {
        AppError::InternalError(format!("Redis connection failed: {}", e))
    })?;
    
    let key = format!("job:timeout:{}", job_id);
    let _: () = conn.del(&key).await.map_err(|e| {
        AppError::InternalError(format!("Failed to delete Redis timeout key: {}", e))
    })?;
    
    Ok(())
}

/// Get timeout for a job (priority: job config > runner max > system default)
async fn get_job_timeout(
    pool: &PgPool,
    redis: &deadpool_redis::Pool,
    job_id: i64,
) -> AppResult<i32> {
    use crate::services::SystemConfigService;
    
    // Get default from config
    let default_timeout = SystemConfigService::get(pool, redis, "ci_default_job_timeout")
        .await
        .and_then(|v| v.as_i64().ok_or_else(|| AppError::BadRequest("Invalid config".to_string())))
        .unwrap_or(3600) as i32;
    
    // Query job config and runner max_timeout
    let row = sqlx::query(
        r#"
        SELECT 
            j.config->>'timeout' as job_timeout,
            r.maximum_timeout
        FROM jobs j
        LEFT JOIN runners r ON j.runner_id = r.id
        WHERE j.id = $1
        "#
    )
    .bind(job_id)
    .fetch_one(pool)
    .await?;
    
    let job_timeout: Option<String> = row.try_get("job_timeout").ok().flatten();
    let runner_max_timeout: Option<i32> = row.try_get("maximum_timeout").ok().flatten();
    
    // Priority: job config > runner > default
    let timeout = job_timeout
        .and_then(|s| s.parse::<i32>().ok())
        .or(runner_max_timeout)
        .unwrap_or(default_timeout);
    
    Ok(timeout)
}

async fn append_job_log(pool: &PgPool, job_id: i64, output: &str) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO job_logs (job_id, output) VALUES ($1, $2)"
    )
    .bind(job_id)
    .bind(output)
    .execute(pool)
    .await?;
    Ok(())
}

async fn update_runner_contact(pool: &PgPool, runner_id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE runners SET last_contact = NOW() WHERE id = $1"
    )
    .bind(runner_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn get_pending_job(pool: &PgPool, runner_id: i64) -> AppResult<Option<Job>> {
    // Get runner info (tags and run_untagged)
    let runner_info = sqlx::query_as::<_, (Vec<String>, bool)>(
        "SELECT tags, run_untagged FROM runners WHERE id = $1"
    )
    .bind(runner_id)
    .fetch_one(pool)
    .await?;
    
    let (runner_tags, run_untagged) = runner_info;

    // Find pending job that matches runner tags
    // Match logic:
    // 1. Job has specific tags -> runner must have ALL those tags
    // 2. Job has no tags && runner.run_untagged = true
    // 3. Job has no tags && runner has no tags
    let job_record = sqlx::query(
        r#"
        UPDATE jobs
        SET status = 'running', runner_id = $1, started_at = NOW()
        WHERE id = (
            SELECT j.id FROM jobs j
            WHERE j.status = 'pending'
            AND (
                -- Job has tags: runner must have all of them
                (j.config->'tags' IS NOT NULL 
                 AND j.config->'tags' != 'null'::jsonb
                 AND $2::TEXT[] @> ARRAY(
                   SELECT jsonb_array_elements_text(j.config->'tags')
                 ))
                OR
                -- Job has no tags: runner can run untagged jobs
                ((j.config->'tags' IS NULL OR j.config->'tags' = 'null'::jsonb OR jsonb_array_length(COALESCE(j.config->'tags', '[]'::jsonb)) = 0)
                 AND ($3 = true OR $4 = 0))
            )
            ORDER BY j.created_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, pipeline_id, project_id, name, stage, config, allow_failure, when_condition
        "#
    )
    .bind(runner_id)
    .bind(&runner_tags)
    .bind(run_untagged)
    .bind(runner_tags.len() as i32)
    .fetch_optional(pool)
    .await?
    .map(|row| -> AppResult<(i64, i64, i64, String, String, serde_json::Value, Option<bool>, Option<String>)> {
        Ok((
            row.try_get("id")?,
            row.try_get("pipeline_id")?,
            row.try_get("project_id")?,
            row.try_get("name")?,
            row.try_get("stage")?,
            row.try_get("config")?,
            row.try_get("allow_failure")?,
            row.try_get("when_condition")?,
        ))
    })
    .transpose()?;

    if let Some((job_id, pipeline_id, project_id, name, stage, config_json, allow_failure, when_condition)) = job_record {
        let config: JobConfig = serde_json::from_value(config_json)?;

        // Get project info for repository URL
        let (project_name, namespace_path): (String, String) = sqlx::query(
            "SELECT p.name, n.path as namespace_path FROM projects p JOIN namespaces n ON p.namespace_id = n.id WHERE p.id = $1"
        )
        .bind(project_id)
        .fetch_one(pool)
        .await
        .and_then(|row| -> Result<(String, String), sqlx::Error> {
            Ok((row.try_get("name")?, row.try_get("namespace_path")?))
        })?;

        // Get pipeline info
        let (ref_name, commit_sha): (String, String) = sqlx::query(
            "SELECT ref_name, commit_sha FROM pipelines WHERE id = $1"
        )
        .bind(pipeline_id)
        .fetch_one(pool)
        .await
        .and_then(|row| -> Result<(String, String), sqlx::Error> {
            Ok((row.try_get("ref_name")?, row.try_get("commit_sha")?))
        })?;

        let job = Job {
            id: job_id,
            pipeline_id,
            project_id,
            name,
            stage,
            script: config.script,
            variables: config.variables.unwrap_or_default(),
            artifacts: config.artifacts,
            cache: config.cache,
            retry: config.retry,
            timeout: config.timeout,
            allow_failure: allow_failure.unwrap_or(false),
            when: WhenCondition::from_str(
                &when_condition.unwrap_or_else(|| "on_success".to_string()),
            ),
            repository_url: format!(
                "http://localhost:8081/{}/{}.git",
                namespace_path, project_name
            ),
            ref_name,
            commit_sha,
            before_sha: String::new(), // TODO: Get from commit parent
        };

        Ok(Some(job))
    } else {
        Ok(None)
    }
}

// Runner Manager - tracks active runners
pub struct RunnerManager {
    runners: HashMap<i64, Addr<RunnerWebSocket>>,
}

impl RunnerManager {
    pub fn new() -> Self {
        Self {
            runners: HashMap::new(),
        }
    }

    pub fn add_runner(&mut self, runner_id: i64, addr: Addr<RunnerWebSocket>) {
        self.runners.insert(runner_id, addr);
        info!("Runner {} added to manager (total: {})", runner_id, self.runners.len());
    }

    pub fn remove_runner(&mut self, runner_id: i64) {
        self.runners.remove(&runner_id);
        info!("Runner {} removed from manager (total: {})", runner_id, self.runners.len());
    }

    pub fn get_runner(&self, runner_id: i64) -> Option<&Addr<RunnerWebSocket>> {
        self.runners.get(&runner_id)
    }

    pub fn active_count(&self) -> usize {
        self.runners.len()
    }
}

// Message types (shared between server and runner)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunnerMessage {
    #[serde(rename = "register")]
    Register {
        token: String,
        name: String,
        tags: Vec<String>,
        executor: String,
    },
    #[serde(rename = "job_update")]
    JobUpdate {
        job_id: i64,
        status: String,
        exit_code: Option<i32>,
        error_message: Option<String>,
    },
    #[serde(rename = "job_log")]
    JobLog { job_id: i64, output: String },
    #[serde(rename = "heartbeat")]
    Heartbeat,
    #[serde(rename = "request_job")]
    RequestJob,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "registered")]
    Registered { runner_id: i64 },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "job_assigned")]
    JobAssigned { job: Job },
    #[serde(rename = "no_jobs")]
    NoJobs,
    #[serde(rename = "ack")]
    Ack,
    #[serde(rename = "config_update")]
    ConfigUpdate {
        tags: Option<Vec<String>>,
        description: Option<String>,
        run_untagged: Option<bool>,
        maximum_timeout: Option<i32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub pipeline_id: i64,
    pub project_id: i64,
    pub name: String,
    pub stage: String,
    pub script: Vec<String>,
    pub variables: HashMap<String, String>,
    pub artifacts: Option<ArtifactsConfig>,
    pub cache: Option<CacheConfig>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<i32>,
    pub allow_failure: bool,
    pub when: WhenCondition,
    pub repository_url: String,
    pub ref_name: String,
    pub commit_sha: String,
    pub before_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub script: Vec<String>,
    pub variables: Option<HashMap<String, String>>,
    pub artifacts: Option<ArtifactsConfig>,
    pub cache: Option<CacheConfig>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<i32>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactsConfig {
    pub paths: Vec<String>,
    pub expire_in: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub paths: Vec<String>,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max: i32,
    pub when: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WhenCondition {
    OnSuccess,
    OnFailure,
    Always,
    Manual,
}

impl WhenCondition {
    fn from_str(s: &str) -> Self {
        match s {
            "on_failure" => WhenCondition::OnFailure,
            "always" => WhenCondition::Always,
            "manual" => WhenCondition::Manual,
            _ => WhenCondition::OnSuccess,
        }
    }
}

// HTTP handler for WebSocket upgrade
pub async fn runner_connect(
    req: HttpRequest,
    stream: web::Payload,
    pool: web::Data<PgPool>,
    redis: web::Data<deadpool_redis::Pool>,
    config: web::Data<Config>,
    runner_manager: web::Data<Arc<RwLock<RunnerManager>>>,
) -> AppResult<HttpResponse> {
    let ws = RunnerWebSocket::new(pool, redis, config, runner_manager.get_ref().clone());
    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}

// ==================== Runner 注册 API ====================

#[derive(Debug, Deserialize)]
pub struct RegisterRunnerRequest {
    pub token: String,  // 注册 token (glrt-xxx)
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub executor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterRunnerResponse {
    pub runner_id: i64,
    pub auth_token: String,  // 用于 WebSocket 认证
    pub websocket_url: String,
}

/// POST /api/v1/runner/register - Runner 注册端点
pub async fn runner_register(
    pool: web::Data<PgPool>,
    req: web::Json<RegisterRunnerRequest>,
) -> AppResult<HttpResponse> {
    // 验证注册 token 是否存在且有效
    let runner_opt = sqlx::query_as::<_, RunnerInfo>(
        r#"
        SELECT 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        FROM runners
        WHERE token = $1 AND is_active = true
        "#
    )
    .bind(&req.token)
    .fetch_optional(pool.get_ref())
    .await?;

    let runner_id: i64;
    let auth_token: String;

    if let Some(existing_runner) = runner_opt {
        // Runner 已存在，更新信息并生成新的认证 token
        runner_id = existing_runner.id;
        auth_token = format!("glrt-auth-{}", Uuid::new_v4().simple());
        
        sqlx::query(
            r#"
            UPDATE runners
            SET name = $1,
                description = $2,
                tags = $3,
                executor = $4,
                token = $5,
                status = 'offline',
                updated_at = NOW()
            WHERE id = $6
            "#
        )
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.tags.clone().unwrap_or_default())
        .bind(&req.executor.clone().unwrap_or_else(|| "shell".to_string()))
        .bind(&auth_token)
        .bind(runner_id)
        .execute(pool.get_ref())
        .await?;

        info!("Runner {} re-registered with new auth token", runner_id);
    } else {
        return Err(crate::error::AppError::Unauthorized(
            "Invalid registration token".to_string()
        ));
    }

    // 获取服务器配置以生成 WebSocket URL
    let websocket_url = format!("ws://localhost:8081/api/v1/runner/connect");

    Ok(HttpResponse::Ok().json(RegisterRunnerResponse {
        runner_id,
        auth_token,
        websocket_url,
    }))
}

// HTTP API 数据结构和处理函数（在 runner.rs 末尾添加）

use chrono::{DateTime, Utc};

// ==================== 数据结构 ====================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RunnerInfo {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub scope: String,  // system, user, namespace, project
    pub user_id: Option<i64>,
    pub namespace_id: Option<i64>,
    pub project_id: Option<i64>,
    pub tags: Vec<String>,
    pub executor: String,
    pub status: String,  // online, offline, paused
    pub is_active: bool,
    pub run_untagged: bool,
    pub locked: bool,
    pub maximum_timeout: Option<i32>,
    pub last_contact: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRunnerRequest {
    pub name: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub executor: Option<String>,  // shell, docker
    pub run_untagged: Option<bool>,
    pub locked: Option<bool>,
    pub maximum_timeout: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CreateRunnerResponse {
    pub runner: RunnerInfo,
    pub token: String,  // 只在创建时返回一次
}

#[derive(Debug, Deserialize)]
pub struct UpdateRunnerRequest {
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub run_untagged: Option<bool>,
    pub locked: Option<bool>,
    pub maximum_timeout: Option<i32>,
}

// ==================== 管理员 API ====================

/// GET /api/v1/admin/runners - 获取所有系统级 runners
pub async fn admin_list_runners(
    pool: web::Data<PgPool>,
    _admin: AdminUser,
) -> AppResult<HttpResponse> {
    let runners = sqlx::query_as::<_, RunnerInfo>(
        r#"
        SELECT 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        FROM runners
        WHERE scope = 'system'
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(runners))
}

/// POST /api/v1/admin/runners - 创建系统级 runner
pub async fn admin_create_runner(
    pool: web::Data<PgPool>,
    _admin: AdminUser,
    req: web::Json<CreateRunnerRequest>,
) -> AppResult<HttpResponse> {
    let token = format!("glrt-{}", Uuid::new_v4().simple());
    let tags = req.tags.clone().unwrap_or_default();
    let executor = req.executor.clone().unwrap_or_else(|| "shell".to_string());
    
    let runner = sqlx::query_as::<_, RunnerInfo>(
        r#"
        INSERT INTO runners (
            name, token, description, scope, tags, executor,
            run_untagged, locked, maximum_timeout, status
        )
        VALUES ($1, $2, $3, 'system', $4, $5, $6, $7, $8, 'offline')
        RETURNING 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        "#
    )
    .bind(&req.name)
    .bind(&token)
    .bind(&req.description)
    .bind(&tags)
    .bind(&executor)
    .bind(req.run_untagged.unwrap_or(true))
    .bind(req.locked.unwrap_or(false))
    .bind(req.maximum_timeout)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(CreateRunnerResponse {
        runner,
        token,
    }))
}

/// PUT /api/v1/admin/runners/:id - 更新系统级 runner
pub async fn admin_update_runner(
    pool: web::Data<PgPool>,
    _admin: AdminUser,
    runner_id: web::Path<i64>,
    req: web::Json<UpdateRunnerRequest>,
    runner_manager: web::Data<Arc<RwLock<RunnerManager>>>,
) -> AppResult<HttpResponse> {
    let runner = sqlx::query_as::<_, RunnerInfo>(
        r#"
        UPDATE runners
        SET 
            description = COALESCE($1, description),
            tags = COALESCE($2, tags),
            is_active = COALESCE($3, is_active),
            run_untagged = COALESCE($4, run_untagged),
            locked = COALESCE($5, locked),
            maximum_timeout = COALESCE($6, maximum_timeout),
            updated_at = NOW()
        WHERE id = $7 AND scope = 'system'
        RETURNING 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        "#
    )
    .bind(&req.description)
    .bind(req.tags.as_ref().map(|t| t.as_slice()))
    .bind(req.is_active)
    .bind(req.run_untagged)
    .bind(req.locked)
    .bind(req.maximum_timeout)
    .bind(*runner_id)
    .fetch_optional(pool.get_ref())
    .await?;

    match runner {
        Some(runner_info) => {
            // 如果 runner 在线，发送配置更新消息
            send_config_update(
                &runner_manager,
                *runner_id,
                req.tags.clone(),
                req.description.clone(),
                req.run_untagged,
                req.maximum_timeout,
            ).await;
            
            Ok(HttpResponse::Ok().json(runner_info))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Runner not found or not a system runner"
        }))),
    }
}

/// DELETE /api/v1/admin/runners/:id - 删除系统级 runner
pub async fn admin_delete_runner(
    pool: web::Data<PgPool>,
    _admin: AdminUser,
    runner_id: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let result = sqlx::query(
        "DELETE FROM runners WHERE id = $1 AND scope = 'system'"
    )
    .bind(*runner_id)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() > 0 {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Runner not found or not a system runner"
        })))
    }
}

// ==================== 用户级 Runner API ====================

/// GET /api/v1/user/runners - 获取当前用户的 runners
pub async fn user_list_runners(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let runners = sqlx::query_as::<_, RunnerInfo>(
        r#"
        SELECT 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        FROM runners
        WHERE scope = 'user' AND user_id = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(runners))
}

/// POST /api/v1/user/runners - 创建用户级 runner
pub async fn user_create_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    req: web::Json<CreateRunnerRequest>,
) -> AppResult<HttpResponse> {
    let token = format!("glrt-{}", Uuid::new_v4().simple());
    let tags = req.tags.clone().unwrap_or_default();
    let executor = req.executor.clone().unwrap_or_else(|| "shell".to_string());
    
    let runner = sqlx::query_as::<_, RunnerInfo>(
        r#"
        INSERT INTO runners (
            name, token, description, scope, user_id, tags, executor,
            run_untagged, locked, maximum_timeout, status
        )
        VALUES ($1, $2, $3, 'user', $4, $5, $6, $7, $8, $9, 'offline')
        RETURNING 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        "#
    )
    .bind(&req.name)
    .bind(&token)
    .bind(&req.description)
    .bind(auth.user_id)
    .bind(&tags)
    .bind(&executor)
    .bind(req.run_untagged.unwrap_or(true))
    .bind(req.locked.unwrap_or(false))
    .bind(req.maximum_timeout)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(CreateRunnerResponse {
        runner,
        token,
    }))
}

/// PUT /api/v1/user/runners/:id - 更新用户级 runner
pub async fn user_update_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    runner_id: web::Path<i64>,
    req: web::Json<UpdateRunnerRequest>,
    runner_manager: web::Data<Arc<RwLock<RunnerManager>>>,
) -> AppResult<HttpResponse> {
    let runner = sqlx::query_as::<_, RunnerInfo>(
        r#"
        UPDATE runners
        SET 
            description = COALESCE($1, description),
            tags = COALESCE($2, tags),
            is_active = COALESCE($3, is_active),
            run_untagged = COALESCE($4, run_untagged),
            locked = COALESCE($5, locked),
            maximum_timeout = COALESCE($6, maximum_timeout),
            updated_at = NOW()
        WHERE id = $7 AND scope = 'user' AND user_id = $8
        RETURNING 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        "#
    )
    .bind(&req.description)
    .bind(req.tags.as_ref().map(|t| t.as_slice()))
    .bind(req.is_active)
    .bind(req.run_untagged)
    .bind(req.locked)
    .bind(req.maximum_timeout)
    .bind(*runner_id)
    .bind(auth.user_id)
    .fetch_optional(pool.get_ref())
    .await?;

    match runner {
        Some(runner_info) => {
            // 如果 runner 在线，发送配置更新消息
            send_config_update(
                &runner_manager,
                *runner_id,
                req.tags.clone(),
                req.description.clone(),
                req.run_untagged,
                req.maximum_timeout,
            ).await;
            
            Ok(HttpResponse::Ok().json(runner_info))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Runner not found or unauthorized"
        }))),
    }
}

/// DELETE /api/v1/user/runners/:id - 删除用户级 runner
pub async fn user_delete_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    runner_id: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let result = sqlx::query(
        "DELETE FROM runners WHERE id = $1 AND scope = 'user' AND user_id = $2"
    )
    .bind(*runner_id)
    .bind(auth.user_id)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() > 0 {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Runner not found or unauthorized"
        })))
    }
}

// 组群和项目级的 API 类似，由于内容较长，放在下一部分文件中...
// 组群级和项目级 Runner API（追加到 runner.rs）

// ==================== 组群级 Runner API ====================

/// GET /api/v1/namespaces/:namespace/runners - 获取组群的 runners
pub async fn namespace_list_runners(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    namespace_path: web::Path<String>,
) -> AppResult<HttpResponse> {
    // 检查用户对组群的访问权限
    let namespace = sqlx::query(
        "SELECT id FROM namespaces WHERE path = $1"
    )
    .bind(namespace_path.as_str())
    .fetch_optional(pool.get_ref())
    .await?;

    let namespace_id = match namespace {
        Some(row) => row.try_get::<i64, _>("id")?,
        None => return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Namespace not found"
        }))),
    };

    // TODO: 检查用户是否是组群成员（需要实现组群成员表）
    // 现在暂时允许所有认证用户查看

    let runners = sqlx::query_as::<_, RunnerInfo>(
        r#"
        SELECT 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        FROM runners
        WHERE scope = 'namespace' AND namespace_id = $1
        ORDER BY created_at DESC
        "#
    )
    .bind(namespace_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(runners))
}

/// POST /api/v1/namespaces/:namespace/runners - 创建组群级 runner
pub async fn namespace_create_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    namespace_path: web::Path<String>,
    req: web::Json<CreateRunnerRequest>,
) -> AppResult<HttpResponse> {
    let namespace_result = sqlx::query(
        "SELECT id, owner_id FROM namespaces WHERE path = $1"
    )
    .bind(namespace_path.as_str())
    .fetch_optional(pool.get_ref())
    .await?;
    
    let (namespace_id, owner_id) = match namespace_result {
        Some(row) => (row.try_get::<i64, _>("id")?, row.try_get::<i64, _>("owner_id")?),
        None => return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Namespace not found"
        }))),
    };

    // 检查权限：只有owner可以创建组群runner
    if owner_id != auth.user_id {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Only namespace owner can create runners"
        })));
    }

    let token = format!("glrt-{}", Uuid::new_v4().simple());
    let tags = req.tags.clone().unwrap_or_default();
    let executor = req.executor.clone().unwrap_or_else(|| "shell".to_string());
    
    let runner = sqlx::query_as::<_, RunnerInfo>(
        r#"
        INSERT INTO runners (
            name, token, description, scope, namespace_id, tags, executor,
            run_untagged, locked, maximum_timeout, status
        )
        VALUES ($1, $2, $3, 'namespace', $4, $5, $6, $7, $8, $9, 'offline')
        RETURNING 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        "#
    )
    .bind(&req.name)
    .bind(&token)
    .bind(&req.description)
    .bind(namespace_id)
    .bind(&tags)
    .bind(&executor)
    .bind(req.run_untagged.unwrap_or(true))
    .bind(req.locked.unwrap_or(false))
    .bind(req.maximum_timeout)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(CreateRunnerResponse {
        runner,
        token,
    }))
}

/// PUT /api/v1/namespaces/:namespace/runners/:id - 更新组群级 runner
pub async fn namespace_update_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, i64)>,
    req: web::Json<UpdateRunnerRequest>,
    runner_manager: web::Data<Arc<RwLock<RunnerManager>>>,
) -> AppResult<HttpResponse> {
    let (namespace_path, runner_id) = path.into_inner();
    
    let namespace_result = sqlx::query(
        "SELECT id, owner_id FROM namespaces WHERE path = $1"
    )
    .bind(&namespace_path)
    .fetch_optional(pool.get_ref())
    .await?;

    let (namespace_id, owner_id) = match namespace_result {
        Some(row) => (row.try_get::<i64, _>("id")?, row.try_get::<i64, _>("owner_id")?),
        None => return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Namespace not found"
        }))),
    };

    if owner_id != auth.user_id {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Only namespace owner can update runners"
        })));
    }

    let runner = sqlx::query_as::<_, RunnerInfo>(
        r#"
        UPDATE runners
        SET 
            description = COALESCE($1, description),
            tags = COALESCE($2, tags),
            is_active = COALESCE($3, is_active),
            run_untagged = COALESCE($4, run_untagged),
            locked = COALESCE($5, locked),
            maximum_timeout = COALESCE($6, maximum_timeout),
            updated_at = NOW()
        WHERE id = $7 AND scope = 'namespace' AND namespace_id = $8
        RETURNING 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        "#
    )
    .bind(&req.description)
    .bind(req.tags.as_ref().map(|t| t.as_slice()))
    .bind(req.is_active)
    .bind(req.run_untagged)
    .bind(req.locked)
    .bind(req.maximum_timeout)
    .bind(runner_id)
    .bind(namespace_id)
    .fetch_optional(pool.get_ref())
    .await?;

    match runner {
        Some(runner_info) => {
            // 如果 runner 在线，发送配置更新消息
            send_config_update(
                &runner_manager,
                runner_id,
                req.tags.clone(),
                req.description.clone(),
                req.run_untagged,
                req.maximum_timeout,
            ).await;
            
            Ok(HttpResponse::Ok().json(runner_info))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Runner not found or unauthorized"
        }))),
    }
}

/// DELETE /api/v1/namespaces/:namespace/runners/:id - 删除组群级 runner
pub async fn namespace_delete_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace_path, runner_id) = path.into_inner();
    
    let namespace_result = sqlx::query(
        "SELECT id, owner_id FROM namespaces WHERE path = $1"
    )
    .bind(&namespace_path)
    .fetch_optional(pool.get_ref())
    .await?;

    let (namespace_id, owner_id) = match namespace_result {
        Some(row) => (row.try_get::<i64, _>("id")?, row.try_get::<i64, _>("owner_id")?),
        None => return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Namespace not found"
        }))),
    };

    if owner_id != auth.user_id {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Only namespace owner can delete runners"
        })));
    }

    let result = sqlx::query(
        "DELETE FROM runners WHERE id = $1 AND scope = 'namespace' AND namespace_id = $2"
    )
    .bind(runner_id)
    .bind(namespace_id)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() > 0 {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Runner not found or unauthorized"
        })))
    }
}

// ==================== 项目级 Runner API ====================

/// GET /api/v1/projects/:namespace/:project/runners - 获取项目可用的 runners
/// 包括：项目专属 + 组群级 + 用户级（如果是owner）+ 系统级
pub async fn project_list_runners(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (namespace_path, project_name) = path.into_inner();
    
    let project_result = sqlx::query(
        r#"
        SELECT p.id, p.namespace_id, n.owner_id as namespace_owner_id
        FROM projects p
        JOIN namespaces n ON p.namespace_id = n.id
        WHERE n.path = $1 AND p.name = $2
        "#
    )
    .bind(&namespace_path)
    .bind(&project_name)
    .fetch_optional(pool.get_ref())
    .await?;

    let (project_id, namespace_id, namespace_owner_id) = match project_result {
        Some(row) => (row.try_get::<i64, _>("id")?, row.try_get::<i64, _>("namespace_id")?, row.try_get::<i64, _>("namespace_owner_id")?),
        None => return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Project not found"
        }))),
    };

    // 获取项目专属、组群级和系统级 runners
    let mut runners = sqlx::query_as::<_, RunnerInfo>(
        r#"
        SELECT 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        FROM runners
        WHERE 
            (scope = 'project' AND project_id = $1) OR
            (scope = 'namespace' AND namespace_id = $2 AND is_active = true) OR
            (scope = 'system' AND is_active = true)
        ORDER BY 
            CASE scope
                WHEN 'project' THEN 1
                WHEN 'namespace' THEN 2
                WHEN 'system' THEN 3
            END,
            created_at DESC
        "#
    )
    .bind(project_id)
    .bind(namespace_id)
    .fetch_all(pool.get_ref())
    .await?;

    // 如果用户是namespace owner，还包括用户级 runners
    if namespace_owner_id == auth.user_id {
        let user_runners = sqlx::query_as::<_, RunnerInfo>(
            r#"
            SELECT 
                id, name, description, scope, user_id, namespace_id, project_id,
                tags, executor, status, is_active, run_untagged, locked,
                maximum_timeout, last_contact, created_at
            FROM runners
            WHERE scope = 'user' AND user_id = $1 AND is_active = true
            ORDER BY created_at DESC
            "#
        )
        .bind(auth.user_id)
        .fetch_all(pool.get_ref())
        .await?;
        runners.extend(user_runners);
    }

    Ok(HttpResponse::Ok().json(runners))
}

/// POST /api/v1/projects/:namespace/:project/runners - 创建项目级 runner
pub async fn project_create_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
    req: web::Json<CreateRunnerRequest>,
) -> AppResult<HttpResponse> {
    let (namespace_path, project_name) = path.into_inner();
    
    let project_id_opt = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT p.id
        FROM projects p
        JOIN namespaces n ON p.namespace_id = n.id
        JOIN project_members pm ON p.id = pm.project_id
        WHERE n.path = $1 AND p.name = $2 AND pm.user_id = $3 AND pm.role IN ('owner', 'maintainer')
        "#
    )
    .bind(&namespace_path)
    .bind(&project_name)
    .bind(auth.user_id)
    .fetch_optional(pool.get_ref())
    .await?;
    
    let project_id = match project_id_opt {
        Some(id) => id,
        None => return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Project not found or insufficient permissions"
        }))),
    };

    let token = format!("glrt-{}", Uuid::new_v4().simple());
    let tags = req.tags.clone().unwrap_or_default();
    let executor = req.executor.clone().unwrap_or_else(|| "shell".to_string());
    
    let runner = sqlx::query_as::<_, RunnerInfo>(
        r#"
        INSERT INTO runners (
            name, token, description, scope, project_id, tags, executor,
            run_untagged, locked, maximum_timeout, status
        )
        VALUES ($1, $2, $3, 'project', $4, $5, $6, $7, $8, $9, 'offline')
        RETURNING 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        "#
    )
    .bind(&req.name)
    .bind(&token)
    .bind(&req.description)
    .bind(project_id)
    .bind(&tags)
    .bind(&executor)
    .bind(req.run_untagged.unwrap_or(true))
    .bind(req.locked.unwrap_or(true))  // 项目级 runner 默认锁定
    .bind(req.maximum_timeout)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(CreateRunnerResponse {
        runner,
        token,
    }))
}

/// PUT /api/v1/projects/:namespace/:project/runners/:id - 更新项目级 runner
pub async fn project_update_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
    req: web::Json<UpdateRunnerRequest>,
    runner_manager: web::Data<Arc<RwLock<RunnerManager>>>,
) -> AppResult<HttpResponse> {
    let (namespace_path, project_name, runner_id) = path.into_inner();
    
    let project_id_opt = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT p.id
        FROM projects p
        JOIN namespaces n ON p.namespace_id = n.id
        JOIN project_members pm ON p.id = pm.project_id
        WHERE n.path = $1 AND p.name = $2 AND pm.user_id = $3 AND pm.role IN ('owner', 'maintainer')
        "#
    )
    .bind(&namespace_path)
    .bind(&project_name)
    .bind(auth.user_id)
    .fetch_optional(pool.get_ref())
    .await?;

    let project_id = match project_id_opt {
        Some(id) => id,
        None => return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Project not found or insufficient permissions"
        }))),
    };

    let runner = sqlx::query_as::<_, RunnerInfo>(
        r#"
        UPDATE runners
        SET 
            description = COALESCE($1, description),
            tags = COALESCE($2, tags),
            is_active = COALESCE($3, is_active),
            run_untagged = COALESCE($4, run_untagged),
            locked = COALESCE($5, locked),
            maximum_timeout = COALESCE($6, maximum_timeout),
            updated_at = NOW()
        WHERE id = $7 AND scope = 'project' AND project_id = $8
        RETURNING 
            id, name, description, scope, user_id, namespace_id, project_id,
            tags, executor, status, is_active, run_untagged, locked,
            maximum_timeout, last_contact, created_at
        "#
    )
    .bind(&req.description)
    .bind(req.tags.as_ref().map(|t| t.as_slice()))
    .bind(req.is_active)
    .bind(req.run_untagged)
    .bind(req.locked)
    .bind(req.maximum_timeout)
    .bind(runner_id)
    .bind(project_id)
    .fetch_optional(pool.get_ref())
    .await?;

    match runner {
        Some(runner_info) => {
            // 如果 runner 在线，发送配置更新消息
            send_config_update(
                &runner_manager,
                runner_id,
                req.tags.clone(),
                req.description.clone(),
                req.run_untagged,
                req.maximum_timeout,
            ).await;
            
            Ok(HttpResponse::Ok().json(runner_info))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Runner not found or unauthorized"
        }))),
    }
}

/// 发送配置更新通知给在线的 runner
async fn send_config_update(
    runner_manager: &Arc<RwLock<RunnerManager>>,
    runner_id: i64,
    tags: Option<Vec<String>>,
    description: Option<String>,
    run_untagged: Option<bool>,
    maximum_timeout: Option<i32>,
) {
    let manager = runner_manager.read().await;
    if let Some(addr) = manager.get_runner(runner_id) {
        let update_msg = ServerMessage::ConfigUpdate {
            tags,
            description,
            run_untagged,
            maximum_timeout,
        };
        
        if let Ok(json) = serde_json::to_string(&update_msg) {
            addr.do_send(SendText(json));
            info!("Sent config update to runner {}", runner_id);
        }
    }
}

/// DELETE /api/v1/projects/:namespace/:project/runners/:id - 删除项目级 runner
pub async fn project_delete_runner(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace_path, project_name, runner_id) = path.into_inner();
    
    let project_id_opt = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT p.id
        FROM projects p
        JOIN namespaces n ON p.namespace_id = n.id
        JOIN project_members pm ON p.id = pm.project_id
        WHERE n.path = $1 AND p.name = $2 AND pm.user_id = $3 AND pm.role IN ('owner', 'maintainer')
        "#
    )
    .bind(&namespace_path)
    .bind(&project_name)
    .bind(auth.user_id)
    .fetch_optional(pool.get_ref())
    .await?;

    let project_id = match project_id_opt {
        Some(id) => id,
        None => return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Project not found or insufficient permissions"
        }))),
    };

    let result = sqlx::query(
        "DELETE FROM runners WHERE id = $1 AND scope = 'project' AND project_id = $2"
    )
    .bind(runner_id)
    .bind(project_id)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() > 0 {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Runner not found or unauthorized"
        })))
    }
}

/// Start Redis keyspace notification listener for job timeouts
/// When Redis key expires, it triggers timeout handling
pub async fn start_redis_timeout_listener(pool: PgPool, redis_url: String, instance_id: String) {
    info!("Redis job timeout listener starting (instance: {})...", instance_id);
    
    loop {
        match listen_for_expired_keys(&pool, &redis_url, &instance_id).await {
            Ok(_) => {
                warn!("Redis listener disconnected, reconnecting...");
            }
            Err(e) => {
                error!("Redis listener error: {}, retrying in 5s...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// Subscribe to Redis keyspace notifications for expired keys
async fn listen_for_expired_keys(pool: &PgPool, redis_url: &str, instance_id: &str) -> AppResult<()> {
    use deadpool_redis::redis;
    use futures::StreamExt;
    
    // Create dedicated Redis client for PubSub (cannot use pooled connection)
    let client = redis::Client::open(redis_url).map_err(|e| {
        AppError::InternalError(format!("Failed to create Redis client: {}", e))
    })?;
    
    // First connection: configure Redis
    let mut config_conn = client.get_async_connection().await.map_err(|e| {
        AppError::InternalError(format!("Redis connection failed: {}", e))
    })?;
    
    // Enable keyspace notifications for expired events
    redis::cmd("CONFIG")
        .arg("SET")
        .arg("notify-keyspace-events")
        .arg("Ex")
        .query_async(&mut config_conn)
        .await
        .map_err(|e| {
            AppError::InternalError(format!("Failed to configure Redis notifications: {}", e))
        })?;
    
    info!("Redis keyspace notifications enabled (Ex)");
    
    // Second connection: create PubSub
    let conn = client.get_async_connection().await.map_err(|e| {
        AppError::InternalError(format!("Redis PubSub connection failed: {}", e))
    })?;
    
    let mut pubsub = conn.into_pubsub();
    pubsub.psubscribe("__keyevent@*__:expired").await.map_err(|e| {
        AppError::InternalError(format!("Failed to subscribe to Redis expired events: {}", e))
    })?;
    
    info!("Subscribed to __keyevent@*__:expired");
    
    let mut stream = pubsub.on_message();
    
    while let Some(msg) = stream.next().await {
        let key: String = match msg.get_payload() {
            Ok(k) => k,
            Err(e) => {
                warn!("Failed to parse expired key payload: {}", e);
                continue;
            }
        };
        
        // Check if this is a job timeout key (format: job:timeout:{job_id})
        if key.starts_with("job:timeout:") {
            if let Some(job_id_str) = key.strip_prefix("job:timeout:") {
                if let Ok(job_id) = job_id_str.parse::<i64>() {
                    info!("Job {} timeout key expired, checking responsibility...", job_id);
                    
                    let pool_clone = pool.clone();
                    let instance_id = instance_id.to_string();
                    tokio::spawn(async move {
                        if let Err(e) = handle_job_timeout(pool_clone, job_id, &instance_id).await {
                            error!("Failed to handle timeout for job {}: {}", job_id, e);
                        }
                    });
                } else {
                    warn!("Invalid job_id in expired key: {}", key);
                }
            }
        }
    }
    
    Ok(())
}

/// Handle job timeout: mark job as failed if still running
/// Only process if current instance is responsible or job has no watcher
async fn handle_job_timeout(pool: PgPool, job_id: i64, instance_id: &str) -> AppResult<()> {
    // Check current job status and watcher
    let row = sqlx::query(
        "SELECT status, watcher_instance FROM jobs WHERE id = $1"
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await?;
    
    let job_status: String = row.try_get("status")?;
    let watcher_instance: Option<String> = row.try_get("watcher_instance").ok().flatten();
    
    // If job already completed, no action needed
    if job_status != "running" {
        info!("Job {} already in '{}' state, skipping timeout", job_id, job_status);
        return Ok(());
    }
    
    // Check responsibility: only process if we are the watcher or no one is watching
    match watcher_instance {
        Some(ref watcher) if watcher != instance_id => {
            info!("Job {} is watched by {}, skipping (current instance: {})", job_id, watcher, instance_id);
            return Ok(());
        }
        None => {
            info!("Job {} has no watcher, taking responsibility (instance: {})", job_id, instance_id);
        }
        Some(ref watcher) => {
            info!("Job {} is watched by us ({}), processing timeout", job_id, watcher);
        }
    }
    
    warn!("Job {} timed out, marking as failed", job_id);
    
    // Get start time to calculate actual running duration
    let started_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT started_at FROM jobs WHERE id = $1"
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await?;
    
    let running_seconds = if let Some(started) = started_at {
        (chrono::Utc::now() - started).num_seconds()
    } else {
        0
    };
    
    // Update job status to failed
    sqlx::query(
        r#"
        UPDATE jobs
        SET 
            status = 'failed',
            finished_at = NOW(),
            updated_at = NOW(),
            error_message = 'Job exceeded maximum execution time limit'
        WHERE id = $1
        "#
    )
    .bind(job_id)
    .execute(&pool)
    .await?;
    
    // Log timeout event
    let timeout_log = format!(
        "\n\n=== Job Timeout ===\nJob exceeded maximum execution time limit\nJob ran for {} seconds before being terminated\n",
        running_seconds
    );
    
    sqlx::query(
        "INSERT INTO job_logs (job_id, output, created_at) VALUES ($1, $2, NOW())"
    )
    .bind(job_id)
    .bind(&timeout_log)
    .execute(&pool)
    .await?;
    
    info!("Job {} marked as failed due to timeout", job_id);
    
    Ok(())
}
