use crate::config::RunnerConfig;
use crate::error::{Result, RunnerError};
use crate::executor::Executor;
use crate::messages::{Job, JobStatus, RunnerMessage, ServerMessage};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct Runner {
    config: RunnerConfig,
    runner_id: Option<i64>,
}

impl Runner {
    pub fn new(config: RunnerConfig) -> Self {
        Self {
            config,
            runner_id: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            if let Err(e) = self.connect_and_run().await {
                error!("Runner disconnected: {}", e);
                info!("Reconnecting in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    async fn connect_and_run(&mut self) -> Result<()> {
        // Build WebSocket URL
        let ws_url = format!(
            "{}/api/v1/runner/connect",
            self.config.server_url.replace("http", "ws")
        );

        info!("Connecting to WebSocket: {}", ws_url);

        let (ws_stream, _) = connect_async(&ws_url).await?;
        info!("WebSocket connected");

        let (mut write, mut read) = ws_stream.split();

        // Register runner
        let register_msg = RunnerMessage::Register {
            token: self.config.token.clone(),
            name: self.config.name.clone(),
            tags: self.config.tags.clone(),
            executor: self.config.executor.clone(),
        };

        let msg_json = serde_json::to_string(&register_msg)?;
        write.send(Message::Text(msg_json)).await?;

        // Wait for registration response
        if let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let server_msg: ServerMessage = serde_json::from_str(&text)?;
                match server_msg {
                    ServerMessage::Registered { runner_id } => {
                        info!("Runner registered with ID: {}", runner_id);
                        self.runner_id = Some(runner_id);
                    }
                    ServerMessage::Error { message } => {
                        return Err(RunnerError::Auth(message));
                    }
                    _ => {
                        return Err(RunnerError::Connection(
                            "Unexpected registration response".to_string(),
                        ));
                    }
                }
            }
        }

        // Start heartbeat task
        let mut heartbeat_interval = interval(Duration::from_secs(30));
        let mut job_request_interval = interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                // Receive messages from server
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.handle_server_message(&text, &mut write).await {
                                error!("Error handling message: {}", e);
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("Server closed connection");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                }

                // Send heartbeat
                _ = heartbeat_interval.tick() => {
                    let heartbeat = RunnerMessage::Heartbeat;
                    if let Ok(msg) = serde_json::to_string(&heartbeat) {
                        let _ = write.send(Message::Text(msg)).await;
                    }
                }

                // Request jobs
                _ = job_request_interval.tick() => {
                    let request = RunnerMessage::RequestJob;
                    if let Ok(msg) = serde_json::to_string(&request) {
                        let _ = write.send(Message::Text(msg)).await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_server_message(
        &self,
        text: &str,
        write: &mut futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    ) -> Result<()> {
        let server_msg: ServerMessage = serde_json::from_str(text)?;
        debug!("Received: {:?}", server_msg);

        match server_msg {
            ServerMessage::JobAssigned { job } => {
                info!("Job assigned: {} (ID: {})", job.name, job.id);
                self.execute_job(job, write).await?;
            }
            ServerMessage::NoJobs => {
                debug!("No jobs available");
            }
            ServerMessage::Ack => {
                debug!("Server acknowledged");
            }
            ServerMessage::Error { message } => {
                error!("Server error: {}", message);
            }
            _ => {}
        }

        Ok(())
    }

    async fn execute_job(
        &self,
        job: Job,
        write: &mut futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    ) -> Result<()> {
        // Update job status to running
        self.send_job_update(job.id, JobStatus::Running, None, write)
            .await?;

        // Create executor
        let executor = Executor::new(&self.config.executor);

        // Execute job and stream logs
        let result = executor.execute(&job, |log_line| {
            let log_msg = RunnerMessage::JobLog {
                job_id: job.id,
                output: log_line.to_string(),
            };
            if let Ok(msg) = serde_json::to_string(&log_msg) {
                // Send in blocking context
                let _ = futures::executor::block_on(write.send(Message::Text(msg)));
            }
        });

        // Update final status
        match result {
            Ok(exit_code) => {
                let status = if exit_code == 0 {
                    JobStatus::Success
                } else {
                    JobStatus::Failed
                };
                info!(
                    "Job {} completed with status: {:?} (exit code: {})",
                    job.id, status, exit_code
                );
                self.send_job_update(job.id, status, Some(exit_code), write)
                    .await?;
            }
            Err(e) => {
                error!("Job {} execution error: {}", job.id, e);
                self.send_job_update(job.id, JobStatus::Failed, Some(-1), write)
                    .await?;
            }
        }

        Ok(())
    }

    async fn send_job_update(
        &self,
        job_id: i64,
        status: JobStatus,
        exit_code: Option<i32>,
        write: &mut futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    ) -> Result<()> {
        let update = RunnerMessage::JobUpdate {
            job_id,
            status,
            exit_code,
        };
        let msg = serde_json::to_string(&update)?;
        write.send(Message::Text(msg)).await?;
        Ok(())
    }
}
