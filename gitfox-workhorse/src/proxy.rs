use actix_web::{web, Error, HttpRequest, HttpResponse};
use crate::http_client::BackendClient;
use futures::StreamExt;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use std::time::Duration;

/// 反向代理处理器 - 将请求转发到后端服务器
pub async fn proxy_to_backend(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<BackendClient>,
    backend_url: web::Data<String>,
    backend_socket: web::Data<Option<String>>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let method = req.method().clone();

    tracing::debug!("Proxying {} {} (backend: {:?})", method, path, 
        if backend_socket.is_some() { "unix" } else { "tcp" });

    // 根据客户端类型选择代理方式
    if client.is_unix() {
        proxy_via_unix(req, body, client, &path, &query).await
    } else {
        proxy_via_tcp(req, body, client, backend_url, &path, &query).await
    }
}

/// 通过 TCP 代理
async fn proxy_via_tcp(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<BackendClient>,
    backend_url: web::Data<String>,
    path: &str,
    query: &str,
) -> Result<HttpResponse, Error> {
    let reqwest_client = client.as_reqwest()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Expected TCP client"))?;

    // 构建目标 URL
    let target_url = if query.is_empty() {
        format!("{}{}", backend_url.as_ref(), path)
    } else {
        format!("{}{}?{}", backend_url.as_ref(), path, query)
    };

    // 构建请求
    let mut backend_req = reqwest_client
        .request(req.method().clone(), &target_url)
        .timeout(Duration::from_secs(300));

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

/// 通过 Unix Socket 代理
async fn proxy_via_unix(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<BackendClient>,
    path: &str,
    query: &str,
) -> Result<HttpResponse, Error> {
    // 构建 Unix Socket URI
    let uri_path = if query.is_empty() {
        path.to_string()
    } else {
        format!("{}?{}", path, query)
    };

    let uri = client.build_unix_uri(&uri_path)
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Failed to build Unix URI"))?;

    // 构建 hyper 请求
    let mut hyper_req = hyper::Request::builder()
        .method(req.method().as_str())
        .uri(uri);

    // 转发请求头 (需转换actix header到hyper header)
    for (name, value) in req.headers().iter() {
        let name_str = name.as_str();
        if name_str != "host" && name_str != "connection" {
            if let Ok(value_bytes) = value.to_str() {
                hyper_req = hyper_req.header(name_str, value_bytes);
            }
        }
    }

    // 添加 X-Forwarded-* 头
    if let Some(peer_addr) = req.peer_addr() {
        hyper_req = hyper_req.header("x-forwarded-for", peer_addr.ip().to_string());
    }
    hyper_req = hyper_req.header("x-forwarded-proto", req.connection_info().scheme());
    if let Some(host) = req.headers().get("host") {
        if let Ok(host_str) = host.to_str() {
            hyper_req = hyper_req.header("x-forwarded-host", host_str);
        }
    }

    // 添加请求体
    let hyper_body = if body.is_empty() {
        Full::new(Bytes::new()).map_err(|never| match never {}).boxed()
    } else {
        Full::new(Bytes::copy_from_slice(&body[..])).map_err(|never| match never {}).boxed()
    };

    let hyper_req = hyper_req.body(hyper_body)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // 发送请求
    let backend_res = match client.send_unix_request(hyper_req).await {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Unix socket request failed: {}", e);
            return Ok(HttpResponse::BadGateway()
                .json(serde_json::json!({
                    "error": "Backend service unavailable (Unix socket)",
                    "message": format!("{}", e)
                })));
        }
    };

    let status = backend_res.status();
    let mut client_resp = HttpResponse::build(actix_web::http::StatusCode::from_u16(status.as_u16())
        .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR));

    // 转发响应头 (hyper header -> actix header)
    for (name, value) in backend_res.headers().iter() {
        let name_str = name.as_str();
        if name_str != "connection" && name_str != "transfer-encoding" {
            if let Ok(value_str) = value.to_str() {
                client_resp.insert_header((name_str, value_str));
            }
        }
    }

    // 读取响应体
    let (parts, incoming) = backend_res.into_parts();
    let body_bytes = incoming.collect().await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .to_bytes();

    Ok(client_resp.body(body_bytes.to_vec()))
}

/// Git HTTP 协议代理 - 特殊处理 Git 相关请求
pub async fn proxy_git_http(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<BackendClient>,
    backend_url: web::Data<String>,
    backend_socket: web::Data<Option<String>>,
) -> Result<HttpResponse, Error> {
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let method = req.method().clone();

    tracing::info!("Git HTTP request: {} {}", method, path);

    // 根据客户端类型选择代理方式（Git 操作超时时间更长）
    if client.is_unix() {
        proxy_via_unix(req, body, client, &path, &query).await
    } else {
        proxy_git_via_tcp(req, body, client, backend_url, &path, &query).await
    }
}

/// 通过 TCP 代理 Git 请求（超时时间更长）
async fn proxy_git_via_tcp(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<BackendClient>,
    backend_url: web::Data<String>,
    path: &str,
    query: &str,
) -> Result<HttpResponse, Error> {
    let reqwest_client = client.as_reqwest()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Expected TCP client"))?;

    // 构建目标 URL
    let target_url = if query.is_empty() {
        format!("{}{}", backend_url.as_ref(), path)
    } else {
        format!("{}{}?{}", backend_url.as_ref(), path, query)
    };

    // 构建请求
    let mut backend_req = reqwest_client
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
