use actix_web::{dev::ServiceRequest, Error, HttpMessage, HttpRequest, FromRequest};
use actix_web::error::{ErrorUnauthorized, ErrorForbidden};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sha2::{Digest, Sha256};
use sqlx::PgPool;


use crate::config::AppConfig;
use crate::models::{Claims, UserRole, PAT_PREFIX, TokenScope, Scope};

/// Hash a token using SHA-256 (same as in PAT and OAuth handlers)
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Validate a Personal Access Token and return (user_id, username, role, scopes)
async fn validate_pat_token(pool: &PgPool, token: &str) -> Result<(i64, String, String, TokenScope), Error> {
    if !token.starts_with(PAT_PREFIX) {
        return Err(ErrorUnauthorized("Not a valid PAT token"));
    }

    let token_hash = hash_token(token);

    let result = sqlx::query_as::<_, (i64, String, String, Vec<String>)>(
        r#"
        SELECT u.id, u.username, u.role::text, pat.scopes
        FROM personal_access_tokens pat
        JOIN users u ON u.id = pat.user_id
        WHERE pat.token_hash = $1 
          AND pat.revoked_at IS NULL
          AND (pat.expires_at IS NULL OR pat.expires_at > NOW())
        "#
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ErrorUnauthorized(format!("Database error: {}", e)))?;

    if let Some((user_id, username, role, scopes_vec)) = result {
        // Convert Vec<String> to TokenScope
        let scopes = TokenScope::from_strings(scopes_vec);
        
        // Update last_used_at asynchronously (don't wait for it)
        let pool_clone = pool.clone();
        let hash_clone = token_hash.clone();
        actix_web::rt::spawn(async move {
            let _ = sqlx::query("UPDATE personal_access_tokens SET last_used_at = NOW() WHERE token_hash = $1")
                .bind(&hash_clone)
                .execute(&pool_clone)
                .await;
        });

        return Ok((user_id, username, role, scopes));
    }

    Err(ErrorUnauthorized("Invalid or expired PAT token"))
}

/// Validate an OAuth2 access token and return (user_id, username, role, scopes)
async fn validate_oauth_token(pool: &PgPool, token: &str) -> Result<(i64, String, String, TokenScope), Error> {
    let token_hash = hash_token(token);

    let result = sqlx::query_as::<_, (i64, String, String, serde_json::Value)>(
        r#"
        SELECT u.id, u.username, u.role::text, oat.scopes
        FROM oauth_access_tokens oat
        JOIN users u ON u.id = oat.user_id
        WHERE oat.token_hash = $1 
          AND oat.revoked_at IS NULL
          AND oat.expires_at > NOW()
        "#
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| ErrorUnauthorized(format!("Database error: {}", e)))?;

    if let Some((user_id, username, role, scopes_json)) = result {
        // Parse scopes from JSONB
        let scopes_vec: Vec<String> = serde_json::from_value(scopes_json)
            .unwrap_or_default();
        
        // Convert to TokenScope
        let scopes = TokenScope::from_strings(scopes_vec);
        
        // Update last_used_at asynchronously
        let pool_clone = pool.clone();
        let hash_clone = token_hash.clone();
        actix_web::rt::spawn(async move {
            let _ = sqlx::query("UPDATE oauth_access_tokens SET last_used_at = NOW() WHERE token_hash = $1")
                .bind(&hash_clone)
                .execute(&pool_clone)
                .await;
        });

        return Ok((user_id, username, role, scopes));
    }

    Err(ErrorUnauthorized("Invalid or expired OAuth token"))
}

/// Validate JWT token and return (user_id, username, role, scopes)
fn validate_jwt_token(config: &AppConfig, token: &str) -> Result<(i64, String, String, TokenScope), Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| ErrorUnauthorized(format!("Invalid JWT token: {}", e)))?;

    Ok((
        token_data.claims.user_id,
        token_data.claims.sub,
        format!("{:?}", token_data.claims.role),
        token_data.claims.scopes, // Full or Limited based on claims
    ))
}


