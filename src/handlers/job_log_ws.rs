use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use log::{debug, error, info};
use sqlx::PgPool;
use std::time::{Duration, Instant};

use crate::error::{AppError, AppResult};
use crate::models::PipelineJobLog;
use crate::services::ProjectService;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// WebSocket Actor for streaming job logs
pub struct JobLogWebSocket {
    pool: web::Data<PgPool>,
    job_id: i64,
    project_id: i64,
    hb: Instant,
    last_log_id: i64,
}

impl JobLogWebSocket {
    pub fn new(pool: web::Data<PgPool>, job_id: i64, project_id: i64) -> Self {
        Self {
            pool,
            job_id,
            project_id,
            hb: Instant::now(),
            last_log_id: 0,
        }
    }

    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Check for timeout
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                info!("Job log WebSocket timeout, disconnecting");
                ctx.stop();
                return;
            }

            // Fetch new logs
            let pool = act.pool.clone();
            let job_id = act.job_id;
            let last_log_id = act.last_log_id;

            actix_web::rt::spawn(async move {
                match fetch_new_logs(pool.get_ref(), job_id, last_log_id).await {
                    Ok(logs) => {
                        if !logs.is_empty() {
                            for log in logs {
                                // Send log to client
                                // This will be handled in the actor context
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch logs: {}", e);
                    }
                }
            });

            ctx.ping(b"");
        });
    }

    fn check_and_send_logs(&mut self, ctx: &mut ws::WebsocketContext<Self>) {
        let pool = self.pool.clone();
        let job_id = self.job_id;
        let last_log_id = self.last_log_id;

        let addr = ctx.address();
        actix_web::rt::spawn(async move {
            match fetch_new_logs(pool.get_ref(), job_id, last_log_id).await {
                Ok(logs) => {
                    if !logs.is_empty() {
                        let output = logs.iter().map(|l| l.output.as_str()).collect::<Vec<_>>().join("");
                        let last_id = logs.last().map(|l| l.id).unwrap_or(last_log_id);
                        addr.do_send(SendLogs { output, last_id });
                    }
                }
                Err(e) => {
                    error!("Failed to fetch logs: {}", e);
                }
            }
        });
    }
}

impl Actor for JobLogWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Job log WebSocket connection established for job {}", self.job_id);
        self.start_heartbeat(ctx);
        // Send initial logs
        self.check_and_send_logs(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("Job log WebSocket connection closed");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for JobLogWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(_)) => {
                // Client can send "fetch" to request logs
                self.check_and_send_logs(ctx);
            }
            Ok(ws::Message::Close(reason)) => {
                debug!("Client closed connection: {:?}", reason);
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

// Actor message to send logs
#[derive(actix::Message)]
#[rtype(result = "()")]
struct SendLogs {
    output: String,
    last_id: i64,
}

impl actix::Handler<SendLogs> for JobLogWebSocket {
    type Result = ();

    fn handle(&mut self, msg: SendLogs, ctx: &mut Self::Context) {
        ctx.text(msg.output);
        self.last_log_id = msg.last_id;
    }
}

async fn fetch_new_logs(pool: &PgPool, job_id: i64, since_id: i64) -> AppResult<Vec<PipelineJobLog>> {
    let logs = sqlx::query_as::<_, PipelineJobLog>(
        "SELECT * FROM job_logs WHERE job_id = $1 AND id > $2 ORDER BY id"
    )
    .bind(job_id)
    .bind(since_id)
    .fetch_all(pool)
    .await?;
    Ok(logs)
}

/// WebSocket endpoint for job logs
pub async fn job_log_stream(
    req: HttpRequest,
    stream: web::Payload,
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, _pipeline_id, job_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;

    // Verify job belongs to project
    let _ = sqlx::query("SELECT id FROM jobs WHERE id = $1 AND project_id = $2")
        .bind(job_id)
        .bind(project.id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Job not found".to_string()))?;

    let ws = JobLogWebSocket::new(pool, job_id, project.id);
    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
