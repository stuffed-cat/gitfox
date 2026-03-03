//! Git HTTP Smart Protocol handler via GitLayer
//!
//! 直接通过 GitLayer gRPC 处理 Git HTTP 请求，
//! 实现真正的双向流式传输，支持大仓库 push/clone。
//!
//! 关键性能优化：
//! 1. 请求体流式读取 → 流式发送到 GitLayer（无内存累积）
//! 2. GitLayer 响应流式返回 → 流式发送给客户端
//! 3. 背压控制：当客户端或 GitLayer 变慢时自动节流

use actix_web::{web, Error, HttpRequest, HttpResponse};
use bytes::{Bytes, BytesMut};
use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Channel;
use tracing::{debug, error, info, warn};

use crate::auth_client::AuthClient;
use crate::config::Config;
use crate::gitlayer_client::proto::{
    self,
    smart_http_service_client::SmartHttpServiceClient,
    Repository, InfoRefsRequest, UploadPackRequest, ReceivePackRequest,
};

/// Git HTTP handler 状态
pub struct GitHttpState {
    pub config: Arc<Config>,
    pub auth_client: Option<AuthClient>,
    channel: tokio::sync::RwLock<Option<Channel>>,
}

impl GitHttpState {
    pub fn new(config: Arc<Config>, auth_client: Option<AuthClient>) -> Self {
        Self {
            config,
            auth_client,
            channel: tokio::sync::RwLock::new(None),
        }
    }

    /// 获取或创建 GitLayer channel
    async fn get_channel(&self) -> Result<Channel, String> {
        // 快速路径：已有连接
        {
            let read_guard = self.channel.read().await;
            if let Some(ref ch) = *read_guard {
                return Ok(ch.clone());
            }
        }

        // 慢路径：创建新连接
        let mut write_guard = self.channel.write().await;

        // Double-check
        if let Some(ref ch) = *write_guard {
            return Ok(ch.clone());
        }

        let gitlayer_addr = self.config.gitlayer_address.as_ref()
            .ok_or_else(|| "GITLAYER_ADDRESS not configured".to_string())?;

        debug!("Connecting to GitLayer at {}", gitlayer_addr);

        let channel = Channel::from_shared(gitlayer_addr.clone())
            .map_err(|e| format!("Invalid GitLayer address: {}", e))?
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(3600)) // 1 hour for large repos
            .connect()
            .await
            .map_err(|e| format!("GitLayer connection failed: {}", e))?;

        *write_guard = Some(channel.clone());
        Ok(channel)
    }
}

/// 解析 Git 路径：/{namespace}/{project}.git/... → (namespace, project)
fn parse_git_path(path: &str) -> Option<(String, String)> {
    // 移除前导斜杠和 .git 后缀及后续路径
    let path = path.trim_start_matches('/');
    
    // 找到 .git 的位置
    let git_suffix_pos = path.find(".git")?;
    let repo_path = &path[..git_suffix_pos];
    
    // 分割最后一个 / 得到 namespace 和 project
    let last_slash = repo_path.rfind('/')?;
    let namespace = &repo_path[..last_slash];
    let project = &repo_path[last_slash + 1..];
    
    if namespace.is_empty() || project.is_empty() {
        return None;
    }
    
    Some((namespace.to_string(), project.to_string()))
}

/// 构建仓库的 relative_path
fn build_repo_relative_path(namespace: &str, project: &str) -> String {
    format!("{}/{}.git", namespace, project)
}

/// GET /{namespace}/{project}.git/info/refs?service=git-upload-pack|git-receive-pack
pub async fn handle_info_refs(
    req: HttpRequest,
    query: web::Query<InfoRefsQuery>,
    state: web::Data<GitHttpState>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    let (namespace, project) = parse_git_path(path)
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Invalid git path"))?;

    let service = query.service.as_deref().unwrap_or("git-upload-pack");
    
    // 验证 service 参数
    if service != "git-upload-pack" && service != "git-receive-pack" {
        return Ok(HttpResponse::Forbidden().body("Invalid service"));
    }

    info!("Git info/refs: {}/{} service={}", namespace, project, service);

    // TODO: 认证检查（对于 receive-pack 需要写权限）
    // if service == "git-receive-pack" {
    //     verify_write_access(&req, &state, &namespace, &project).await?;
    // }

    let channel = state.get_channel().await
        .map_err(actix_web::error::ErrorServiceUnavailable)?;

    let mut client = SmartHttpServiceClient::new(channel);

    let repo_path = build_repo_relative_path(&namespace, &project);
    let repository = Repository {
        storage_path: String::new(),
        relative_path: repo_path,
    };

    let response = client
        .info_refs(InfoRefsRequest {
            repository: Some(repository),
            service: service.to_string(),
        })
        .await
        .map_err(|e| {
            error!("GitLayer info_refs failed: {}", e);
            actix_web::error::ErrorBadGateway(format!("GitLayer error: {}", e))
        })?;

    let data = response.into_inner().data;
    let content_type = format!("application/x-{}-advertisement", service);

    Ok(HttpResponse::Ok()
        .content_type(content_type)
        .append_header(("Cache-Control", "no-cache"))
        .body(data))
}

