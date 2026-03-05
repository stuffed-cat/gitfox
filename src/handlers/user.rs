use actix_web::{web, HttpResponse};
use actix_multipart::Multipart;
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use std::io::Write;
use std::path::Path;
use futures::stream::TryStreamExt;

use crate::error::{AppResult, AppError};
use crate::models::{UpdateUserRequest, UserInfo};
use crate::services::{UserService, RunnerUsageService};
use crate::middleware::auth::AuthenticatedUser;

#[derive(Debug, serde::Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
}

pub async fn list_users(
    pool: web::Data<PgPool>,
    query: web::Query<ListQuery>,
) -> AppResult<HttpResponse> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    
    let users = if let Some(search) = &query.search {
        UserService::search_users(pool.get_ref(), search, page, per_page).await?
    } else {
        UserService::list_users(pool.get_ref(), page, per_page).await?
    };
    let user_infos: Vec<UserInfo> = users.into_iter().map(UserInfo::from).collect();
    
    Ok(HttpResponse::Ok().json(user_infos))
}

pub async fn get_user(
    pool: web::Data<PgPool>,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let user = UserService::get_user_by_id(pool.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(UserInfo::from(user)))
}

pub async fn get_user_by_username(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let user = UserService::get_user_by_username(pool.get_ref(), &path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(UserInfo::from(user)))
}

pub async fn update_user(
    pool: web::Data<PgPool>,
    path: web::Path<i64>,
    body: web::Json<UpdateUserRequest>,
) -> AppResult<HttpResponse> {
    let user = UserService::update_user(
        pool.get_ref(),
        path.into_inner(),
        body.display_name.clone(),
        body.avatar_url.clone(),
    ).await?;
    
    Ok(HttpResponse::Ok().json(UserInfo::from(user)))
}

pub async fn delete_user(
    pool: web::Data<PgPool>,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    UserService::delete_user(pool.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Request for batch avatar lookup by emails
#[derive(Debug, serde::Deserialize)]
pub struct AvatarsByEmailsRequest {
    pub emails: Vec<String>,
}

/// Response for avatar lookup
#[derive(Debug, serde::Serialize)]
pub struct AvatarInfo {
    pub email: String,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
}

/// POST /api/v1/users/avatars - Get avatars by email addresses
pub async fn get_avatars_by_emails(
    pool: web::Data<PgPool>,
    body: web::Json<AvatarsByEmailsRequest>,
) -> AppResult<HttpResponse> {
    let emails = &body.emails;
    
    if emails.is_empty() {
        return Ok(HttpResponse::Ok().json(Vec::<AvatarInfo>::new()));
    }
    
    // Query users by email
    let users = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        r#"SELECT email, avatar_url, display_name FROM users WHERE email = ANY($1)"#
    )
    .bind(emails)
    .fetch_all(pool.get_ref())
    .await?;
    
    let avatars: Vec<AvatarInfo> = users
        .into_iter()
        .map(|(email, avatar_url, display_name)| AvatarInfo {
            email,
            avatar_url,
            display_name,
        })
        .collect();
    
    Ok(HttpResponse::Ok().json(avatars))
}

/// PUT /api/v1/user/profile - Update current user's profile including status
pub async fn update_current_user_profile(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    body: web::Json<UpdateUserRequest>,
) -> AppResult<HttpResponse> {
    let user = UserService::update_user_profile(
        pool.get_ref(),
        auth.user_id,
        body.display_name.clone(),
        body.avatar_url.clone(),
        body.status_emoji.clone(),
        body.status_message.clone(),
        body.busy,
        body.clear_status_after.clone(),
    ).await?;
    
    Ok(HttpResponse::Ok().json(UserInfo::from(user)))
}

#[derive(Debug, serde::Serialize)]
pub struct AvatarUploadResponse {
    pub avatar_url: String,
}

/// POST /api/v1/user/avatar - Upload user avatar
pub async fn upload_avatar(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    mut payload: Multipart,
) -> AppResult<HttpResponse> {
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        
        // Only accept 'avatar' field
        if content_disposition.get_name() != Some("avatar") {
            continue;
        }
        
        let content_type = field.content_type()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());
        
        // Determine file extension based on content type
        let ext = match content_type.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => {
                return Err(AppError::BadRequest(
                    "Only image files are allowed (JPEG, PNG, WebP, GIF)".to_string()
                ))
            }
        };
        
        // Read all chunks from the field
        let mut bytes = Vec::new();
        while let Ok(Some(chunk)) = field.try_next().await {
            bytes.extend_from_slice(&chunk);
        }
        
        if bytes.is_empty() {
            return Err(AppError::BadRequest("Avatar file is empty".to_string()));
        }
        
        // Generate avatar filename
        let avatar_filename = format!("avatar_{}_{}.{}", auth.user_id, chrono::Utc::now().timestamp(), ext);
        let assets_dir = Path::new("assets");
        let full_path = assets_dir.join(&avatar_filename);
        
        // Create assets directory if it doesn't exist
        std::fs::create_dir_all(assets_dir)?;
        
        // Save file
        let mut file = std::fs::File::create(&full_path)?;
        file.write_all(&bytes)?;
        
        // Build avatar URL
        let avatar_url = format!("/assets/{}", avatar_filename);
        
        // Update user's avatar_url
        UserService::update_user(
            pool.get_ref(),
            auth.user_id,
            None,
            Some(avatar_url.clone()),
        ).await?;
        
        return Ok(HttpResponse::Ok().json(AvatarUploadResponse {
            avatar_url,
        }));
    }
    
    Err(AppError::BadRequest("No avatar field in request".to_string()))
}

/// GET /api/v1/user/runner-usage - Get current user's runner usage
pub async fn get_runner_usage(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    auth: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let summary = RunnerUsageService::get_usage_summary(pool.get_ref(), redis.get_ref(), auth.user_id).await?;
    Ok(HttpResponse::Ok().json(summary))
}

/// GET /api/v1/user/runner-usage/history - Get current user's runner usage history
#[derive(Debug, serde::Deserialize)]
pub struct UsageHistoryQuery {
    pub limit: Option<i64>,
}

pub async fn get_runner_usage_history(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    query: web::Query<UsageHistoryQuery>,
) -> AppResult<HttpResponse> {
    let limit = query.limit.unwrap_or(50).min(200);
    let history = RunnerUsageService::get_usage_history(pool.get_ref(), auth.user_id, limit).await?;
    Ok(HttpResponse::Ok().json(history))
}

/// GET /api/v1/user/runner-usage/stats - Get current user's monthly statistics
#[derive(Debug, serde::Deserialize)]
pub struct MonthlyStatsQuery {
    pub months: Option<i32>,
}

pub async fn get_runner_usage_stats(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    query: web::Query<MonthlyStatsQuery>,
) -> AppResult<HttpResponse> {
    let months = query.months.unwrap_or(6).min(12);
    let stats = RunnerUsageService::get_monthly_stats(pool.get_ref(), auth.user_id, months).await?;
    Ok(HttpResponse::Ok().json(stats))
}
