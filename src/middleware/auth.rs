use actix_web::{dev::ServiceRequest, Error, HttpMessage, HttpRequest, FromRequest};
use actix_web::error::{ErrorUnauthorized, ErrorForbidden};
use jsonwebtoken::{decode, DecodingKey, Validation};
use std::future::{Ready, ready};


use crate::config::AppConfig;
use crate::models::{Claims, UserRole};

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

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| ErrorUnauthorized(format!("Invalid token: {}", e)))?;

    Ok(token_data.claims)
}

/// Try to validate token, returns Option<Claims> instead of error
pub async fn try_validate_token(req: &ServiceRequest, config: &AppConfig) -> Option<Claims> {
    let auth_header = req.headers().get("Authorization")?;
    let auth_str = auth_header.to_str().ok()?;
    
    if !auth_str.starts_with("Bearer ") {
        return None;
    }
    
    let token = &auth_str[7..];
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;
    
    Some(token_data.claims)
}

pub fn extract_user_from_request(req: &ServiceRequest) -> Option<Claims> {
    req.extensions().get::<Claims>().cloned()
}

/// Extractor for authenticated user (required)
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

impl FromRequest for AuthenticatedUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization");
        
        let result = (|| {
            let auth_str = auth_header
                .ok_or_else(|| ErrorUnauthorized("Missing authorization header"))?
                .to_str()
                .map_err(|_| ErrorUnauthorized("Invalid authorization header"))?;
            
            if !auth_str.starts_with("Bearer ") {
                return Err(ErrorUnauthorized("Invalid authorization scheme"));
            }
            
            let token = &auth_str[7..];
            
            // Get config from app data
            let config = req.app_data::<actix_web::web::Data<AppConfig>>()
                .ok_or_else(|| ErrorUnauthorized("Server configuration error"))?;
            
            let token_data = decode::<Claims>(
                token,
                &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
                &Validation::default(),
            )
            .map_err(|e| ErrorUnauthorized(format!("Invalid token: {}", e)))?;
            
            Ok(AuthenticatedUser {
                user_id: token_data.claims.user_id,
                username: token_data.claims.sub,
                role: format!("{:?}", token_data.claims.role),
            })
        })();
        
        ready(result)
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
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization");
        
        let user = (|| {
            let auth_str = auth_header?.to_str().ok()?;
            
            if !auth_str.starts_with("Bearer ") {
                return None;
            }
            
            let token = &auth_str[7..];
            
            let config = req.app_data::<actix_web::web::Data<AppConfig>>()?;
            
            let token_data = decode::<Claims>(
                token,
                &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
                &Validation::default(),
            )
            .ok()?;
            
            Some(AuthenticatedUser {
                user_id: token_data.claims.user_id,
                username: token_data.claims.sub,
                role: format!("{:?}", token_data.claims.role),
            })
        })();
        
        ready(Ok(OptionalAuth(user)))
    }
}

/// Extractor for admin-only authentication (requires admin role)
pub struct AdminUser {
    pub user_id: i64,
    pub username: String,
}

impl FromRequest for AdminUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let auth_header = req.headers().get("Authorization");
        
        let result = (|| {
            let auth_str = auth_header
                .ok_or_else(|| ErrorUnauthorized("Missing authorization header"))?
                .to_str()
                .map_err(|_| ErrorUnauthorized("Invalid authorization header"))?;
            
            if !auth_str.starts_with("Bearer ") {
                return Err(ErrorUnauthorized("Invalid authorization scheme"));
            }
            
            let token = &auth_str[7..];
            
            let config = req.app_data::<actix_web::web::Data<AppConfig>>()
                .ok_or_else(|| ErrorUnauthorized("Server configuration error"))?;
            
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
        })();
        
        ready(result)
    }
}
