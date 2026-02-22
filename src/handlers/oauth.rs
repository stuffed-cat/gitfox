use actix_web::{web, HttpResponse, HttpRequest};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::middleware::AuthenticatedUser;
use crate::models::{
    OAuthApplication, OAuthApplicationInfo, OAuthApplicationWithSecret,
    CreateOAuthApplicationRequest, UpdateOAuthApplicationRequest,
    OAuthAuthorizeRequest, OAuthTokenRequest, OAuthTokenResponse,
    OAuthTokenRevocationRequest, OAuthUserInfoResponse,
    OAuthProviderRecord, OAuthProviderInfo, OAuthProvidersResponse,
    OAuthIdentityInfo, UserRole,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helper functions
// ─────────────────────────────────────────────────────────────────────────────

fn generate_client_id() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

fn generate_client_secret() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    format!("gfcs_{}", hex::encode(bytes))
}

fn generate_auth_code() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

fn generate_access_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    format!("gfat_{}", hex::encode(bytes))
}

fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

fn verify_pkce(verifier: &str, challenge: &str, method: &str) -> bool {
    match method {
        "plain" => verifier == challenge,
        "S256" => {
            let mut hasher = Sha256::new();
            hasher.update(verifier.as_bytes());
            let computed = base64_url_encode(&hasher.finalize());
            computed == challenge
        }
        _ => false,
    }
}

fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

/// Generate OIDC ID Token (JWT)
fn generate_id_token(
    config: &AppConfig,
    user_id: i64,
    username: &str,
    email: Option<&str>,
    client_id: &str,
    scopes: &[String],
) -> AppResult<Option<String>> {
    // Only generate id_token if 'openid' scope is present
    if !scopes.iter().any(|s| s == "openid") {
        return Ok(None);
    }

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct IdTokenClaims {
        /// Issuer
        iss: String,
        /// Subject (user ID)
        sub: String,
        /// Audience (client ID)
        aud: String,
        /// Expiration time
        exp: i64,
        /// Issued at
        iat: i64,
        /// Authorized party
        azp: String,
        /// Username
        preferred_username: String,
        /// Email (if email scope is present)
        #[serde(skip_serializing_if = "Option::is_none")]
        email: Option<String>,
        /// Email verified
        #[serde(skip_serializing_if = "Option::is_none")]
        email_verified: Option<bool>,
    }

    let now = Utc::now().timestamp();
    let exp = now + 3600; // 1 hour

    let claims = IdTokenClaims {
        iss: config.base_url.clone(),
        sub: user_id.to_string(),
        aud: client_id.to_string(),
        exp,
        iat: now,
        azp: client_id.to_string(),
        preferred_username: username.to_string(),
        email: if scopes.iter().any(|s| s == "email") {
            email.map(String::from)
        } else {
            None
        },
        email_verified: if email.is_some() { Some(true) } else { None },
    };

    let header = Header::new(Algorithm::HS256);
    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    ).map_err(|e| AppError::InternalError(format!("Failed to generate id_token: {}", e)))?;

    Ok(Some(token))
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Applications (GitFox as Provider)
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/v1/oauth/applications - List user's OAuth applications
pub async fn list_applications(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let apps = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE owner_id = $1 ORDER BY created_at DESC"
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await?;

    let app_infos: Vec<OAuthApplicationInfo> = apps.into_iter().map(|a| a.into()).collect();
    Ok(HttpResponse::Ok().json(app_infos))
}

/// POST /api/v1/oauth/applications - Create a new OAuth application
pub async fn create_application(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    body: web::Json<CreateOAuthApplicationRequest>,
) -> AppResult<HttpResponse> {
    let req = body.into_inner();

    // Validate redirect URIs
    for uri in &req.redirect_uris {
        if !uri.starts_with("http://") && !uri.starts_with("https://") && !uri.starts_with("urn:") {
            return Err(AppError::BadRequest(format!("Invalid redirect URI: {}", uri)));
        }
    }

    let client_id = generate_client_id();
    let client_secret = generate_client_secret();
    let client_secret_hash = hash_secret(&client_secret);
    let scopes = req.scopes.unwrap_or_else(|| vec!["read_user".to_string()]);
    let confidential = req.confidential.unwrap_or(true);

    // Check if user is admin
    let is_admin = auth.role == "Admin";

    // Only admins can create trusted applications
    let trusted = if is_admin {
        req.trusted.unwrap_or(false)
    } else {
        false
    };

    // Only trusted applications can skip authorization
    let skip_authorization = if trusted && is_admin {
        req.skip_authorization.unwrap_or(false)
    } else {
        false
    };

    let redirect_uris_json = serde_json::to_value(&req.redirect_uris)
        .map_err(|e| AppError::InternalError(format!("JSON serialization error: {}", e)))?;
    let scopes_json = serde_json::to_value(&scopes)
        .map_err(|e| AppError::InternalError(format!("JSON serialization error: {}", e)))?;

    let app = sqlx::query_as::<_, OAuthApplication>(
        r#"
        INSERT INTO oauth_applications 
            (owner_id, name, uid, secret_hash, redirect_uris, scopes, 
             description, homepage_url, confidential, trusted, skip_authorization, 
             created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
        RETURNING *
        "#
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(&redirect_uris_json)
    .bind(&scopes_json)
    .bind(&req.description)
    .bind(&req.homepage_url)
    .bind(confidential)
    .bind(trusted)
    .bind(skip_authorization)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(OAuthApplicationWithSecret {
        id: app.id,
        name: app.name,
        uid: app.uid,
        secret: client_secret,
        redirect_uris: serde_json::from_value(app.redirect_uris).unwrap_or_default(),
        scopes: serde_json::from_value(app.scopes).unwrap_or_default(),
        created_at: app.created_at,
    }))
}

/// GET /api/v1/oauth/applications/{id} - Get an OAuth application
pub async fn get_application(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let app_id = path.into_inner();

    let app = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE id = $1 AND owner_id = $2"
    )
    .bind(app_id)
    .bind(auth.user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Application not found".to_string()))?;

    Ok(HttpResponse::Ok().json(OAuthApplicationInfo::from(app)))
}

/// PUT /api/v1/oauth/applications/{id} - Update an OAuth application
pub async fn update_application(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
    body: web::Json<UpdateOAuthApplicationRequest>,
) -> AppResult<HttpResponse> {
    let app_id = path.into_inner();
    let req = body.into_inner();

    // Validate redirect URIs if provided
    if let Some(ref uris) = req.redirect_uris {
        for uri in uris {
            if !uri.starts_with("http://") && !uri.starts_with("https://") && !uri.starts_with("urn:") {
                return Err(AppError::BadRequest(format!("Invalid redirect URI: {}", uri)));
            }
        }
    }

    // Convert Vec<String> to JSON for JSONB columns
    let redirect_uris_json = req.redirect_uris.as_ref().map(|uris| {
        serde_json::to_value(uris)
            .map_err(|e| AppError::InternalError(format!("JSON serialization error: {}", e)))
    }).transpose()?;
    
    let scopes_json = req.scopes.as_ref().map(|scopes| {
        serde_json::to_value(scopes)
            .map_err(|e| AppError::InternalError(format!("JSON serialization error: {}", e)))
    }).transpose()?;

    let app = sqlx::query_as::<_, OAuthApplication>(
        r#"
        UPDATE oauth_applications SET
            name = COALESCE($3, name),
            redirect_uris = COALESCE($4, redirect_uris),
            scopes = COALESCE($5, scopes),
            description = COALESCE($6, description),
            homepage_url = COALESCE($7, homepage_url),
            updated_at = NOW()
        WHERE id = $1 AND owner_id = $2
        RETURNING *
        "#
    )
    .bind(app_id)
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&redirect_uris_json)
    .bind(&scopes_json)
    .bind(&req.description)
    .bind(&req.homepage_url)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Application not found".to_string()))?;

    Ok(HttpResponse::Ok().json(OAuthApplicationInfo::from(app)))
}

