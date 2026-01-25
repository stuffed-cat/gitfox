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
