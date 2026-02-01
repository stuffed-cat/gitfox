use actix_web::{web, HttpResponse};
use sqlx::PgPool;


use crate::error::AppResult;
use crate::models::{UpdateUserRequest, UserInfo};
use crate::services::UserService;

#[derive(Debug, serde::Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub async fn list_users(
    pool: web::Data<PgPool>,
    query: web::Query<ListQuery>,
) -> AppResult<HttpResponse> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    
    let users = UserService::list_users(pool.get_ref(), page, per_page).await?;
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