/// DELETE /api/v1/oauth/applications/{id} - Delete an OAuth application
pub async fn delete_application(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let app_id = path.into_inner();

    let result = sqlx::query("DELETE FROM oauth_applications WHERE id = $1 AND owner_id = $2")
        .bind(app_id)
        .bind(auth.user_id)
        .execute(pool.get_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Application not found".to_string()));
    }

    Ok(HttpResponse::NoContent().finish())
}

/// POST /api/v1/oauth/applications/{id}/regenerate_secret - Regenerate client secret
pub async fn regenerate_secret(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let app_id = path.into_inner();

    let client_secret = generate_client_secret();
    let client_secret_hash = hash_secret(&client_secret);

    let app = sqlx::query_as::<_, OAuthApplication>(
        r#"
        UPDATE oauth_applications 
        SET secret_hash = $3, updated_at = NOW()
        WHERE id = $1 AND owner_id = $2
        RETURNING *
        "#
    )
    .bind(app_id)
    .bind(auth.user_id)
    .bind(&client_secret_hash)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Application not found".to_string()))?;

    Ok(HttpResponse::Ok().json(OAuthApplicationWithSecret {
        id: app.id,
        name: app.name,
        uid: app.uid,
        secret: client_secret,
        redirect_uris: serde_json::from_value(app.redirect_uris).unwrap_or_default(),
        scopes: serde_json::from_value(app.scopes).unwrap_or_default(),
        created_at: app.created_at,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Flow Endpoints (Standard OAuth2 endpoints)
// ─────────────────────────────────────────────────────────────────────────────

/// GET /oauth/authorize - OAuth authorization endpoint
/// This returns HTML for user consent (or redirects if already authorized)
pub async fn authorize(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    auth: crate::middleware::OptionalAuth,
    query: web::Query<OAuthAuthorizeRequest>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    let oauth_req = query.into_inner();

    // Check if user is authenticated
    let user = match auth.user() {
        Some(u) => u,
        None => {
            // User not logged in, redirect to login page with OAuth parameters preserved
            // Encode the full OAuth request URL for redirect after login
            let original_url = req.uri().to_string();
            let encoded_redirect = urlencoding::encode(&original_url);
            
            // Determine base URL for redirect
            let base_url = config.base_url.trim_end_matches('/');
            let login_url = format!("{}/login?redirect={}", base_url, encoded_redirect);
            
            return Ok(HttpResponse::Found()
                .insert_header(("Location", login_url))
                .finish());
        }
    };

    // Validate response_type
    if oauth_req.response_type != "code" {
        return Err(AppError::BadRequest("Unsupported response_type".to_string()));
    }

    // Find application
    let app = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE uid = $1"
    )
    .bind(&oauth_req.client_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid client_id".to_string()))?;

    // Convert JSONB to Vec<String>
    let redirect_uris: Vec<String> = serde_json::from_value(app.redirect_uris.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to parse redirect_uris in authorize: {}", e)))?;
    let app_scopes: Vec<String> = serde_json::from_value(app.scopes.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to parse scopes in authorize: {}", e)))?;

    // Validate redirect_uri
    if !redirect_uris.contains(&oauth_req.redirect_uri) {
        return Err(AppError::BadRequest("Invalid redirect_uri".to_string()));
    }

    // Parse requested scopes
    let requested_scopes: Vec<String> = oauth_req.scope
        .unwrap_or_default()
        .split_whitespace()
        .map(String::from)
        .collect();

    // Validate scopes against app's allowed scopes
    for scope in &requested_scopes {
        if !app_scopes.contains(scope) {
            return Err(AppError::BadRequest(format!("Scope '{}' not allowed for this application", scope)));
        }
    }

    // For apps with skip_authorization enabled, skip consent and issue code directly
    if app.skip_authorization {
        return issue_authorization_code(
            pool.get_ref(),
            &app,
            user.user_id,
            &oauth_req.redirect_uri,
            &requested_scopes,
            oauth_req.code_challenge,
            oauth_req.code_challenge_method,
            oauth_req.state,
        ).await;
    }

    // For apps requiring authorization, return info for consent screen
    // In a real implementation, this would render an HTML consent page
    // For API purposes, we return JSON that the frontend can use
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "application": {
            "name": app.name,
            "description": app.description,
            "homepage_url": app.homepage_url,
            "logo_url": app.logo_url,
        },
        "requested_scopes": requested_scopes,
        "redirect_uri": oauth_req.redirect_uri,
        "state": oauth_req.state,
    })))
}

/// GET /api/v1/oauth/authorize/info - Get OAuth authorization info (for frontend)
/// This endpoint requires authentication and returns authorization info for the consent screen
pub async fn authorize_info(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    query: web::Query<OAuthAuthorizeRequest>,
) -> AppResult<HttpResponse> {
    let req = query.into_inner();

    // Validate response_type
    if req.response_type != "code" {
        return Err(AppError::BadRequest("Unsupported response_type".to_string()));
    }

    // Find application
    let app = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE uid = $1"
    )
    .bind(&req.client_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid client_id".to_string()))?;

    // Convert JSONB to Vec<String>
    let redirect_uris: Vec<String> = serde_json::from_value(app.redirect_uris.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to parse redirect_uris: {}", e)))?;
    let app_scopes: Vec<String> = serde_json::from_value(app.scopes.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to parse scopes: {}", e)))?;

    // Validate redirect_uri
    if !redirect_uris.contains(&req.redirect_uri) {
        return Err(AppError::BadRequest("Invalid redirect_uri".to_string()));
    }

    // Parse requested scopes
    let requested_scopes: Vec<String> = req.scope
        .clone()
        .unwrap_or_default()
        .split_whitespace()
        .map(String::from)
        .collect();
    
    // If no scopes requested, use app's default scopes
    let requested_scopes = if requested_scopes.is_empty() {
        app_scopes.clone()
    } else {
        requested_scopes
    };

    // Validate scopes against app's allowed scopes
    for scope in &requested_scopes {
        if !app_scopes.contains(scope) {
            return Err(AppError::BadRequest(format!("Scope '{}' not allowed for this application", scope)));
        }
    }

    // Return info for consent screen (including whether it's a trusted app)
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "application": {
            "name": app.name,
            "description": app.description,
            "homepage_url": app.homepage_url,
            "logo_url": app.logo_url,
        },
        "requested_scopes": requested_scopes,
        "redirect_uri": req.redirect_uri,
        "state": req.state,
        "trusted": app.trusted,
        "user": {
            "id": auth.user_id,
            "username": auth.username,
        }
    })))
}

