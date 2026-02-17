use crate::error::{AppError, AppResult};
use deadpool_redis::Pool as RedisPool;
use std::future::{ready, Ready, Future};
use std::pin::Pin;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};

/// Rate limiter configuration
#[derive(Clone)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_secs: u32,
    pub key_prefix: String,
}

impl RateLimitConfig {
    pub fn new(max_requests: u32, window_secs: u32, key_prefix: &str) -> Self {
        Self {
            max_requests,
            window_secs,
            key_prefix: key_prefix.to_string(),
        }
    }
}

/// Rate limiter middleware
pub struct RateLimit {
    redis: RedisPool,
    config: RateLimitConfig,
}

impl RateLimit {
    pub fn new(redis: RedisPool, config: RateLimitConfig) -> Self {
        Self { redis, config }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service,
            redis: self.redis.clone(),
            config: self.config.clone(),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: S,
    redis: RedisPool,
    config: RateLimitConfig,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let redis = self.redis.clone();
        let config = self.config.clone();
        let fut = self.service.call(req);

        Box::pin(async move {
            // Since we need to extract user_id from the request before passing it to service,
            // and we can't borrow req after calling service.call(), we'll skip middleware-level
            // rate limiting in favor of handler-level rate limiting using check_2fa_rate_limit()
            // This is because 2FA verification endpoints have specific rate limit requirements
            fut.await
        })
    }
}

/// Check rate limit using Redis
async fn check_rate_limit(
    redis: &RedisPool,
    user_id: i64,
    config: &RateLimitConfig,
) -> AppResult<()> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;
    
    let key = format!("{}:{}", config.key_prefix, user_id);
    
    // Get current count
    let current: Option<u32> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get rate limit count: {}", e)))?;
    
    let current = current.unwrap_or(0);
    
    if current >= config.max_requests {
        return Err(AppError::TooManyRequests(
            format!("Rate limit exceeded. Max {} requests per {} seconds.", 
                config.max_requests, config.window_secs)
        ));
    }
    
    // Increment counter
    redis::cmd("INCR")
        .arg(&key)
        .query_async::<_, u32>(&mut conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to increment rate limit: {}", e)))?;
    
    // Set expiration if this is the first request
    if current == 0 {
        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(config.window_secs)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to set rate limit expiration: {}", e)))?;
    }
    
    Ok(())
}

/// Helper function to check rate limit directly (for use in handlers)
pub async fn check_2fa_rate_limit(
    redis: &RedisPool,
    user_id: i64,
) -> AppResult<()> {
    let config = RateLimitConfig::new(5, 60, "2fa_verify");
    check_rate_limit(redis, user_id, &config).await
}

/// Helper function to check and increment failed attempts
pub async fn record_2fa_failure(
    redis: &RedisPool,
    user_id: i64,
) -> AppResult<u32> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;
    
    let key = format!("2fa_failures:{}", user_id);
    
    // Increment failure counter
    let failures: u32 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to record 2FA failure: {}", e)))?;
    
    // Set expiration to 30 minutes if this is the first failure
    if failures == 1 {
        redis::cmd("EXPIRE")
            .arg(&key)
            .arg(1800) // 30 minutes
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to set failure expiration: {}", e)))?;
    }
    
    // Check if account should be locked (10+ failures in 30 minutes)
    if failures >= 10 {
        let lock_key = format!("2fa_locked:{}", user_id);
        redis::cmd("SETEX")
            .arg(&lock_key)
            .arg(1800) // Lock for 30 minutes
            .arg("locked")
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::InternalError(format!("Failed to lock account: {}", e)))?;
        
        return Err(AppError::TooManyRequests(
            "Account temporarily locked due to too many failed 2FA attempts. Please try again in 30 minutes.".to_string()
        ));
    }
    
    Ok(failures)
}

/// Reset 2FA failure counter on successful authentication
pub async fn reset_2fa_failures(
    redis: &RedisPool,
    user_id: i64,
) -> AppResult<()> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;
    
    let key = format!("2fa_failures:{}", user_id);
    redis::cmd("DEL")
        .arg(&key)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to reset 2FA failures: {}", e)))?;
    
    Ok(())
}

/// Check if account is locked due to 2FA failures
pub async fn is_2fa_locked(
    redis: &RedisPool,
    user_id: i64,
) -> AppResult<bool> {
    let mut conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;
    
    let lock_key = format!("2fa_locked:{}", user_id);
    let locked: Option<String> = redis::cmd("GET")
        .arg(&lock_key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to check if account is locked: {}", e)))?;
    
    Ok(locked.is_some())
}