pub async fn validate_token(req: &ServiceRequest, config: &AppConfig) -> Result<Claims, Error> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok_or_else(|| ErrorUnauthorized("Missing authorization header"))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| ErrorUnauthorized("Invalid authorization header"))?;

    if !auth_str.starts_with("Bearer ") {
        return Err(ErrorUnauthorized("Invalid authorization scheme"));
    }

    let token = &auth_str[7..];

    // Try JWT token first
    if let Ok(token_data) = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        return Ok(token_data.claims);
    }

    // Try PAT and OAuth tokens if we have DB pool
    if let Some(pool) = req.app_data::<actix_web::web::Data<PgPool>>() {
        // Try Personal Access Token
        if token.starts_with(PAT_PREFIX) {
            if let Ok((user_id, username, role_str, scopes)) = validate_pat_token(pool.get_ref(), token).await {
                let role = match role_str.as_str() {
                    "admin" => UserRole::Admin,
                    "developer" => UserRole::Developer,
                    _ => UserRole::Viewer,
                };
                return Ok(Claims {
                    sub: username,
                    user_id,
                    role,
                    exp: 0, // Will not be checked for PAT
                    iat: 0,
                    scopes,
                });
            }
        }
        
        // Try OAuth2 access token
        if let Ok((user_id, username, role_str, scopes)) = validate_oauth_token(pool.get_ref(), token).await {
            let role = match role_str.as_str() {
                "admin" => UserRole::Admin,
                "developer" => UserRole::Developer,
                _ => UserRole::Viewer,
            };
            return Ok(Claims {
                sub: username,
                user_id,
                role,
                exp: 0, // Will not be checked for OAuth
                iat: 0,
                scopes,
            });
        }
    }

    Err(ErrorUnauthorized("Invalid or expired token"))
}

/// Try to validate token, returns Option<Claims> instead of error
pub async fn try_validate_token(req: &ServiceRequest, config: &AppConfig) -> Option<Claims> {
    let auth_header = req.headers().get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    if !auth_str.starts_with("Bearer ") {
        return None;
    }
    
    let token = &auth_str[7..];
    
    // Try JWT token first
    if let Ok(token_data) = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        return Some(token_data.claims);
    }
    
    // Try PAT and OAuth tokens
    if let Some(pool) = req.app_data::<actix_web::web::Data<PgPool>>() {
        // Try PAT
        if token.starts_with(PAT_PREFIX) {
            if let Ok((user_id, username, role_str, scopes)) = validate_pat_token(pool.get_ref(), token).await {
                let role = match role_str.as_str() {
                    "admin" => UserRole::Admin,
                    "developer" => UserRole::Developer,
                    _ => UserRole::Viewer,
                };
                return Some(Claims {
                    sub: username,
                    user_id,
                    role,
                    exp: 0,
                    iat: 0,
                    scopes,
                });
            }
        }
        
        // Try OAuth2
        if let Ok((user_id, username, role_str, scopes)) = validate_oauth_token(pool.get_ref(), token).await {
            let role = match role_str.as_str() {
                "admin" => UserRole::Admin,
                "developer" => UserRole::Developer,
                _ => UserRole::Viewer,
            };
            return Some(Claims {
                sub: username,
                user_id,
                role,
                exp: 0,
                iat: 0,
                scopes,
            });
        }
    }
    
    None
}

pub fn extract_user_from_request(req: &ServiceRequest) -> Option<Claims> {
    req.extensions().get::<Claims>().cloned()
}

/// Extractor for authenticated user (required)
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub username: String,
    pub role: String,
    /// Token scopes: Full = JWT (full access), Limited = PAT/OAuth (restricted)
    pub scopes: TokenScope,
}

impl AuthenticatedUser {
    /// Check if user has a specific scope (or full access via JWT)
    pub fn has_scope(&self, scope: &Scope) -> bool {
        self.scopes.has(scope)
    }
    
    /// Check if user has any of the given scopes
    pub fn has_any_scope(&self, required_scopes: &[Scope]) -> bool {
        self.scopes.has_any(required_scopes)
    }
    
    /// Check if user has all of the given scopes
    pub fn has_all_scopes(&self, required_scopes: &[Scope]) -> bool {
        self.scopes.has_all(required_scopes)
    }
}

impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization").cloned();
        let config = req.app_data::<actix_web::web::Data<AppConfig>>().cloned();
        let pool = req.app_data::<actix_web::web::Data<PgPool>>().cloned();
        
        Box::pin(async move {
            let auth_header_value = auth_header
                .ok_or_else(|| ErrorUnauthorized("Missing authorization header"))?;
            
            let auth_str = auth_header_value
                .to_str()
                .map_err(|_| ErrorUnauthorized("Invalid authorization header"))?;
            
            if !auth_str.starts_with("Bearer ") {
                return Err(ErrorUnauthorized("Invalid authorization scheme"));
            }
            
            let token = &auth_str[7..];
            let config = config.ok_or_else(|| ErrorUnauthorized("Server configuration error"))?;
            
            // Try JWT token first (fastest, no DB query)
            if let Ok((user_id, username, role, scopes)) = validate_jwt_token(&config, token) {
                return Ok(AuthenticatedUser {
                    user_id,
                    username,
                    role,
                    scopes,
                });
            }
            
            // If JWT fails and we have DB pool, try PAT and OAuth tokens
            if let Some(pool) = pool {
                // Try Personal Access Token
                if token.starts_with(PAT_PREFIX) {
                    if let Ok((user_id, username, role, scopes)) = validate_pat_token(pool.get_ref(), token).await {
                        return Ok(AuthenticatedUser {
                            user_id,
                            username,
                            role,
                            scopes,
                        });
                    }
                }
                
                // Try OAuth2 access token
                if let Ok((user_id, username, role, scopes)) = validate_oauth_token(pool.get_ref(), token).await {
                    return Ok(AuthenticatedUser {
                        user_id,
                        username,
                        role,
                        scopes,
                    });
                }
            }
            
            Err(ErrorUnauthorized("Invalid or expired token"))
        })
    }
}

/// Extractor for optional authentication (not required)
pub struct OptionalAuth(Option<AuthenticatedUser>);

impl OptionalAuth {
    pub fn user_id(&self) -> Option<i64> {
        self.0.as_ref().map(|u| u.user_id)
    }
    
    pub fn user(&self) -> Option<&AuthenticatedUser> {
        self.0.as_ref()
    }
}

impl FromRequest for OptionalAuth {
    type Error = actix_web::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization").cloned();
        let config = req.app_data::<actix_web::web::Data<AppConfig>>().cloned();
        let pool = req.app_data::<actix_web::web::Data<PgPool>>().cloned();
        
        Box::pin(async move {
            let user = (|| async {
                let auth_header_value = auth_header?;
                let auth_str = auth_header_value.to_str().ok()?;
                
                if !auth_str.starts_with("Bearer ") {
                    return None;
                }
                
                let token = &auth_str[7..];
                let config = config?;
                
                // Try JWT token first
                if let Ok((user_id, username, role, scopes)) = validate_jwt_token(&config, token) {
                    return Some(AuthenticatedUser {
                        user_id,
                        username,
                        role,
                        scopes,
                    });
                }
                
                // Try PAT and OAuth if DB pool is available
                if let Some(pool) = pool {
                    // Try PAT
                    if token.starts_with(PAT_PREFIX) {
                        if let Ok((user_id, username, role, scopes)) = validate_pat_token(pool.get_ref(), token).await {
                            return Some(AuthenticatedUser {
                                user_id,
                                username,
                                role,
                                scopes,
                            });
                        }
                    }
                    
                    // Try OAuth2
                    if let Ok((user_id, username, role, scopes)) = validate_oauth_token(pool.get_ref(), token).await {
                        return Some(AuthenticatedUser {
                            user_id,
                            username,
                            role,
                            scopes,
                        });
                    }
                }
                
                None
            })()
            .await;
            
            Ok(OptionalAuth(user))
        })
    }
}

/// Extractor for admin-only authentication (requires admin role)
/// Note: Only JWT tokens can grant admin access. PAT and OAuth tokens are always treated as regular users.
pub struct AdminUser {
    pub user_id: i64,
    pub username: String,
}

impl FromRequest for AdminUser {
    type Error = actix_web::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization").cloned();
        let config = req.app_data::<actix_web::web::Data<AppConfig>>().cloned();
        
        Box::pin(async move {
            let auth_header_value = auth_header
                .ok_or_else(|| ErrorUnauthorized("Missing authorization header"))?;
            
            let auth_str = auth_header_value
                .to_str()
                .map_err(|_| ErrorUnauthorized("Invalid authorization header"))?;
            
            if !auth_str.starts_with("Bearer ") {
                return Err(ErrorUnauthorized("Invalid authorization scheme"));
            }
            
            let token = &auth_str[7..];
            let config = config.ok_or_else(|| ErrorUnauthorized("Server configuration error"))?;
            
            // Admin access ONLY via JWT tokens (not PAT or OAuth)
            let token_data = decode::<Claims>(
                token,
                &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
                &Validation::default(),
            )
            .map_err(|e| ErrorUnauthorized(format!("Invalid token: {}", e)))?;
            
            // Check admin role
            if token_data.claims.role != UserRole::Admin {
                return Err(ErrorForbidden("Admin access required"));
            }
            
            Ok(AdminUser {
                user_id: token_data.claims.user_id,
                username: token_data.claims.sub,
            })
        })
    }
}