/// POST /api/v1/oauth/authorize/confirm - Confirm OAuth authorization (for frontend)
pub async fn authorize_confirm(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    body: web::Json<OAuthAuthorizeRequest>,
) -> AppResult<HttpResponse> {
    let req = body.into_inner();

    let app = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE uid = $1"
    )
    .bind(&req.client_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid client_id".to_string()))?;

    let redirect_uris: Vec<String> = serde_json::from_value(app.redirect_uris.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to parse redirect_uris in authorize_confirm: {}", e)))?;
    if !redirect_uris.contains(&req.redirect_uri) {
        return Err(AppError::BadRequest("Invalid redirect_uri".to_string()));
    }

    let app_scopes: Vec<String> = serde_json::from_value(app.scopes.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to parse scopes in authorize_confirm: {}", e)))?;

    log::debug!("authorize_confirm: requested scope string: {:?}", req.scope);
    
    let scopes: Vec<String> = req.scope
        .unwrap_or_default()
        .split_whitespace()
        .map(String::from)
        .collect();
    
    // If no scopes requested, use app's default scopes
    let scopes = if scopes.is_empty() {
        log::debug!("authorize_confirm: no scopes requested, using app defaults: {:?}", app_scopes);
        app_scopes
    } else {
        scopes
    };
    
    log::debug!("authorize_confirm: final scopes to issue: {:?}", scopes);
    
    log::debug!("authorize_confirm: final scopes: {:?}", scopes);

    issue_authorization_code(
        pool.get_ref(),
        &app,
        auth.user_id,
        &req.redirect_uri,
        &scopes,
        req.code_challenge,
        req.code_challenge_method,
        req.state,
    ).await
}

/// POST /oauth/authorize - User grants authorization
pub async fn authorize_grant(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    body: web::Json<OAuthAuthorizeRequest>,
) -> AppResult<HttpResponse> {
    let req = body.into_inner();

    let app = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE uid = $1"
    )
    .bind(&req.client_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid client_id".to_string()))?;

    let redirect_uris: Vec<String> = serde_json::from_value(app.redirect_uris.clone())
        .map_err(|e| AppError::InternalError(format!("Failed to parse redirect_uris in authorize_grant: {}", e)))?;
    if !redirect_uris.contains(&req.redirect_uri) {
        return Err(AppError::BadRequest("Invalid redirect_uri".to_string()));
    }

    let scopes: Vec<String> = req.scope
        .unwrap_or_default()
        .split_whitespace()
        .map(String::from)
        .collect();

    issue_authorization_code(
        pool.get_ref(),
        &app,
        auth.user_id,
        &req.redirect_uri,
        &scopes,
        req.code_challenge,
        req.code_challenge_method,
        req.state,
    ).await
}

async fn issue_authorization_code(
    pool: &PgPool,
    app: &OAuthApplication,
    user_id: i64,
    redirect_uri: &str,
    scopes: &[String],
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    state: Option<String>,
) -> AppResult<HttpResponse> {
    let code = generate_auth_code();
    let code_hash = hash_secret(&code);
    let expires_at = Utc::now() + Duration::minutes(10);

    // Convert scopes to JSONB
    let scopes_json = serde_json::to_value(scopes)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize scopes: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO oauth_authorization_codes 
            (application_id, user_id, code_hash, redirect_uri, scopes, 
             code_challenge, code_challenge_method, expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
        "#
    )
    .bind(app.id)
    .bind(user_id)
    .bind(&code_hash)
    .bind(redirect_uri)
    .bind(scopes_json)
    .bind(&code_challenge)
    .bind(&code_challenge_method)
    .bind(expires_at)
    .execute(pool)
    .await?;

    // Build redirect URL
    let mut redirect_url = format!("{}?code={}", redirect_uri, code);
    if let Some(s) = state {
        redirect_url.push_str(&format!("&state={}", s));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "redirect_uri": redirect_url,
        "code": code
    })))
}

/// POST /oauth/token - OAuth token endpoint
pub async fn token(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    req: HttpRequest,
    body: web::Form<OAuthTokenRequest>,
) -> AppResult<HttpResponse> {
    let token_req = body.into_inner();

    match token_req.grant_type.as_str() {
        "authorization_code" => {
            exchange_authorization_code(pool.get_ref(), config.get_ref(), &req, token_req).await
        }
        "refresh_token" => {
            refresh_access_token(pool.get_ref(), config.get_ref(), &req, token_req).await
        }
        _ => Err(AppError::BadRequest("Unsupported grant_type".to_string())),
    }
}

async fn exchange_authorization_code(
    pool: &PgPool,
    config: &AppConfig,
    req: &HttpRequest,
    token_req: OAuthTokenRequest,
) -> AppResult<HttpResponse> {
    let code = token_req.code.clone()
        .ok_or_else(|| AppError::BadRequest("code is required".to_string()))?;
    let redirect_uri = token_req.redirect_uri.clone()
        .ok_or_else(|| AppError::BadRequest("redirect_uri is required".to_string()))?;
    let code_verifier = token_req.code_verifier.clone();

    // Get client credentials (from body or Basic auth)
    let (client_id, client_secret) = get_client_credentials(req, &token_req)?;

    // Find and validate application
    let app = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE uid = $1"
    )
    .bind(&client_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid client_id".to_string()))?;

    // Verify client secret for confidential clients
    if app.confidential {
        let secret = client_secret
            .ok_or_else(|| AppError::Unauthorized("client_secret is required".to_string()))?;
        if hash_secret(&secret) != app.secret_hash {
            return Err(AppError::Unauthorized("Invalid client_secret".to_string()));
        }
    }

    // Find authorization code
    let code_hash = hash_secret(&code);
    let auth_code = sqlx::query_as::<_, (i64, i64, String, serde_json::Value, Option<String>, Option<String>)>(
        r#"
        SELECT user_id, application_id, redirect_uri, scopes, code_challenge, code_challenge_method
        FROM oauth_authorization_codes
        WHERE code_hash = $1 AND application_id = $2 AND used_at IS NULL AND expires_at > NOW()
        "#
    )
    .bind(&code_hash)
    .bind(app.id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("Invalid or expired authorization code".to_string()))?;

    let (user_id, _, stored_redirect_uri, scopes_value, code_challenge, code_challenge_method) = auth_code;
    
    // Deserialize scopes from JSONB
    let scopes: Vec<String> = serde_json::from_value(scopes_value)
        .map_err(|e| AppError::InternalError(format!("Failed to deserialize scopes: {}", e)))?;

    // Validate redirect_uri matches
    if stored_redirect_uri != redirect_uri {
        return Err(AppError::BadRequest("redirect_uri mismatch".to_string()));
    }

    // Validate PKCE if code_challenge was provided
    if let Some(challenge) = code_challenge {
        let verifier = code_verifier
            .ok_or_else(|| AppError::BadRequest("code_verifier is required".to_string()))?;
        let method = code_challenge_method.unwrap_or_else(|| "plain".to_string());
        if !verify_pkce(&verifier, &challenge, &method) {
            return Err(AppError::BadRequest("Invalid code_verifier".to_string()));
        }
    }

    // Mark code as used
    sqlx::query("UPDATE oauth_authorization_codes SET used_at = NOW() WHERE code_hash = $1")
        .bind(&code_hash)
        .execute(pool)
        .await?;

    // Generate tokens
    let access_token = generate_access_token();
    let access_token_hash = hash_secret(&access_token);
    let refresh_token = generate_access_token();
    let refresh_token_hash = hash_secret(&refresh_token);
    let expires_at = Utc::now() + Duration::hours(2);

    // Convert scopes to JSONB
    let scopes_json = serde_json::to_value(&scopes)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize scopes: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO oauth_access_tokens 
            (application_id, user_id, token_hash, refresh_token_hash, scopes, 
             expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        "#
    )
    .bind(app.id)
    .bind(user_id)
    .bind(&access_token_hash)
    .bind(&refresh_token_hash)
    .bind(scopes_json)
    .bind(expires_at)
    .execute(pool)
    .await?;

    // Get user info for id_token
    let user = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT username, email FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let (username, email) = user;

    // Generate OIDC ID Token if 'openid' scope is present
    let id_token = generate_id_token(
        config,
        user_id,
        &username,
        email.as_deref(),
        &app.uid,
        &scopes,
    )?;

    Ok(HttpResponse::Ok().json(OAuthTokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: Some(7200), // 2 hours
        refresh_token: Some(refresh_token),
        scope: scopes.join(" "),
        id_token,
        created_at: Some(Utc::now().timestamp()),
    }))
}

