//! Smart HTTP service implementation (Git HTTP protocol)

use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info};

use crate::config::Config;
use crate::proto::*;

pub struct SmartHttpServiceImpl {
    config: Arc<Config>,
}

impl SmartHttpServiceImpl {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
    
    fn get_repo_path(&self, repo: Option<&Repository>) -> Result<String, Status> {
        let repo = repo.ok_or_else(|| Status::invalid_argument("Repository required"))?;
        
        if !repo.storage_path.is_empty() {
            Ok(repo.storage_path.clone())
        } else if !repo.relative_path.is_empty() {
            Ok(self.config.repo_path(&repo.relative_path))
        } else {
            Err(Status::invalid_argument("Repository path required"))
        }
    }
}

#[tonic::async_trait]
impl smart_http_service_server::SmartHttpService for SmartHttpServiceImpl {
    async fn info_refs(
        &self,
        request: Request<InfoRefsRequest>,
    ) -> Result<Response<InfoRefsResponse>, Status> {
        let req = request.into_inner();
        let repo_path = self.get_repo_path(req.repository.as_ref())?;
        
        let service = &req.service;
        debug!("Git info/refs for {} at {}", service, repo_path);
        
        // Run git command to get refs
        let output = Command::new(&self.config.git_bin_path)
            .arg(service.strip_prefix("git-").unwrap_or(service))
            .arg("--stateless-rpc")
            .arg("--advertise-refs")
            .arg(&repo_path)
            .output()
            .await
            .map_err(|e| Status::internal(format!("Failed to run git: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Git command failed: {}", stderr);
            return Err(Status::internal(format!("Git command failed: {}", stderr)));
        }
        
        // Build response with Git protocol header
        let mut data = Vec::new();
        let header = format!("# service={}\n", service);
        let pkt_line = format!("{:04x}{}", header.len() + 4, header);
        data.extend_from_slice(pkt_line.as_bytes());
        data.extend_from_slice(b"0000"); // flush-pkt
        data.extend_from_slice(&output.stdout);
        
        Ok(Response::new(InfoRefsResponse { data }))
    }
    
    type UploadPackStream = Pin<Box<dyn Stream<Item = Result<UploadPackResponse, Status>> + Send>>;
    
    async fn upload_pack(
        &self,
        request: Request<Streaming<UploadPackRequest>>,
    ) -> Result<Response<Self::UploadPackStream>, Status> {
        let mut stream = request.into_inner();
        
        // Get first message to extract repository info
        let first = stream.message().await?
            .ok_or_else(|| Status::invalid_argument("No request data"))?;
        
        let repo_path = self.get_repo_path(first.repository.as_ref())?;
        debug!("Git upload-pack for {}", repo_path);
        
        // Spawn git-upload-pack process
        let mut child = Command::new(&self.config.git_bin_path)
            .arg("upload-pack")
            .arg("--stateless-rpc")
            .arg(&repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Status::internal(format!("Failed to spawn git: {}", e)))?;
        
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        
        // Write first chunk
        stdin.write_all(&first.data).await
            .map_err(|e| Status::internal(format!("Failed to write to git: {}", e)))?;
        
        // Spawn task to write remaining input
        let write_task = tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                if let Err(e) = stdin.write_all(&msg.data).await {
                    error!("Failed to write to git stdin: {}", e);
                    break;
                }
            }
            drop(stdin);
        });
        
        // Read output as stream
        let output_stream = async_stream::try_stream! {
            let mut buffer = vec![0u8; 65536];
            loop {
                match stdout.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        yield UploadPackResponse {
                            data: buffer[..n].to_vec(),
                        };
                    }
                    Err(e) => {
                        error!("Failed to read from git stdout: {}", e);
                        break;
                    }
                }
            }
            write_task.await.ok();
        };
        
        Ok(Response::new(Box::pin(output_stream)))
    }
    
    type ReceivePackStream = Pin<Box<dyn Stream<Item = Result<ReceivePackResponse, Status>> + Send>>;
    
    async fn receive_pack(
        &self,
        request: Request<Streaming<ReceivePackRequest>>,
    ) -> Result<Response<Self::ReceivePackStream>, Status> {
        let mut stream = request.into_inner();
        
        // Get first message
        let first = stream.message().await?
            .ok_or_else(|| Status::invalid_argument("No request data"))?;
        
        let repo_path = self.get_repo_path(first.repository.as_ref())?;
        info!("Git receive-pack for {} by user {}", repo_path, first.username);
        
        // Spawn git-receive-pack process
        let mut child = Command::new(&self.config.git_bin_path)
            .arg("receive-pack")
            .arg("--stateless-rpc")
            .arg(&repo_path)
            .env("GL_ID", format!("user-{}", first.user_id))
            .env("GL_USERNAME", &first.username)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Status::internal(format!("Failed to spawn git: {}", e)))?;
        
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        
        // Write first chunk
        stdin.write_all(&first.data).await
            .map_err(|e| Status::internal(format!("Failed to write to git: {}", e)))?;
        
        // Spawn task to write remaining input
        let write_task = tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                if let Err(e) = stdin.write_all(&msg.data).await {
                    error!("Failed to write to git stdin: {}", e);
                    break;
                }
            }
            drop(stdin);
        });
        
        // Read output as stream
        let output_stream = async_stream::try_stream! {
            let mut buffer = vec![0u8; 65536];
            loop {
                match stdout.read(&mut buffer).await {
                    Ok(0) => break,
                    Ok(n) => {
                        yield ReceivePackResponse {
                            data: buffer[..n].to_vec(),
                        };
                    }
                    Err(e) => {
                        error!("Failed to read from git stdout: {}", e);
                        break;
                    }
                }
            }
            write_task.await.ok();
        };
        
        Ok(Response::new(Box::pin(output_stream)))
    }
}

