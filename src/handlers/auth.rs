use actix_web::{web, HttpRequest, HttpResponse};
use sqlx::PgPool;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::middleware::validate_token;
use crate::models::{CreateUserRequest, LoginRequest, UserInfo};
use crate::services::UserService;

pub async fn register(
    pool: web::Data<PgPool>,
    body: web::Json<CreateUserRequest>,
) -> AppResult<HttpResponse> {
    let user = UserService::create_user(pool.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(UserInfo::from(user)))
}

pub async fn login(
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
    body: web::Json<LoginRequest>,
) -> AppResult<HttpResponse> {
    let response = UserService::login(pool.get_ref(), config.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn me(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<AppConfig>,
) -> AppResult<HttpResponse> {
    let service_req = actix_web::dev::ServiceRequest::from_request(req.clone());
    let claims = validate_token(&service_req, config.get_ref()).await?;
    let user = UserService::get_user_by_id(pool.get_ref(), claims.user_id).await?;
    Ok(HttpResponse::Ok().json(UserInfo::from(user)))
}