async fn refresh_access_token(
    pool: &PgPool,
    config: &AppConfig,
    req: &HttpRequest,
    token_req: OAuthTokenRequest,
) -> AppResult<HttpResponse> {
    let refresh_token = token_req.refresh_token.clone()
        .ok_or_else(|| AppError::BadRequest("refresh_token is required".to_string()))?;

    let (client_id, client_secret) = get_client_credentials(req, &token_req)?;

    // Find application
    let app = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE uid = $1"
    )
    .bind(&client_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid client_id".to_string()))?;

    // Verify client secret
    if app.confidential {
        let secret = client_secret
            .ok_or_else(|| AppError::Unauthorized("client_secret is required".to_string()))?;
        if hash_secret(&secret) != app.secret_hash {
            return Err(AppError::Unauthorized("Invalid client_secret".to_string()));
        }
    }

    // Find and validate refresh token
    let refresh_token_hash = hash_secret(&refresh_token);
    let token_info = sqlx::query_as::<_, (i64, i64, serde_json::Value)>(
        r#"
        SELECT id, user_id, scopes FROM oauth_access_tokens
        WHERE refresh_token_hash = $1 AND application_id = $2 
          AND revoked_at IS NULL
        "#
    )
    .bind(&refresh_token_hash)
    .bind(app.id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid or expired refresh token".to_string()))?;

    let (old_token_id, user_id, scopes_value) = token_info;
    
    // Deserialize scopes from JSONB
    let scopes: Vec<String> = serde_json::from_value(scopes_value)
        .map_err(|e| AppError::InternalError(format!("Failed to deserialize scopes: {}", e)))?;

    // Revoke old token
    sqlx::query("UPDATE oauth_access_tokens SET revoked_at = NOW() WHERE id = $1")
        .bind(old_token_id)
        .execute(pool)
        .await?;

    // Issue new tokens
    let new_access_token = generate_access_token();
    let new_access_token_hash = hash_secret(&new_access_token);
    let new_refresh_token = generate_access_token();
    let new_refresh_token_hash = hash_secret(&new_refresh_token);
    let expires_at = Utc::now() + Duration::hours(2);

    // Convert scopes to JSONB
    let scopes_json = serde_json::to_value(&scopes)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize scopes: {}", e)))?;

    sqlx::query(
        r#"
        INSERT INTO oauth_access_tokens 
            (application_id, user_id, token_hash, refresh_token_hash, scopes, 
             expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        "#
    )
    .bind(app.id)
    .bind(user_id)
    .bind(&new_access_token_hash)
    .bind(&new_refresh_token_hash)
    .bind(scopes_json)
    .bind(expires_at)
    .bind(expires_at)
    .execute(pool)
    .await?;

    // Get user info for id_token
    let user = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT username, email FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    let (username, email) = user;

    // Generate OIDC ID Token if 'openid' scope is present
    let id_token = generate_id_token(
        config,
        user_id,
        &username,
        email.as_deref(),
        &app.uid,
        &scopes,
    )?;

    Ok(HttpResponse::Ok().json(OAuthTokenResponse {
        access_token: new_access_token,
        token_type: "Bearer".to_string(),
        expires_in: Some(7200),
        refresh_token: Some(new_refresh_token),
        scope: scopes.join(" "),
        id_token,
        created_at: Some(Utc::now().timestamp()),
    }))
}

fn get_client_credentials(req: &HttpRequest, token_req: &OAuthTokenRequest) -> AppResult<(String, Option<String>)> {
    // Try Basic auth first
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Basic ") {
                use base64::{engine::general_purpose::STANDARD, Engine};
                if let Ok(decoded) = STANDARD.decode(&auth_str[6..]) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        if let Some((id, secret)) = credentials.split_once(':') {
                            return Ok((id.to_string(), Some(secret.to_string())));
                        }
                    }
                }
            }
        }
    }

    // Fall back to body params
    let client_id = token_req.client_id.clone()
        .ok_or_else(|| AppError::BadRequest("client_id is required".to_string()))?;
    
    Ok((client_id, token_req.client_secret.clone()))
}

// ─────────────────────────────────────────────────────────────────────────────
// RFC 7009 Token Revocation
// ─────────────────────────────────────────────────────────────────────────────

/// POST /oauth/revoke - RFC 7009 Token Revocation Endpoint
pub async fn revoke(
    pool: web::Data<PgPool>,
    req: HttpRequest,
    body: web::Form<OAuthTokenRevocationRequest>,
) -> AppResult<HttpResponse> {
    let revoke_req = body.into_inner();

    // Get client credentials (from body or Basic auth)
    let (client_id, client_secret) = get_client_credentials_from_revoke_req(&req, &revoke_req)?;

    // Find and validate application
    let app = sqlx::query_as::<_, OAuthApplication>(
        "SELECT * FROM oauth_applications WHERE uid = $1"
    )
    .bind(&client_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid client_id".to_string()))?;

    // Verify client secret for confidential clients
    if app.confidential {
        let secret = client_secret
            .ok_or_else(|| AppError::Unauthorized("client_secret is required".to_string()))?;
        if hash_secret(&secret) != app.secret_hash {
            return Err(AppError::Unauthorized("Invalid client_secret".to_string()));
        }
    }

    let token_hash = hash_secret(&revoke_req.token);

    // Try to revoke the token (could be access_token or refresh_token)
    // RFC 7009 states that the endpoint should return success even if token doesn't exist
    let hint = revoke_req.token_type_hint.as_deref();
    
    if hint.is_none() || hint == Some("access_token") {
        // Try as access token
        let _ = sqlx::query(
            "UPDATE oauth_access_tokens SET revoked_at = NOW() WHERE token_hash = $1 AND application_id = $2 AND revoked_at IS NULL"
        )
        .bind(&token_hash)
        .bind(app.id)
        .execute(pool.get_ref())
        .await?;
    }
    
    if hint.is_none() || hint == Some("refresh_token") {
        // Try as refresh token
        let _ = sqlx::query(
            "UPDATE oauth_access_tokens SET revoked_at = NOW() WHERE refresh_token_hash = $1 AND application_id = $2 AND revoked_at IS NULL"
        )
        .bind(&token_hash)
        .bind(app.id)
        .execute(pool.get_ref())
        .await?;
    }

    // RFC 7009 mandates that we return 200 OK regardless of whether the token existed
    Ok(HttpResponse::Ok().finish())
}

fn get_client_credentials_from_revoke_req(
    req: &HttpRequest,
    revoke_req: &OAuthTokenRevocationRequest,
) -> AppResult<(String, Option<String>)> {
    // Try Basic auth first
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Basic ") {
                use base64::{engine::general_purpose::STANDARD, Engine};
                if let Ok(decoded) = STANDARD.decode(&auth_str[6..]) {
                    if let Ok(credentials) = String::from_utf8(decoded) {
                        if let Some((id, secret)) = credentials.split_once(':') {
                            return Ok((id.to_string(), Some(secret.to_string())));
                        }
                    }
                }
            }
        }
    }

    // Fall back to body params
    let client_id = revoke_req.client_id.clone()
        .ok_or_else(|| AppError::BadRequest("client_id is required".to_string()))?;
    
    Ok((client_id, revoke_req.client_secret.clone()))
}

// ─────────────────────────────────────────────────────────────────────────────
// OIDC UserInfo Endpoint
// ─────────────────────────────────────────────────────────────────────────────

