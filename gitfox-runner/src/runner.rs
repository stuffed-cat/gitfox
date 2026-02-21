use crate::config::RunnerConfig;
use crate::error::{Result, RunnerError};
use crate::executor::Executor;
use crate::messages::{Job, JobStatus, RunnerMessage, ServerMessage};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use std::path::PathBuf;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct Runner {
    config: RunnerConfig,
    config_path: PathBuf,
    runner_id: Option<i64>,
}

impl Runner {
    pub fn new(config: RunnerConfig, config_path: PathBuf) -> Self {
        Self {
            config,
            config_path,
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
        // 使用配置中的完整 WebSocket URL（已包含路径）
        let ws_url = self.config.server_url.replace("http://", "ws://").replace("https://", "wss://");

        info!("Connecting to GitFox server...");
        info!("  WebSocket URL: {}", ws_url);
        info!("  Runner name:   {}", self.config.name);
        info!("  Executor:      {}", self.config.executor);

        let (ws_stream, _) = connect_async(&ws_url).await?;
        info!("WebSocket connection established successfully");

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
                        info!("Runner registered successfully!");
                        info!("  Runner ID:   {}", runner_id);
                        info!("  Runner name: {}", self.config.name);
                        info!("  Tags:        {}", if self.config.tags.is_empty() { "(none)".to_string() } else { self.config.tags.join(", ") });
                        info!("Awaiting jobs...");
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
        &mut self,
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
                info!("────────────────────────────────────────");
                info!("Job received: {} (ID: {})", job.name, job.id);
                info!("  Pipeline ID:  {}", job.pipeline_id);
                info!("  Stage:        {}", job.stage);
                info!("  Ref:          {}", job.ref_name);
                info!("  Commit:       {}", &job.commit_sha[..8.min(job.commit_sha.len())]);
                info!("  Scripts:      {} command(s)", job.script.len());
                info!("  Allow fail:   {}", job.allow_failure);
                if let Some(timeout) = job.timeout {
                    info!("  Timeout:      {}s", timeout);
                }
                info!("────────────────────────────────────────");
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
            ServerMessage::ConfigUpdate { tags, description, run_untagged, maximum_timeout } => {
                info!("Received configuration update from server");
                if let Err(e) = self.update_config(tags, description, run_untagged, maximum_timeout) {
                    error!("Failed to update configuration: {}", e);
                } else {
                    info!("Configuration updated and saved successfully");
                }
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
        self.send_job_update(job.id, JobStatus::Running, None, None, write)
            .await?;

        // Get job timeout (in seconds)
        let timeout_seconds = job.timeout.unwrap_or(3600) as u64;
        let timeout_duration = Duration::from_secs(timeout_seconds);
        
        info!("Job {} will timeout after {} seconds", job.id, timeout_seconds);

        // Create executor
        let executor = Executor::new(self.config.clone());

        // Execute with timeout control
        let job_id = job.id;
        let job_name = job.name.clone();
        
        // Execute job with actual timeout enforcement
        let execution_future = async {
            executor.execute(&job, |log_line| {
                let log_msg = RunnerMessage::JobLog {
                    job_id,
                    output: log_line.to_string(),
                };
                if let Ok(msg) = serde_json::to_string(&log_msg) {
                    // Send in blocking context
                    let _ = futures::executor::block_on(write.send(Message::Text(msg)));
                }
            })
        };

        // Wrap execution with timeout
        let result = match tokio::time::timeout(timeout_duration, execution_future).await {
            Ok(exec_result) => exec_result,
            Err(_) => {
                // Timeout occurred - runner side timeout enforcement
                error!("Job {} '{}' exceeded timeout of {}s (runner-side enforcement)", job_id, job_name, timeout_seconds);
                
                // Send timeout log
                let timeout_log = format!("\n\n=== Runner Timeout ===\nJob exceeded {} seconds execution time limit\nTerminated by runner\n", timeout_seconds);
                let log_msg = RunnerMessage::JobLog {
                    job_id,
                    output: timeout_log,
                };
                if let Ok(msg) = serde_json::to_string(&log_msg) {
                    let _ = write.send(Message::Text(msg)).await;
                }
                
                Err(crate::error::RunnerError::Execution(format!("Job timeout after {} seconds", timeout_seconds)))
            }
        };

        // Update final status
        match result {
            Ok(exit_code) => {
                let status = if exit_code == 0 {
                    JobStatus::Success
                } else {
                    JobStatus::Failed
                };
                info!("────────────────────────────────────────");
                info!("Job {} '{}' completed", job_id, job_name);
                info!("  Status:    {:?}", status);
                info!("  Exit code: {}", exit_code);
                info!("────────────────────────────────────────");
                self.send_job_update(job_id, status, Some(exit_code), None, write)
                    .await?;
            }
            Err(e) => {
                error!("Job {} execution error: {}", job_id, e);
                let error_msg = format!("Execution error: {}", e);
                self.send_job_update(job_id, JobStatus::Failed, Some(-1), Some(error_msg), write)
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
        error_message: Option<String>,
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
            error_message,
        };
        let msg = serde_json::to_string(&update)?;
        write.send(Message::Text(msg)).await?;
        Ok(())
    }

    /// Update runner configuration and save to file
    fn update_config(
        &mut self,
        tags: Option<Vec<String>>,
        description: Option<String>,
        run_untagged: Option<bool>,
        maximum_timeout: Option<i32>,
    ) -> Result<()> {
        let mut updated = false;

        if let Some(new_tags) = tags {
            info!("  Tags: {} -> {}", 
                if self.config.tags.is_empty() { "(none)".to_string() } else { self.config.tags.join(", ") },
                if new_tags.is_empty() { "(none)".to_string() } else { new_tags.join(", ") }
            );
            self.config.tags = new_tags;
            updated = true;
        }

        if let Some(_new_description) = description {
            info!("  Description updated");
            // Note: RunnerConfig doesn't currently have a description field
            // but we'll keep this for future compatibility
            updated = true;
        }

        if let Some(_new_run_untagged) = run_untagged {
            // Note: RunnerConfig doesn't have run_untagged field, it's server-side only
            // Keeping for future compatibility
            updated = true;
        }

        if let Some(new_timeout) = maximum_timeout {
            // Note: This is maximum_timeout on server side, different from script_timeout
            info!("  Maximum timeout: {} seconds", new_timeout);
            updated = true;
        }

        if updated {
            // Save updated configuration to file
            self.config.save(&self.config_path)?;
            info!("Configuration saved to: {}", self.config_path.display());
        }

        Ok(())
    }
}
