//! Registry 认证模块
//!
//! 处理 Docker Registry 和 npm Registry 的认证

use actix_web::{HttpRequest, HttpResponse, http::StatusCode};
use base64::Engine;
use tracing::{debug, warn};

use crate::auth_client::{AuthClient, HttpAccessResult};
use super::types::{DockerError, NpmError, AuthResult, WwwAuthenticate, DockerTokenResponse};

/// 从请求中提取 Basic Auth 凭据
pub fn extract_basic_auth(req: &HttpRequest) -> Option<(String, String)> {
    let auth_header = req.headers().get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;

    if !auth_str.starts_with("Basic ") {
        return None;
    }

    let encoded = &auth_str[6..];
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).ok()?;
    let credentials = String::from_utf8(decoded).ok()?;
    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
    
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

/// 从请求中提取 Bearer Token
pub fn extract_bearer_token(req: &HttpRequest) -> Option<String> {
    let auth_header = req.headers().get("authorization")?;
    let auth_str = auth_header.to_str().ok()?;

    if auth_str.starts_with("Bearer ") {
        Some(auth_str[7..].to_string())
    } else {
        None
    }
}

/// Docker Registry 认证
/// 
/// Docker 使用 token-based 认证：
/// 1. 客户端请求资源
/// 2. Registry 返回 401 + WWW-Authenticate header
/// 3. 客户端用凭据向 token endpoint 请求 token
/// 4. 客户端用 token 重新请求资源
pub async fn docker_authenticate(
    req: &HttpRequest,
    repository: &str,
    action: &str,  // "pull" 或 "push"
    auth_client: &mut AuthClient,
) -> Result<AuthResult, HttpResponse> {
    debug!("Docker auth: repository={}, action={}", repository, action);

    // 解析仓库路径 (namespace/project/image 或 namespace/project)
    let parts: Vec<&str> = repository.split('/').collect();
    if parts.len() < 2 {
        return Err(HttpResponse::build(StatusCode::BAD_REQUEST)
            .json(DockerError::name_invalid("repository name must include namespace")));
    }

    // 提取 namespace 和 project
    // 格式可能是: namespace/project 或 namespace/project/image
    let (namespace, project_name) = if parts.len() >= 2 {
        (parts[0], parts[1])
    } else {
        return Err(HttpResponse::build(StatusCode::BAD_REQUEST)
            .json(DockerError::name_invalid("invalid repository format")));
    };

    let repo_path = format!("{}/{}", namespace, project_name);
    let grpc_action = if action == "push" { "git-receive-pack" } else { "git-upload-pack" };

    // 尝试从请求中获取认证信息
    if let Some((username, password)) = extract_basic_auth(req) {
        // Basic Auth
        match auth_client.check_http_access_basic(&repo_path, grpc_action, &username, &password).await {
            Ok(result) if result.allowed => {
                return Ok(AuthResult {
                    user_id: uuid::Uuid::nil(), // 需要从 result 获取
                    username: result.username,
                    project_id: result.project_id,
                    can_read: true,
                    can_write: result.can_write,
                });
            }
            Ok(_) => {
                warn!("Docker auth denied for user: {}", username);
                return Err(HttpResponse::build(StatusCode::FORBIDDEN)
                    .json(DockerError::denied()));
            }
            Err(e) => {
                warn!("Docker auth error: {}", e);
                // 返回 401 让客户端重新认证
            }
        }
    } else if let Some(token) = extract_bearer_token(req) {
        // Bearer Token (JWT 或 PAT)
        match auth_client.check_http_access_jwt(&repo_path, grpc_action, &token).await {
            Ok(result) if result.allowed => {
                return Ok(AuthResult {
                    user_id: uuid::Uuid::nil(),
                    username: result.username,
                    project_id: result.project_id,
                    can_read: true,
                    can_write: result.can_write,
                });
            }
            Ok(_) => {
                return Err(HttpResponse::build(StatusCode::FORBIDDEN)
                    .json(DockerError::denied()));
            }
            Err(e) => {
                warn!("Docker auth error: {}", e);
            }
        }
    }

    // 尝试匿名访问（公开仓库可能允许 pull）
    if action == "pull" {
        match auth_client.check_http_access_anonymous(&repo_path, grpc_action).await {
            Ok(result) if result.allowed => {
                return Ok(AuthResult {
                    user_id: uuid::Uuid::nil(),
                    username: "anonymous".to_string(),
                    project_id: result.project_id,
                    can_read: true,
                    can_write: false,
                });
            }
            _ => {}
        }
    }

    // 返回 401 要求认证
    // 注意: 实际部署时需要配置正确的 realm URL
    Err(HttpResponse::build(StatusCode::UNAUTHORIZED)
        .insert_header(("WWW-Authenticate", format!(
            "Bearer realm=\"/v2/auth\",service=\"registry\",scope=\"repository:{}:{}\"",
            repository,
            action
        )))
        .json(DockerError::unauthorized()))
}