/// GET /oauth/userinfo - OIDC UserInfo Endpoint
pub async fn userinfo(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    req: HttpRequest,
) -> AppResult<HttpResponse> {
    // Extract access token from Authorization header
    let auth_header = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized("Invalid Authorization header".to_string()));
    }

    let access_token = &auth_header[7..];
    let token_hash = hash_secret(access_token);

    // Find and validate access token
    let token_info = sqlx::query_as::<_, (i64, i64, serde_json::Value)>(
        r#"
        SELECT user_id, application_id, scopes FROM oauth_access_tokens
        WHERE token_hash = $1 AND revoked_at IS NULL AND expires_at > NOW()
        "#
    )
    .bind(&token_hash)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid or expired access token".to_string()))?;

    let (user_id, _app_id, scopes_json) = token_info;
    let scopes: Vec<String> = serde_json::from_value(scopes_json.clone()).unwrap_or_default();
    
    log::debug!("userinfo: scopes_json from DB: {:?}", scopes_json);
    log::debug!("userinfo: parsed scopes: {:?}", scopes);

    // Check if user info scope is present (openid for OIDC, or read_user for OAuth 2.0)
    let has_user_scope = scopes.iter().any(|s| s == "openid" || s == "read_user");
    if !has_user_scope {
        return Err(AppError::Forbidden("Insufficient scope (openid or read_user required)".to_string()));
    }

    // Get user info
    let user = sqlx::query_as::<_, (String, Option<String>, Option<String>, bool, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        "SELECT username, display_name, email, is_active, created_at, updated_at FROM users WHERE id = $1"
    )
    .bind(user_id)
    .fetch_one(pool.get_ref())
    .await?;

    let (username, display_name, email, _is_active, created_at, updated_at) = user;

    // Build profile URL
    let profile_url = format!("{}/{}", config.base_url, username);

    let response = OAuthUserInfoResponse {
        sub: user_id.to_string(),
        preferred_username: username,
        name: display_name,
        // Return email if user has openid/read_user scope (email scope is for extra validation)
        email: if has_user_scope { email.clone() } else { None },
        email_verified: email.is_some(),
        picture: None, // TODO: Add avatar support
        profile: Some(profile_url),
        created_at: Some(created_at.timestamp()),
        updated_at: Some(updated_at.timestamp()),
    };

    Ok(HttpResponse::Ok().json(response))
}

// ─────────────────────────────────────────────────────────────────────────────
// OAuth Providers (GitFox as OAuth Client - social login)
// ─────────────────────────────────────────────────────────────────────────────

/// 常用提供商列表，这些提供商的凭证从 .env 读取
const BUILTIN_PROVIDERS: &[&str] = &["github", "gitlab", "google", "azure_ad", "bitbucket"];

/// 检查是否是内置提供商（凭证从 .env 读取）
fn is_builtin_provider(name: &str) -> bool {
    BUILTIN_PROVIDERS.contains(&name)
}

/// 从 .env 获取内置提供商的凭证
fn get_builtin_credentials(config: &AppConfig, provider_name: &str) -> Option<(String, String)> {
    match provider_name {
        "github" => config.oauth.github.client_id.clone()
            .zip(config.oauth.github.client_secret.clone()),
        "gitlab" => config.oauth.gitlab.client_id.clone()
            .zip(config.oauth.gitlab.client_secret.clone()),
        "google" => config.oauth.google.client_id.clone()
            .zip(config.oauth.google.client_secret.clone()),
        "azure_ad" | "azure" => config.oauth.azure_ad.client_id.clone()
            .zip(config.oauth.azure_ad.client_secret.clone()),
        "bitbucket" => config.oauth.bitbucket.client_id.clone()
            .zip(config.oauth.bitbucket.client_secret.clone()),
        _ => None,
    }
}

/// GET /api/v1/oauth/providers - List enabled OAuth providers for login
/// 所有配置从数据库读取，内置提供商需要 .env 中配置了凭证才显示
pub async fn list_providers(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
) -> AppResult<HttpResponse> {
    let db_providers = sqlx::query_as::<_, OAuthProviderRecord>(
        "SELECT * FROM oauth_providers WHERE enabled = true ORDER BY sort_order, name"
    )
    .fetch_all(pool.get_ref())
    .await?;

    let mut providers = Vec::new();
    for provider in db_providers {
        // 内置提供商需要 .env 配置了凭证才显示
        if is_builtin_provider(&provider.name) {
            if get_builtin_credentials(&config, &provider.name).is_none() {
                continue; // .env 没配置凭证，跳过
            }
        }
        // 自定义提供商需要数据库里有凭证
        else if provider.client_id.is_empty() || provider.client_secret_encrypted.is_empty() {
            continue;
        }

        providers.push(OAuthProviderInfo {
            name: provider.name.clone(),
            display_name: provider.display_name,
            provider_type: provider.provider_type,
            icon: provider.icon,
            authorize_url: format!("{}/api/v1/oauth/{}/authorize", config.base_url, provider.name),
        });
    }

    Ok(HttpResponse::Ok().json(OAuthProvidersResponse { providers }))
}

/// GET /api/v1/oauth/{provider}/authorize - Redirect to OAuth provider
/// 通用逻辑：从数据库读 URL 配置，凭证根据是否内置提供商决定来源
pub async fn provider_authorize(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> AppResult<HttpResponse> {
    let provider_name = path.into_inner();
    let state = query.get("state").cloned().unwrap_or_else(|| {
        let mut rng = rand::thread_rng();
        hex::encode(rng.gen::<[u8; 16]>())
    });

    // 从数据库读取 provider 配置
    let provider = sqlx::query_as::<_, OAuthProviderRecord>(
        "SELECT * FROM oauth_providers WHERE name = $1 AND enabled = true"
    )
    .bind(&provider_name)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("OAuth provider '{}' not found or disabled", provider_name)))?;

    // 获取凭证：内置提供商从 .env，其他从数据库
    let client_id = if is_builtin_provider(&provider_name) {
        get_builtin_credentials(&config, &provider_name)
            .map(|(id, _)| id)
            .ok_or_else(|| AppError::BadRequest(format!("{} OAuth not configured in .env", provider_name)))?
    } else {
        if provider.client_id.is_empty() {
            return Err(AppError::BadRequest(format!("{} OAuth client_id not configured", provider_name)));
        }
        provider.client_id.clone()
    };

    let redirect_uri = format!("{}/api/v1/oauth/{}/callback", config.base_url, provider_name);
    
    // 构建 scopes 参数
    let scopes: Vec<String> = serde_json::from_value(provider.scopes.clone()).unwrap_or_default();
    let scope_str = scopes.join(" ");

    // 通用的授权 URL 构建
    let authorize_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        provider.authorization_url,
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&scope_str),
        urlencoding::encode(&state)
    );

    Ok(HttpResponse::Found()
        .insert_header(("Location", authorize_url))
        .finish())
}

