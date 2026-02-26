use actix_web::{web, Error, HttpRequest, HttpResponse};
use futures::StreamExt;
use reqwest::Client;
use std::time::Duration;

/// 反向代理处理器 - 将请求转发到后端服务器
pub async fn proxy_to_backend(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Client>,
    backend_url: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");
    
    // 构建目标 URL
    let target_url = if query.is_empty() {
        format!("{}{}", backend_url.as_ref(), path)
    } else {
        format!("{}{}?{}", backend_url.as_ref(), path, query)
    };

    tracing::debug!("Proxying {} {} to {}", req.method(), path, target_url);

    // 构建请求
    let mut backend_req = client
        .request(req.method().clone(), &target_url)
        .timeout(Duration::from_secs(300)); // 5分钟超时，适用于大文件上传

    // 转发请求头（排除 Host 和 Connection）
    for (name, value) in req.headers().iter() {
        let name_str = name.as_str();
        if name_str != "host" && name_str != "connection" {
            if let Ok(value_str) = value.to_str() {
                backend_req = backend_req.header(name_str, value_str);
            }
        }
    }

    // 添加 X-Forwarded-* 头
    if let Some(peer_addr) = req.peer_addr() {
        backend_req = backend_req.header("X-Forwarded-For", peer_addr.ip().to_string());
    }
    backend_req = backend_req.header("X-Forwarded-Proto", req.connection_info().scheme());
    if let Some(host) = req.headers().get("host") {
        if let Ok(host_str) = host.to_str() {
            backend_req = backend_req.header("X-Forwarded-Host", host_str);
        }
    }

    // 添加请求体
    if !body.is_empty() {
        backend_req = backend_req.body(body.to_vec());
    }

    // 发送请求
    let backend_res = match backend_req.send().await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Backend request failed: {}", e);
            return Ok(HttpResponse::BadGateway()
                .json(serde_json::json!({
                    "error": "Backend service unavailable",
                    "message": format!("{}", e)
                })));
        }
    };

    let status = backend_res.status();
    let mut client_resp = HttpResponse::build(status);

    // 转发响应头（排除 Connection 和 Transfer-Encoding）
    for (name, value) in backend_res.headers().iter() {
        let name_str = name.as_str();
        if name_str != "connection" && name_str != "transfer-encoding" {
            client_resp.insert_header((name.clone(), value.clone()));
        }
    }

    // 流式传输响应体
    let stream = backend_res.bytes_stream();
    Ok(client_resp.streaming(stream.map(|result| {
        result.map_err(|e| {
            tracing::error!("Error streaming response: {}", e);
            actix_web::error::ErrorInternalServerError(e)
        })
    })))
}

/// Git HTTP 协议代理 - 特殊处理 Git 相关请求
pub async fn proxy_git_http(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Client>,
    backend_url: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");

    tracing::info!("Git HTTP request: {} {}", req.method(), path);

    // 构建目标 URL
    let target_url = if query.is_empty() {
        format!("{}{}", backend_url.as_ref(), path)
    } else {
        format!("{}{}?{}", backend_url.as_ref(), path, query)
    };

    // 构建请求
    let mut backend_req = client
        .request(req.method().clone(), &target_url)
        .timeout(Duration::from_secs(600)); // Git 操作可能需要更长时间

    // 转发请求头
    for (name, value) in req.headers().iter() {
        let name_str = name.as_str();
        if name_str != "host" && name_str != "connection" {
            if let Ok(value_str) = value.to_str() {
                backend_req = backend_req.header(name_str, value_str);
            }
        }
    }

    // 添加 X-Forwarded-* 头
    if let Some(peer_addr) = req.peer_addr() {
        backend_req = backend_req.header("X-Forwarded-For", peer_addr.ip().to_string());
    }
    backend_req = backend_req.header("X-Forwarded-Proto", req.connection_info().scheme());

    // 添加请求体
    if !body.is_empty() {
        backend_req = backend_req.body(body.to_vec());
    }

    // 发送请求
    let backend_res = match backend_req.send().await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Git backend request failed: {}", e);
            return Ok(HttpResponse::BadGateway()
                .content_type("text/plain")
                .body(format!("Git backend service unavailable: {}", e)));
        }
    };

    let status = backend_res.status();
    let mut client_resp = HttpResponse::build(status);

    // 转发响应头
    for (name, value) in backend_res.headers().iter() {
        let name_str = name.as_str();
        if name_str != "connection" && name_str != "transfer-encoding" {
            client_resp.insert_header((name.clone(), value.clone()));
        }
    }

    // 流式传输响应体（Git packfile 可能很大）
    let stream = backend_res.bytes_stream();
    Ok(client_resp.streaming(stream.map(|result| {
        result.map_err(|e| {
            tracing::error!("Error streaming Git response: {}", e);
            actix_web::error::ErrorInternalServerError(e)
        })
    })))
}

/// 健康检查端点
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "gitfox-workhorse",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
mod tests {
    use super::*;

    #[test]
    fn test_should_proxy_to_backend() {
        assert!(should_proxy_to_backend("/api/v1/users"));
        assert!(should_proxy_to_backend("/oauth/token"));
        assert!(should_proxy_to_backend("/namespace/repo.git"));
        assert!(should_proxy_to_backend("/namespace/repo.git/info/refs"));
        assert!(!should_proxy_to_backend("/assets/logo.png"));
        assert!(!should_proxy_to_backend("/-/ide/index.html"));
    }

    #[test]
    fn test_is_git_http_path() {
        assert!(is_git_http_path("/namespace/project.git"));
        assert!(is_git_http_path("/namespace/project.git/info/refs"));
        assert!(is_git_http_path("/namespace/project.git/git-upload-pack"));
        assert!(!is_git_http_path("/api/v1/projects"));
    }
}