/// npm Registry 认证
pub async fn npm_authenticate(
    req: &HttpRequest,
    package_name: &str,
    action: &str,  // "read" 或 "write"
    auth_client: &mut AuthClient,
) -> Result<AuthResult, HttpResponse> {
    debug!("npm auth: package={}, action={}", package_name, action);

    // npm 包名格式: @scope/name 或 name
    // 映射到 GitFox: namespace/project
    let (namespace, project_name) = parse_npm_package_name(package_name)?;
    let repo_path = format!("{}/{}", namespace, project_name);
    let grpc_action = if action == "write" { "git-receive-pack" } else { "git-upload-pack" };

    // 从 Authorization header 或 _authToken 获取认证信息
    if let Some((username, password)) = extract_basic_auth(req) {
        match auth_client.check_http_access_basic(&repo_path, grpc_action, &username, &password).await {
            Ok(result) if result.allowed => {
                return Ok(AuthResult {
                    user_id: uuid::Uuid::nil(),
                    username: result.username,
                    project_id: result.project_id,
                    can_read: true,
                    can_write: result.can_write,
                });
            }
            Ok(_) => {
                return Err(HttpResponse::build(StatusCode::FORBIDDEN)
                    .json(NpmError::forbidden()));
            }
            Err(e) => {
                warn!("npm auth error: {}", e);
            }
        }
    } else if let Some(token) = extract_bearer_token(req) {
        match auth_client.check_http_access_jwt(&repo_path, grpc_action, &token).await {
            Ok(result) if result.allowed => {
                return Ok(AuthResult {
                    user_id: uuid::Uuid::nil(),
                    username: result.username,
                    project_id: result.project_id,
                    can_read: true,
                    can_write: result.can_write,
                });
            }
            Ok(_) => {
                return Err(HttpResponse::build(StatusCode::FORBIDDEN)
                    .json(NpmError::forbidden()));
            }
            Err(e) => {
                warn!("npm auth error: {}", e);
            }
        }
    }

    // 尝试匿名访问
    if action == "read" {
        match auth_client.check_http_access_anonymous(&repo_path, grpc_action).await {
            Ok(result) if result.allowed => {
                return Ok(AuthResult {
                    user_id: uuid::Uuid::nil(),
                    username: "anonymous".to_string(),
                    project_id: result.project_id,
                    can_read: true,
                    can_write: false,
                });
            }
            _ => {}
        }
    }

    Err(HttpResponse::build(StatusCode::UNAUTHORIZED)
        .json(NpmError::unauthorized()))
}

/// 解析 npm 包名
/// @scope/name -> (scope, name)
/// name -> 需要从 URL 路径中获取 namespace
fn parse_npm_package_name(name: &str) -> Result<(String, String), HttpResponse> {
    if name.starts_with('@') {
        // @scope/name
        let parts: Vec<&str> = name[1..].splitn(2, '/').collect();
        if parts.len() == 2 {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(HttpResponse::build(StatusCode::BAD_REQUEST)
                .json(NpmError::bad_request("invalid scoped package name")))
        }
    } else {
        // 对于非 scoped 包，需要从 URL 中获取 project
        // 返回错误，让调用者处理
        Err(HttpResponse::build(StatusCode::BAD_REQUEST)
            .json(NpmError::bad_request("package must be scoped (@namespace/name)")))
    }
}

/// 生成 Docker token 响应
pub fn generate_docker_token(
    username: &str,
    repository: &str,
    actions: &[&str],
    expires_in: u64,
    secret: &str,
) -> DockerTokenResponse {
    use jsonwebtoken::{encode, Header, EncodingKey};
    use serde::{Serialize, Deserialize};
    use chrono::Utc;

    #[derive(Debug, Serialize, Deserialize)]
    struct TokenClaims {
        iss: String,
        sub: String,
        aud: String,
        exp: i64,
        nbf: i64,
        iat: i64,
        jti: String,
        access: Vec<TokenAccess>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct TokenAccess {
        #[serde(rename = "type")]
        resource_type: String,
        name: String,
        actions: Vec<String>,
    }

    let now = Utc::now();
    let claims = TokenClaims {
        iss: "gitfox-registry".to_string(),
        sub: username.to_string(),
        aud: "registry".to_string(),
        exp: now.timestamp() + expires_in as i64,
        nbf: now.timestamp(),
        iat: now.timestamp(),
        jti: uuid::Uuid::new_v4().to_string(),
        access: vec![TokenAccess {
            resource_type: "repository".to_string(),
            name: repository.to_string(),
            actions: actions.iter().map(|s| s.to_string()).collect(),
        }],
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap_or_default();

    DockerTokenResponse {
        token: token.clone(),
        access_token: Some(token),
        expires_in: expires_in as i64,
        issued_at: now.to_rfc3339(),
    }
}