/// GET /api/v1/oauth/{provider}/callback - OAuth callback handler
/// 通用逻辑：从数据库读 URL 配置，凭证根据是否内置提供商决定来源
pub async fn provider_callback(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> AppResult<HttpResponse> {
    let provider_name = path.into_inner();
    let code = query.get("code")
        .ok_or_else(|| AppError::BadRequest("Missing authorization code".to_string()))?;
    let _state = query.get("state");

    // Handle error from OAuth provider
    if let Some(error) = query.get("error") {
        let error_desc = query.get("error_description").map(|s| s.as_str()).unwrap_or("");
        return Err(AppError::BadRequest(format!("OAuth error: {} - {}", error, error_desc)));
    }

    // 从数据库读取 provider 配置（必须存在）
    let provider = sqlx::query_as::<_, OAuthProviderRecord>(
        "SELECT * FROM oauth_providers WHERE name = $1 AND enabled = true"
    )
    .bind(&provider_name)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("OAuth provider '{}' not found or disabled", provider_name)))?;

    let redirect_uri = format!("{}/api/v1/oauth/{}/callback", config.base_url, provider_name);
    
    // 交换 code 获取用户信息（通用逻辑）
    let (external_uid, external_username, external_email, external_avatar) = 
        exchange_code_for_user_info(&provider, &config, code, &redirect_uri).await?;

    let provider_id = provider.id;

    // Find existing identity or user
    let existing_identity = sqlx::query_as::<_, (i64, i64)>(
        "SELECT id, user_id FROM oauth_identities WHERE provider_id = $1 AND external_uid = $2"
    )
    .bind(provider_id)
    .bind(&external_uid)
    .fetch_optional(pool.get_ref())
    .await?;

    let user_id = if let Some((identity_id, uid)) = existing_identity {
        // Update existing identity with latest info
        sqlx::query(
            r#"
            UPDATE oauth_identities SET 
                external_username = COALESCE($2, external_username),
                external_email = COALESCE($3, external_email),
                external_avatar_url = COALESCE($4, external_avatar_url),
                last_sign_in_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(identity_id)
        .bind(&external_username)
        .bind(&external_email)
        .bind(&external_avatar)
        .execute(pool.get_ref())
        .await?;
        
        uid
    } else {
        // Try to find user by email
        let existing_user = if let Some(ref email) = external_email {
            sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(pool.get_ref())
                .await?
        } else {
            None
        };

        let uid = if let Some(user_id) = existing_user {
            user_id
        } else {
            // Create new user
            let username = generate_unique_username(pool.get_ref(), &external_username, &external_uid).await?;
            let display_name = external_username.clone().unwrap_or_else(|| username.clone());
            
            sqlx::query_scalar::<_, i64>(
                r#"
                INSERT INTO users (username, display_name, email, password_hash, role, is_active, created_at, updated_at)
                VALUES ($1, $2, $3, '', 'developer', true, NOW(), NOW())
                RETURNING id
                "#
            )
            .bind(&username)
            .bind(&display_name)
            .bind(&external_email)
            .fetch_one(pool.get_ref())
            .await?
        };

        // Create identity link
        sqlx::query(
            r#"
            INSERT INTO oauth_identities (user_id, provider_id, external_uid, external_username, external_email, external_avatar_url, last_sign_in_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            "#
        )
        .bind(uid)
        .bind(provider_id)
        .bind(&external_uid)
        .bind(&external_username)
        .bind(&external_email)
        .bind(&external_avatar)
        .execute(pool.get_ref())
        .await?;

        uid
    };

    // Get user info for JWT
    let user = sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool.get_ref())
        .await?;

    // Generate JWT
    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::seconds(config.jwt_expiration);

    let claims = crate::models::Claims {
        sub: user.username.clone(),
        user_id: user.id,
        role: user.role.clone(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;

    // Redirect to frontend with token
    let frontend_url = format!(
        "{}/oauth/callback?token={}&provider={}",
        config.base_url.trim_end_matches("/api/v1"),
        urlencoding::encode(&token),
        urlencoding::encode(&provider_name)
    );

    Ok(HttpResponse::Found()
        .insert_header(("Location", frontend_url))
        .finish())
}

/// 通用的 OAuth code 交换逻辑
/// - 内置提供商（github/gitlab/google/azure_ad/bitbucket）：凭证从 .env 读取
/// - 其他自定义提供商：凭证从数据库读取
/// - URL 配置和字段映射都从数据库读取
async fn exchange_code_for_user_info(
    provider: &OAuthProviderRecord,
    config: &AppConfig,
    code: &str,
    redirect_uri: &str,
) -> AppResult<(String, Option<String>, Option<String>, Option<String>)> {
    let client = reqwest::Client::new();
    
    // 获取凭证：内置提供商从 .env，其他从数据库
    let (client_id, client_secret) = if is_builtin_provider(&provider.name) {
        get_builtin_credentials(config, &provider.name)
            .ok_or_else(|| AppError::BadRequest(format!("{} OAuth not configured in .env", provider.name)))?
    } else {
        if provider.client_id.is_empty() || provider.client_secret_encrypted.is_empty() {
            return Err(AppError::BadRequest(format!("{} OAuth credentials not configured", provider.name)));
        }
        // TODO: 生产环境应解密 client_secret_encrypted
        (provider.client_id.clone(), provider.client_secret_encrypted.clone())
    };

    // 交换 code 获取 access_token
    let token_response = client.post(&provider.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
        ])
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to exchange code: {}", e)))?;

    if !token_response.status().is_success() {
        let error_text = token_response.text().await.unwrap_or_default();
        return Err(AppError::BadRequest(format!("Token exchange failed: {}", error_text)));
    }

    let token_data: serde_json::Value = token_response.json().await
        .map_err(|e| AppError::InternalError(format!("Failed to parse token response: {}", e)))?;
    
    // 检查错误响应
    if let Some(error) = token_data["error"].as_str() {
        let error_desc = token_data["error_description"].as_str().unwrap_or("");
        return Err(AppError::BadRequest(format!("OAuth error: {} - {}", error, error_desc)));
    }

    let access_token = token_data["access_token"].as_str()
        .ok_or_else(|| AppError::InternalError("No access_token in response".to_string()))?;

    // 获取用户信息
    let userinfo_url = provider.userinfo_url.as_ref()
        .ok_or_else(|| AppError::InternalError("No userinfo_url configured".to_string()))?;

    let user_response = client.get(userinfo_url)
        .header("User-Agent", "GitFox")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get user info: {}", e)))?;

    let user_data: serde_json::Value = user_response.json().await
        .map_err(|e| AppError::InternalError(format!("Failed to parse user info: {}", e)))?;

    // 使用字段映射提取用户信息
    let field_mappings: std::collections::HashMap<String, String> = 
        serde_json::from_value(provider.field_mappings.clone()).unwrap_or_default();
    
    let id_field = field_mappings.get("id").map(|s| s.as_str()).unwrap_or("id");
    let username_field = field_mappings.get("username").map(|s| s.as_str()).unwrap_or("login");
    let email_field = field_mappings.get("email").map(|s| s.as_str()).unwrap_or("email");
    let avatar_field = field_mappings.get("avatar").map(|s| s.as_str()).unwrap_or("avatar_url");

    // 提取 ID（可能是字符串或数字）
    let external_uid = if let Some(s) = user_data[id_field].as_str() {
        s.to_string()
    } else if let Some(n) = user_data[id_field].as_i64() {
        n.to_string()
    } else {
        user_data[id_field].to_string().trim_matches('"').to_string()
    };
    
    let external_username = user_data[username_field].as_str().map(String::from);
    let external_email = user_data[email_field].as_str().map(String::from);
    let external_avatar = user_data[avatar_field].as_str().map(String::from);

    Ok((external_uid, external_username, external_email, external_avatar))
}

/// Generate a unique username for new OAuth users
async fn generate_unique_username(pool: &PgPool, external_username: &Option<String>, external_uid: &str) -> AppResult<String> {
    let base = external_username.clone()
        .unwrap_or_else(|| format!("user_{}", &external_uid[..external_uid.len().min(8)]));
    
    // Clean the username (lowercase, alphanumeric)
    let clean_base: String = base.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(30)
        .collect();
    
    let clean_base = if clean_base.is_empty() {
        format!("user_{}", &external_uid[..external_uid.len().min(8)])
    } else {
        clean_base
    };

    // Check if username exists, add suffix if needed
    let mut username = clean_base.clone();
    let mut suffix = 1;
    
    loop {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)"
        )
        .bind(&username)
        .fetch_one(pool)
        .await?;

        if !exists {
            break;
        }

        username = format!("{}_{}", clean_base, suffix);
        suffix += 1;

        if suffix > 1000 {
            return Err(AppError::InternalError("Failed to generate unique username".to_string()));
        }
    }

    Ok(username)
}