/// POST /{namespace}/{project}.git/git-upload-pack
/// 
/// 流式处理 fetch/clone 请求
pub async fn handle_upload_pack(
    req: HttpRequest,
    mut payload: web::Payload,
    state: web::Data<GitHttpState>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    let (namespace, project) = parse_git_path(path)
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Invalid git path"))?;

    info!("Git upload-pack: {}/{}", namespace, project);

    let channel = state.get_channel().await
        .map_err(actix_web::error::ErrorServiceUnavailable)?;

    let mut client = SmartHttpServiceClient::new(channel);

    let repo_path = build_repo_relative_path(&namespace, &project);
    let repository = Repository {
        storage_path: String::new(),
        relative_path: repo_path,
    };

    // 创建请求流（流式读取 HTTP body）
    let (tx, rx) = mpsc::channel::<UploadPackRequest>(16);
    
    // 首次发送包含 repository 信息
    let first_msg = UploadPackRequest {
        repository: Some(repository.clone()),
        data: Vec::new(),
    };
    
    // 启动后台任务：从 HTTP payload 流式读取并发送到 gRPC
    // 使用 actix_web::rt::spawn 因为 Payload 不是 Send
    let payload_task = actix_web::rt::spawn(async move {
        // 先发送首条消息
        if tx.send(first_msg).await.is_err() {
            return;
        }
        
        // 流式读取 HTTP body
        while let Some(chunk_result) = payload.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let msg = UploadPackRequest {
                        repository: None, // 后续消息不需要重复 repository
                        data: chunk.to_vec(),
                    };
                    if tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Error reading HTTP payload: {}", e);
                    break;
                }
            }
        }
        // tx drop 时自动关闭 stream
    });

    // 发送请求到 GitLayer
    let request_stream = ReceiverStream::new(rx);
    let response = client
        .upload_pack(request_stream)
        .await
        .map_err(|e| {
            error!("GitLayer upload_pack failed: {}", e);
            actix_web::error::ErrorBadGateway(format!("GitLayer error: {}", e))
        })?;

    // 流式返回响应
    let response_stream = response.into_inner().map(|result| {
        result
            .map(|r| Bytes::from(r.data))
            .map_err(|e| {
                error!("GitLayer stream error: {}", e);
                actix_web::error::ErrorBadGateway(format!("Stream error: {}", e))
            })
    });

    // 等待 payload 任务完成（非阻塞，响应流会继续）
    tokio::spawn(async move {
        let _ = payload_task.await;
    });

    Ok(HttpResponse::Ok()
        .content_type("application/x-git-upload-pack-result")
        .append_header(("Cache-Control", "no-cache"))
        .streaming(response_stream))
}