// SSH Service implementation
pub struct SshServiceImpl {
    config: Arc<Config>,
}

impl SshServiceImpl {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }
    
    fn get_repo_path(&self, repo: Option<&Repository>) -> Result<String, Status> {
        let repo = repo.ok_or_else(|| Status::invalid_argument("Repository required"))?;
        
        if !repo.storage_path.is_empty() {
            Ok(repo.storage_path.clone())
        } else if !repo.relative_path.is_empty() {
            Ok(self.config.repo_path(&repo.relative_path))
        } else {
            Err(Status::invalid_argument("Repository path required"))
        }
    }
}

#[tonic::async_trait]
impl ssh_service_server::SshService for SshServiceImpl {
    type SshUploadPackStream = Pin<Box<dyn Stream<Item = Result<SshPackResponse, Status>> + Send>>;
    
    async fn ssh_upload_pack(
        &self,
        request: Request<Streaming<SshPackRequest>>,
    ) -> Result<Response<Self::SshUploadPackStream>, Status> {
        let mut stream = request.into_inner();
        
        let first = stream.message().await?
            .ok_or_else(|| Status::invalid_argument("No request data"))?;
        
        let repo_path = self.get_repo_path(first.repository.as_ref())?;
        debug!("SSH upload-pack for {}", repo_path);
        
        let mut child = Command::new(&self.config.git_bin_path)
            .arg("upload-pack")
            .arg(&repo_path)
            .envs(first.env_vars.iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Status::internal(format!("Failed to spawn git: {}", e)))?;
        
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        
        stdin.write_all(&first.stdin).await
            .map_err(|e| Status::internal(format!("Failed to write: {}", e)))?;
        
        let write_task = tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                if let Err(_) = stdin.write_all(&msg.stdin).await {
                    break;
                }
            }
            drop(stdin);
        });
        
        let output_stream = async_stream::try_stream! {
            let mut stdout_buf = vec![0u8; 65536];
            let mut stderr_buf = vec![0u8; 4096];
            
            loop {
                tokio::select! {
                    result = stdout.read(&mut stdout_buf) => {
                        match result {
                            Ok(0) => break,
                            Ok(n) => {
                                yield SshPackResponse {
                                    stdout: stdout_buf[..n].to_vec(),
                                    stderr: Vec::new(),
                                    exit_code: 0,
                                };
                            }
                            Err(_) => break,
                        }
                    }
                    result = stderr.read(&mut stderr_buf) => {
                        if let Ok(n) = result {
                            if n > 0 {
                                yield SshPackResponse {
                                    stdout: Vec::new(),
                                    stderr: stderr_buf[..n].to_vec(),
                                    exit_code: 0,
                                };
                            }
                        }
                    }
                }
            }
            
            write_task.await.ok();
            let status = child.wait().await.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
            yield SshPackResponse {
                stdout: Vec::new(),
                stderr: Vec::new(),
                exit_code: status,
            };
        };
        
        Ok(Response::new(Box::pin(output_stream)))
    }
    
    type SshReceivePackStream = Pin<Box<dyn Stream<Item = Result<SshPackResponse, Status>> + Send>>;
    
    async fn ssh_receive_pack(
        &self,
        request: Request<Streaming<SshPackRequest>>,
    ) -> Result<Response<Self::SshReceivePackStream>, Status> {
        let mut stream = request.into_inner();
        
        let first = stream.message().await?
            .ok_or_else(|| Status::invalid_argument("No request data"))?;
        
        let repo_path = self.get_repo_path(first.repository.as_ref())?;
        info!("SSH receive-pack for {}", repo_path);
        
        let mut child = Command::new(&self.config.git_bin_path)
            .arg("receive-pack")
            .arg(&repo_path)
            .envs(first.env_vars.iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Status::internal(format!("Failed to spawn git: {}", e)))?;
        
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        
        stdin.write_all(&first.stdin).await
            .map_err(|e| Status::internal(format!("Failed to write: {}", e)))?;
        
        let write_task = tokio::spawn(async move {
            while let Ok(Some(msg)) = stream.message().await {
                if let Err(_) = stdin.write_all(&msg.stdin).await {
                    break;
                }
            }
            drop(stdin);
        });
        
        let output_stream = async_stream::try_stream! {
            let mut stdout_buf = vec![0u8; 65536];
            let mut stderr_buf = vec![0u8; 4096];
            
            loop {
                tokio::select! {
                    result = stdout.read(&mut stdout_buf) => {
                        match result {
                            Ok(0) => break,
                            Ok(n) => {
                                yield SshPackResponse {
                                    stdout: stdout_buf[..n].to_vec(),
                                    stderr: Vec::new(),
                                    exit_code: 0,
                                };
                            }
                            Err(_) => break,
                        }
                    }
                    result = stderr.read(&mut stderr_buf) => {
                        if let Ok(n) = result {
                            if n > 0 {
                                yield SshPackResponse {
                                    stdout: Vec::new(),
                                    stderr: stderr_buf[..n].to_vec(),
                                    exit_code: 0,
                                };
                            }
                        }
                    }
                }
            }
            
            write_task.await.ok();
            let status = child.wait().await.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
            yield SshPackResponse {
                stdout: Vec::new(),
                stderr: Vec::new(),
                exit_code: status,
            };
        };
        
        Ok(Response::new(Box::pin(output_stream)))
    }
    
    type SshUploadArchiveStream = Pin<Box<dyn Stream<Item = Result<SshPackResponse, Status>> + Send>>;
    
    async fn ssh_upload_archive(
        &self,
        _request: Request<Streaming<SshPackRequest>>,
    ) -> Result<Response<Self::SshUploadArchiveStream>, Status> {
        Err(Status::unimplemented("upload-archive not yet implemented"))
    }
}