// ─────────────────────────────────────────────────────────────────────────────
// User's Linked Identities
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/v1/user/identities - List user's linked OAuth identities
pub async fn list_identities(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    // Join with oauth_providers to get provider name and display_name
    let identity_infos = sqlx::query_as::<_, OAuthIdentityInfo>(
        r#"
        SELECT 
            i.id, i.provider_id, p.name as provider_name, p.display_name as provider_display_name,
            i.external_username, i.external_email, i.external_avatar_url, 
            i.last_sign_in_at, i.created_at
        FROM oauth_identities i
        JOIN oauth_providers p ON i.provider_id = p.id
        WHERE i.user_id = $1 
        ORDER BY i.created_at DESC
        "#
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(identity_infos))
}

/// DELETE /api/v1/user/identities/{id} - Unlink an OAuth identity
pub async fn unlink_identity(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let identity_id = path.into_inner();

    // Check if user has password set (can't unlink last identity if no password)
    let has_password = sqlx::query_scalar::<_, bool>(
        "SELECT password_hash IS NOT NULL AND password_hash != '' FROM users WHERE id = $1"
    )
    .bind(auth.user_id)
    .fetch_one(pool.get_ref())
    .await?;

    let identity_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM oauth_identities WHERE user_id = $1"
    )
    .bind(auth.user_id)
    .fetch_one(pool.get_ref())
    .await?;

    if !has_password && identity_count <= 1 {
        return Err(AppError::BadRequest(
            "Cannot unlink last OAuth identity without a password set".to_string()
        ));
    }

    let result = sqlx::query("DELETE FROM oauth_identities WHERE id = $1 AND user_id = $2")
        .bind(identity_id)
        .bind(auth.user_id)
        .execute(pool.get_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Identity not found".to_string()));
    }

    Ok(HttpResponse::NoContent().finish())
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin OAuth Provider Management
// ─────────────────────────────────────────────────────────────────────────────

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateOAuthProviderRequest {
    pub name: String,
    pub display_name: String,
    pub provider_type: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub userinfo_url: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub icon: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOAuthProviderRequest {
    pub display_name: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub authorization_url: Option<String>,
    pub token_url: Option<String>,
    pub userinfo_url: Option<String>,
    pub scopes: Option<Vec<String>>,
    pub icon: Option<String>,
    pub enabled: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct OAuthProviderAdminInfo {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub provider_type: String,
    pub client_id: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub scopes: Vec<String>,
    pub icon: Option<String>,
    pub enabled: bool,
    pub sort_order: i32,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// GET /api/v1/admin/oauth/providers - List all OAuth providers (admin)
pub async fn admin_list_providers(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    // Check admin role
    let role = sqlx::query_scalar::<_, UserRole>("SELECT role FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(pool.get_ref())
        .await?;
    
    if role != UserRole::Admin {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    let providers = sqlx::query_as::<_, OAuthProviderRecord>(
        "SELECT * FROM oauth_providers ORDER BY sort_order, name"
    )
    .fetch_all(pool.get_ref())
    .await?;

    let result: Vec<OAuthProviderAdminInfo> = providers.into_iter().map(|p| {
        OAuthProviderAdminInfo {
            id: p.id,
            name: p.name,
            display_name: p.display_name,
            provider_type: p.provider_type,
            client_id: p.client_id,
            authorization_url: p.authorization_url,
            token_url: p.token_url,
            userinfo_url: p.userinfo_url,
            scopes: serde_json::from_value(p.scopes).unwrap_or_default(),
            icon: p.icon,
            enabled: p.enabled,
            sort_order: p.sort_order,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }).collect();

    Ok(HttpResponse::Ok().json(result))
}

/// POST /api/v1/admin/oauth/providers - Create a new OAuth provider (admin)
pub async fn admin_create_provider(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    body: web::Json<CreateOAuthProviderRequest>,
) -> AppResult<HttpResponse> {
    // Check admin role
    let role = sqlx::query_scalar::<_, UserRole>("SELECT role FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(pool.get_ref())
        .await?;
    
    if role != UserRole::Admin {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    let req = body.into_inner();
    
    // Validate name format (lowercase, alphanumeric with underscores)
    if !req.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(AppError::BadRequest(
            "Provider name must be lowercase alphanumeric with underscores".to_string()
        ));
    }

    // Check for duplicate name
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM oauth_providers WHERE name = $1)"
    )
    .bind(&req.name)
    .fetch_one(pool.get_ref())
    .await?;

    if exists {
        return Err(AppError::BadRequest(format!("Provider '{}' already exists", req.name)));
    }

    // Set default URLs based on provider type
    let (auth_url, tok_url, user_url) = get_default_provider_urls(&req.provider_type, &req);
    
    let scopes_json = serde_json::to_value(&req.scopes.unwrap_or_else(|| 
        get_default_scopes(&req.provider_type)
    )).unwrap_or(serde_json::json!([]));

    // For simplicity, we just hash the secret (in production, use AES-256-GCM encryption)
    let client_secret_encrypted = hash_secret(&req.client_secret);

    let provider = sqlx::query_as::<_, OAuthProviderRecord>(
        r#"
        INSERT INTO oauth_providers 
            (name, display_name, provider_type, client_id, client_secret_encrypted,
             authorization_url, token_url, userinfo_url, scopes, icon, enabled, sort_order,
             created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 
                COALESCE((SELECT MAX(sort_order) FROM oauth_providers), 0) + 1,
                NOW(), NOW())
        RETURNING *
        "#
    )
    .bind(&req.name)
    .bind(&req.display_name)
    .bind(&req.provider_type)
    .bind(&req.client_id)
    .bind(&client_secret_encrypted)
    .bind(&auth_url)
    .bind(&tok_url)
    .bind(&user_url)
    .bind(&scopes_json)
    .bind(&req.icon)
    .bind(req.enabled.unwrap_or(true))
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(OAuthProviderAdminInfo {
        id: provider.id,
        name: provider.name,
        display_name: provider.display_name,
        provider_type: provider.provider_type,
        client_id: provider.client_id,
        authorization_url: provider.authorization_url,
        token_url: provider.token_url,
        userinfo_url: provider.userinfo_url,
        scopes: serde_json::from_value(provider.scopes).unwrap_or_default(),
        icon: provider.icon,
        enabled: provider.enabled,
        sort_order: provider.sort_order,
        created_at: provider.created_at,
        updated_at: provider.updated_at,
    }))
}

/// PUT /api/v1/admin/oauth/providers/{id} - Update an OAuth provider (admin)
pub async fn admin_update_provider(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
    body: web::Json<UpdateOAuthProviderRequest>,
) -> AppResult<HttpResponse> {
    // Check admin role
    let role = sqlx::query_scalar::<_, UserRole>("SELECT role FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(pool.get_ref())
        .await?;
    
    if role != UserRole::Admin {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    let provider_id = path.into_inner();
    let req = body.into_inner();

    // Build update query dynamically based on provided fields
    let mut updates = vec!["updated_at = NOW()".to_string()];
    let mut param_idx = 2;
    
    if req.display_name.is_some() {
        updates.push(format!("display_name = ${}", param_idx));
        param_idx += 1;
    }
    if req.client_id.is_some() {
        updates.push(format!("client_id = ${}", param_idx));
        param_idx += 1;
    }
    if req.client_secret.is_some() {
        updates.push(format!("client_secret_encrypted = ${}", param_idx));
        param_idx += 1;
    }
    if req.authorization_url.is_some() {
        updates.push(format!("authorization_url = ${}", param_idx));
        param_idx += 1;
    }
    if req.token_url.is_some() {
        updates.push(format!("token_url = ${}", param_idx));
        param_idx += 1;
    }
    if req.userinfo_url.is_some() {
        updates.push(format!("userinfo_url = ${}", param_idx));
        param_idx += 1;
    }
    if req.scopes.is_some() {
        updates.push(format!("scopes = ${}", param_idx));
        param_idx += 1;
    }
    if req.icon.is_some() {
        updates.push(format!("icon = ${}", param_idx));
        param_idx += 1;
    }
    if req.enabled.is_some() {
        updates.push(format!("enabled = ${}", param_idx));
        param_idx += 1;
    }
    if req.sort_order.is_some() {
        updates.push(format!("sort_order = ${}", param_idx));
        // param_idx += 1;  // Not needed after last one
    }

    let query = format!(
        "UPDATE oauth_providers SET {} WHERE id = $1 RETURNING *",
        updates.join(", ")
    );

    let mut query_builder = sqlx::query_as::<_, OAuthProviderRecord>(&query)
        .bind(provider_id);

    if let Some(ref v) = req.display_name { query_builder = query_builder.bind(v); }
    if let Some(ref v) = req.client_id { query_builder = query_builder.bind(v); }
    if let Some(ref v) = req.client_secret { query_builder = query_builder.bind(hash_secret(v)); }
    if let Some(ref v) = req.authorization_url { query_builder = query_builder.bind(v); }
    if let Some(ref v) = req.token_url { query_builder = query_builder.bind(v); }
    if let Some(ref v) = req.userinfo_url { query_builder = query_builder.bind(v); }
    if let Some(ref v) = req.scopes { 
        query_builder = query_builder.bind(serde_json::to_value(v).unwrap_or(serde_json::json!([]))); 
    }
    if let Some(ref v) = req.icon { query_builder = query_builder.bind(v); }
    if let Some(v) = req.enabled { query_builder = query_builder.bind(v); }
    if let Some(v) = req.sort_order { query_builder = query_builder.bind(v); }

    let provider = query_builder
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    Ok(HttpResponse::Ok().json(OAuthProviderAdminInfo {
        id: provider.id,
        name: provider.name,
        display_name: provider.display_name,
        provider_type: provider.provider_type,
        client_id: provider.client_id,
        authorization_url: provider.authorization_url,
        token_url: provider.token_url,
        userinfo_url: provider.userinfo_url,
        scopes: serde_json::from_value(provider.scopes).unwrap_or_default(),
        icon: provider.icon,
        enabled: provider.enabled,
        sort_order: provider.sort_order,
        created_at: provider.created_at,
        updated_at: provider.updated_at,
    }))
}

/// DELETE /api/v1/admin/oauth/providers/{id} - Delete an OAuth provider (admin)
pub async fn admin_delete_provider(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    // Check admin role
    let role = sqlx::query_scalar::<_, UserRole>("SELECT role FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(pool.get_ref())
        .await?;
    
    if role != UserRole::Admin {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    let provider_id = path.into_inner();

    // Check if any identities are using this provider
    let identity_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM oauth_identities WHERE provider_id = $1"
    )
    .bind(provider_id)
    .fetch_one(pool.get_ref())
    .await?;

    if identity_count > 0 {
        return Err(AppError::BadRequest(format!(
            "Cannot delete provider: {} user(s) have linked identities. Disable it instead.",
            identity_count
        )));
    }

    let result = sqlx::query("DELETE FROM oauth_providers WHERE id = $1")
        .bind(provider_id)
        .execute(pool.get_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Provider not found".to_string()));
    }

    Ok(HttpResponse::NoContent().finish())
}

/// GET /api/v1/admin/oauth/providers/{id} - Get a single OAuth provider (admin)
pub async fn admin_get_provider(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    // Check admin role
    let role = sqlx::query_scalar::<_, UserRole>("SELECT role FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(pool.get_ref())
        .await?;
    
    if role != UserRole::Admin {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    let provider_id = path.into_inner();

    let provider = sqlx::query_as::<_, OAuthProviderRecord>(
        "SELECT * FROM oauth_providers WHERE id = $1"
    )
    .bind(provider_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    Ok(HttpResponse::Ok().json(OAuthProviderAdminInfo {
        id: provider.id,
        name: provider.name,
        display_name: provider.display_name,
        provider_type: provider.provider_type,
        client_id: provider.client_id,
        authorization_url: provider.authorization_url,
        token_url: provider.token_url,
        userinfo_url: provider.userinfo_url,
        scopes: serde_json::from_value(provider.scopes).unwrap_or_default(),
        icon: provider.icon,
        enabled: provider.enabled,
        sort_order: provider.sort_order,
        created_at: provider.created_at,
        updated_at: provider.updated_at,
    }))
}

// Helper functions for default provider URLs
fn get_default_provider_urls(
    provider_type: &str,
    req: &CreateOAuthProviderRequest,
) -> (String, String, Option<String>) {
    let auth = req.authorization_url.clone();
    let token = req.token_url.clone();
    let user = req.userinfo_url.clone();

    match provider_type {
        "github" => (
            auth.unwrap_or_else(|| "https://github.com/login/oauth/authorize".to_string()),
            token.unwrap_or_else(|| "https://github.com/login/oauth/access_token".to_string()),
            user.or_else(|| Some("https://api.github.com/user".to_string())),
        ),
        "gitlab" => (
            auth.unwrap_or_else(|| "https://gitlab.com/oauth/authorize".to_string()),
            token.unwrap_or_else(|| "https://gitlab.com/oauth/token".to_string()),
            user.or_else(|| Some("https://gitlab.com/api/v4/user".to_string())),
        ),
        "google" => (
            auth.unwrap_or_else(|| "https://accounts.google.com/o/oauth2/v2/auth".to_string()),
            token.unwrap_or_else(|| "https://oauth2.googleapis.com/token".to_string()),
            user.or_else(|| Some("https://www.googleapis.com/oauth2/v2/userinfo".to_string())),
        ),
        "azure_ad" => (
            auth.unwrap_or_else(|| "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string()),
            token.unwrap_or_else(|| "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string()),
            user.or_else(|| Some("https://graph.microsoft.com/v1.0/me".to_string())),
        ),
        "bitbucket" => (
            auth.unwrap_or_else(|| "https://bitbucket.org/site/oauth2/authorize".to_string()),
            token.unwrap_or_else(|| "https://bitbucket.org/site/oauth2/access_token".to_string()),
            user.or_else(|| Some("https://api.bitbucket.org/2.0/user".to_string())),
        ),
        _ => (
            auth.unwrap_or_else(|| "".to_string()),
            token.unwrap_or_else(|| "".to_string()),
            user,
        ),
    }
}

fn get_default_scopes(provider_type: &str) -> Vec<String> {
    match provider_type {
        "github" => vec!["user:email".to_string()],
        "gitlab" => vec!["read_user".to_string()],
        "google" => vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
        "azure_ad" => vec!["openid".to_string(), "email".to_string(), "profile".to_string()],
        "bitbucket" => vec!["account".to_string()],
        _ => vec![],
    }
}
