use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    InternalError(String),
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    DatabaseError(String),
    GitError(String),
    QueueError(String),
    ValidationError(String),
    TooManyRequests(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            AppError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            AppError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AppError::GitError(msg) => write!(f, "Git error: {}", msg),
            AppError::QueueError(msg) => write!(f, "Queue error: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::TooManyRequests(msg) => write!(f, "Too many requests: {}", msg),
        }
    }
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, error_type) = match self {
            AppError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database_error"),
            AppError::GitError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "git_error"),
            AppError::QueueError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "queue_error"),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, "validation_error"),
            AppError::TooManyRequests(_) => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_exceeded"),
        };

        HttpResponse::build(status).json(serde_json::json!({
            "error": error_type,
            "message": self.to_string()
        }))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::InternalError(err.to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::QueueError(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::Unauthorized(err.to_string())
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        AppError::InternalError(err.to_string())
    }
}

impl From<lettre::transport::smtp::Error> for AppError {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        AppError::InternalError(format!("SMTP error: {}", err))
    }
}

impl From<actix_web::Error> for AppError {
    fn from(err: actix_web::Error) -> Self {
        AppError::InternalError(err.to_string())
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::ValidationError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InternalError(format!("JSON error: {}", err))
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(err: serde_yaml::Error) -> Self {
        AppError::InternalError(format!("YAML error: {}", err))
    }
}

// Convenience constructors
impl AppError {
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        AppError::InternalError(msg.into())
    }
    
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        AppError::NotFound(msg.into())
    }
    
    pub fn bad_request<S: Into<String>>(msg: S) -> Self {
        AppError::BadRequest(msg.into())
    }
    
    pub fn unauthorized<S: Into<String>>(msg: S) -> Self {
        AppError::Unauthorized(msg.into())
    }
    
    pub fn forbidden<S: Into<String>>(msg: S) -> Self {
        AppError::Forbidden(msg.into())
    }
    
    pub fn conflict<S: Into<String>>(msg: S) -> Self {
        AppError::Conflict(msg.into())
    }
}

pub type AppResult<T> = Result<T, AppError>;