/// POST /{namespace}/{project}.git/git-receive-pack
/// 
/// 流式处理 push 请求
/// 这是解决大仓库 push 502/504 的关键函数
pub async fn handle_receive_pack(
    req: HttpRequest,
    mut payload: web::Payload,
    state: web::Data<GitHttpState>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    let (namespace, project) = parse_git_path(path)
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Invalid git path"))?;

    info!("Git receive-pack: {}/{}", namespace, project);

    // TODO: 认证检查
    // let (user_id, username) = verify_write_access(&req, &state, &namespace, &project).await?;
    let user_id: i64 = 0; // 临时：需要从认证中获取
    let username = "anonymous".to_string();

    let channel = state.get_channel().await
        .map_err(actix_web::error::ErrorServiceUnavailable)?;

    let mut client = SmartHttpServiceClient::new(channel);

    let repo_path = build_repo_relative_path(&namespace, &project);
    let repository = Repository {
        storage_path: String::new(),
        relative_path: repo_path,
    };

    // 创建请求流
    // 使用较大的 buffer 以支持高吞吐量，但仍然是有界的以防止 OOM
    let (tx, rx) = mpsc::channel::<ReceivePackRequest>(32);
    
    // 首次发送包含完整信息
    let first_msg = ReceivePackRequest {
        repository: Some(repository.clone()),
        data: Vec::new(),
        push_options: Vec::new(),
        user_id,
        username: username.clone(),
    };
    
    // 统计信息用于日志
    let stats = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stats_clone = stats.clone();

    // 后台任务：流式读取 HTTP body → 发送到 gRPC
    // 使用 actix_web::rt::spawn 因为 Payload 不是 Send
    let payload_task = actix_web::rt::spawn(async move {
        if tx.send(first_msg).await.is_err() {
            return;
        }
        
        let mut total_bytes: usize = 0;
        let mut chunk_count: usize = 0;
        
        while let Some(chunk_result) = payload.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let chunk_len = chunk.len();
                    total_bytes += chunk_len;
                    chunk_count += 1;
                    
                    let msg = ReceivePackRequest {
                        repository: None,
                        data: chunk.to_vec(),
                        push_options: Vec::new(),
                        user_id: 0,
                        username: String::new(),
                    };
                    
                    // 背压：如果 GitLayer 处理慢，这里会等待
                    if tx.send(msg).await.is_err() {
                        warn!("GitLayer receiver dropped, aborting push");
                        break;
                    }
                    
                    // 每 100 chunks 记录一次进度
                    if chunk_count % 100 == 0 {
                        debug!(
                            "Push progress: {} chunks, {} MB",
                            chunk_count,
                            total_bytes / (1024 * 1024)
                        );
                    }
                }
                Err(e) => {
                    error!("Error reading push payload: {}", e);
                    break;
                }
            }
        }
        
        stats_clone.store(total_bytes, std::sync::atomic::Ordering::Relaxed);
        info!(
            "Push payload complete: {} chunks, {} bytes ({} MB)",
            chunk_count,
            total_bytes,
            total_bytes / (1024 * 1024)
        );
    });

    // 发送到 GitLayer
    let request_stream = ReceiverStream::new(rx);
    let response = client
        .receive_pack(request_stream)
        .await
        .map_err(|e| {
            error!("GitLayer receive_pack failed: {}", e);
            actix_web::error::ErrorBadGateway(format!("GitLayer error: {}", e))
        })?;

    // 流式返回响应
    let response_stream = response.into_inner().map(|result| {
        result
            .map(|r| Bytes::from(r.data))
            .map_err(|e| {
                error!("GitLayer stream error: {}", e);
                actix_web::error::ErrorBadGateway(format!("Stream error: {}", e))
            })
    });

    // 异步等待 payload 任务
    tokio::spawn(async move {
        let _ = payload_task.await;
        let total_bytes = stats.load(std::sync::atomic::Ordering::Relaxed);
        info!("Push complete, total {} MB", total_bytes / (1024 * 1024));
    });

    Ok(HttpResponse::Ok()
        .content_type("application/x-git-receive-pack-result")
        .append_header(("Cache-Control", "no-cache"))
        .streaming(response_stream))
}

#[derive(Debug, serde::Deserialize)]
pub struct InfoRefsQuery {
    pub service: Option<String>,
}

/// 配置 Git HTTP 路由
pub fn configure_git_routes(cfg: &mut web::ServiceConfig, state: web::Data<GitHttpState>) {
    cfg.app_data(state)
        // info/refs
        .route(
            "/{namespace}/{project}.git/info/refs",
            web::get().to(handle_info_refs),
        )
        // upload-pack (fetch/clone)
        .route(
            "/{namespace}/{project}.git/git-upload-pack",
            web::post().to(handle_upload_pack),
        )
        // receive-pack (push)
        .route(
            "/{namespace}/{project}.git/git-receive-pack",
            web::post().to(handle_receive_pack),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_path() {
        assert_eq!(
            parse_git_path("/user/repo.git/info/refs"),
            Some(("user".to_string(), "repo".to_string()))
        );
        assert_eq!(
            parse_git_path("/org/team/project.git/git-receive-pack"),
            Some(("org/team".to_string(), "project".to_string()))
        );
        assert_eq!(
            parse_git_path("/a/b/c/d.git"),
            Some(("a/b/c".to_string(), "d".to_string()))
        );
        assert_eq!(parse_git_path("/invalid"), None);
        assert_eq!(parse_git_path("/.git/info/refs"), None);
    }
    
    #[test]
    fn test_build_repo_path() {
        assert_eq!(
            build_repo_relative_path("user", "repo"),
            "user/repo.git"
        );
        assert_eq!(
            build_repo_relative_path("org/team", "project"),
            "org/team/project.git"
        );
    }
}
