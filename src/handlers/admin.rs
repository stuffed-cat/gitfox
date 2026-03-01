use actix_web::{web, HttpResponse};
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

use crate::error::AppResult;
use crate::middleware::AdminUser;
use crate::models::{UserInfo, UserRole};
use crate::services::SystemConfigService;
use crate::services::system_config::BulkUpdateConfigRequest;

/// Admin dashboard statistics
#[derive(Debug, serde::Serialize)]
pub struct SystemStats {
    pub total_users: i64,
    pub active_users: i64,
    pub total_projects: i64,
    pub total_groups: i64,
    pub admin_count: i64,
}

/// Admin user info (includes more fields than regular UserInfo)
#[derive(Debug, serde::Serialize)]
pub struct AdminUserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub projects_count: i64,
    pub is_pro: bool,
}

/// Admin user list with pagination
#[derive(Debug, serde::Serialize)]
pub struct AdminUserListResponse {
    pub users: Vec<AdminUserInfo>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, serde::Deserialize)]
pub struct AdminListQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
}

/// Request to update a user as admin
#[derive(Debug, serde::Deserialize)]
pub struct AdminUpdateUserRequest {
    pub role: Option<UserRole>,
    pub is_active: Option<bool>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub is_pro: Option<bool>,
}

/// GET /api/v1/admin/dashboard - System overview statistics
pub async fn dashboard(
    pool: web::Data<PgPool>,
    _admin: AdminUser,
) -> AppResult<HttpResponse> {
    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool.get_ref())
        .await?;

    let active_users = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE is_active = true"
    )
    .fetch_one(pool.get_ref())
    .await?;

    let total_projects = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects")
        .fetch_one(pool.get_ref())
        .await?;

    let total_groups = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM namespaces WHERE namespace_type = 'group'"
    )
    .fetch_one(pool.get_ref())
    .await?;

    let admin_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE role = 'admin'"
    )
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(SystemStats {
        total_users,
        active_users,
        total_projects,
        total_groups,
        admin_count,
    }))
}

/// GET /api/v1/admin/users - List all users with admin details
pub async fn list_users(
    pool: web::Data<PgPool>,
    _admin: AdminUser,
    query: web::Query<AdminListQuery>,
) -> AppResult<HttpResponse> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page.saturating_sub(1)) * per_page;

    // Build dynamic WHERE clause
    let mut conditions = Vec::new();
    let mut param_idx = 1u32;
    
    if let Some(ref search) = query.search {
        if !search.is_empty() {
            conditions.push(format!(
                "(username ILIKE '%' || ${} || '%' OR email ILIKE '%' || ${} || '%' OR display_name ILIKE '%' || ${} || '%')",
                param_idx, param_idx, param_idx
            ));
            param_idx += 1;
        }
    }
    
    if let Some(ref role) = query.role {
        if !role.is_empty() && role != "all" {
            conditions.push(format!("role = ${}::user_role", param_idx));
            param_idx += 1;
        }
    }
    
    if let Some(ref status) = query.status {
        match status.as_str() {
            "active" => conditions.push("is_active = true".to_string()),
            "blocked" => conditions.push("is_active = false".to_string()),
            _ => {}
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count total with filters
    let count_sql = format!("SELECT COUNT(*) FROM users {}", where_clause);
    let list_sql = format!(
        r#"SELECT u.id, u.username, u.email, u.display_name, u.avatar_url, 
                  u.role, u.is_active, u.created_at, u.updated_at, u.is_pro,
                  COALESCE((SELECT COUNT(*) FROM projects p WHERE p.owner_id = u.id), 0) as projects_count
           FROM users u {}
           ORDER BY u.created_at DESC
           LIMIT {} OFFSET {}"#,
        where_clause, per_page, offset
    );

    // Execute with dynamic binds
    let total: i64;
    let users: Vec<AdminUserInfo>;

    // For simplicity: build queries with bind parameters
    match (&query.search, &query.role) {
        (Some(search), Some(role)) if !search.is_empty() && !role.is_empty() && role != "all" => {
            total = sqlx::query_scalar::<_, i64>(&count_sql)
                .bind(search)
                .bind(role)
                .fetch_one(pool.get_ref())
                .await?;
            users = sqlx::query_as::<_, AdminUserInfoRow>(&list_sql)
                .bind(search)
                .bind(role)
                .fetch_all(pool.get_ref())
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
        }
        (Some(search), _) if !search.is_empty() => {
            total = sqlx::query_scalar::<_, i64>(&count_sql)
                .bind(search)
                .fetch_one(pool.get_ref())
                .await?;
            users = sqlx::query_as::<_, AdminUserInfoRow>(&list_sql)
                .bind(search)
                .fetch_all(pool.get_ref())
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
        }
        (_, Some(role)) if !role.is_empty() && role != "all" => {
            total = sqlx::query_scalar::<_, i64>(&count_sql)
                .bind(role)
                .fetch_one(pool.get_ref())
                .await?;
            users = sqlx::query_as::<_, AdminUserInfoRow>(&list_sql)
                .bind(role)
                .fetch_all(pool.get_ref())
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
        }
        _ => {
            total = sqlx::query_scalar::<_, i64>(&count_sql)
                .fetch_one(pool.get_ref())
                .await?;
            users = sqlx::query_as::<_, AdminUserInfoRow>(&list_sql)
                .fetch_all(pool.get_ref())
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
        }
    }

    Ok(HttpResponse::Ok().json(AdminUserListResponse {
        users,
        total,
        page,
        per_page,
    }))
}

#[derive(Debug, sqlx::FromRow)]
struct AdminUserInfoRow {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub projects_count: i64,
    pub is_pro: bool,
}

impl From<AdminUserInfoRow> for AdminUserInfo {
    fn from(row: AdminUserInfoRow) -> Self {
        AdminUserInfo {
            id: row.id,
            username: row.username,
            email: row.email,
            display_name: row.display_name,
            avatar_url: row.avatar_url,
            role: row.role,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
            projects_count: row.projects_count,
            is_pro: row.is_pro,
        }
    }
}

/// GET /api/v1/admin/users/{id} - Get user detail as admin
pub async fn get_user(
    pool: web::Data<PgPool>,
    _admin: AdminUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let user_id = path.into_inner();
    
    let user = sqlx::query_as::<_, AdminUserInfoRow>(
        r#"SELECT u.id, u.username, u.email, u.display_name, u.avatar_url, 
                  u.role, u.is_active, u.created_at, u.updated_at, u.is_pro,
                  COALESCE((SELECT COUNT(*) FROM projects p WHERE p.owner_id = u.id), 0) as projects_count
           FROM users u WHERE u.id = $1"#
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("User not found".to_string()))?;

    Ok(HttpResponse::Ok().json(AdminUserInfo::from(user)))
}

/// PUT /api/v1/admin/users/{id} - Update user as admin
pub async fn update_user(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    admin: AdminUser,
    path: web::Path<i64>,
    body: web::Json<AdminUpdateUserRequest>,
) -> AppResult<HttpResponse> {
    let user_id = path.into_inner();

    // Prevent admin from demoting themselves
    if user_id == admin.user_id {
        if let Some(ref role) = body.role {
            if *role != UserRole::Admin {
                return Err(crate::error::AppError::BadRequest(
                    "Cannot change your own admin role".to_string(),
                ));
            }
        }
        if body.is_active == Some(false) {
            return Err(crate::error::AppError::BadRequest(
                "Cannot deactivate your own account".to_string(),
            ));
        }
    }

    let user = sqlx::query_as::<_, crate::models::User>(
        r#"
        UPDATE users
        SET role = COALESCE($2, role),
            is_active = COALESCE($3, is_active),
            display_name = COALESCE($4, display_name),
            email = COALESCE($5, email),
            is_pro = COALESCE($6, is_pro),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(&body.role)
    .bind(body.is_active)
    .bind(&body.display_name)
    .bind(&body.email)
    .bind(body.is_pro)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("User not found".to_string()))?;

    // If role was changed, set Redis invalidation flag for JWT refresh
    if body.role.is_some() {
        let role_changed_key = format!("user:{}:role_changed", user_id);
        if let Ok(mut conn) = redis.get().await {
            // Set flag with 24h expiration (longer than max JWT lifetime)
            let _ = deadpool_redis::redis::cmd("SETEX")
                .arg(&role_changed_key)
                .arg(86400) // 24 hours
                .arg("1")
                .query_async::<_, ()>(&mut conn)
                .await;
        }
    }

    Ok(HttpResponse::Ok().json(UserInfo::from(user)))
}

/// DELETE /api/v1/admin/users/{id} - Delete user as admin
pub async fn delete_user(
    pool: web::Data<PgPool>,
    admin: AdminUser,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    let user_id = path.into_inner();

    // Prevent admin from deleting themselves
    if user_id == admin.user_id {
        return Err(crate::error::AppError::BadRequest(
            "Cannot delete your own account".to_string(),
        ));
    }

    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool.get_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(crate::error::AppError::NotFound("User not found".to_string()));
    }

    Ok(HttpResponse::NoContent().finish())
}

/// Request to set pro status
#[derive(Debug, serde::Deserialize)]
pub struct SetProStatusRequest {
    pub is_pro: bool,
}

/// PUT /api/v1/admin/users/{id}/pro - Set user pro status
pub async fn set_pro_status(
    pool: web::Data<PgPool>,
    _admin: AdminUser,
    path: web::Path<i64>,
    body: web::Json<SetProStatusRequest>,
) -> AppResult<HttpResponse> {
    let user_id = path.into_inner();

    let user = sqlx::query_as::<_, crate::models::User>(
        r#"
        UPDATE users
        SET is_pro = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(user_id)
    .bind(body.is_pro)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| crate::error::AppError::NotFound("User not found".to_string()))?;

    Ok(HttpResponse::Ok().json(UserInfo::from(user)))
}

/// GET /api/v1/admin/settings/configs - Get all system configs
pub async fn get_configs(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    _admin: AdminUser,
) -> AppResult<HttpResponse> {
    let configs = SystemConfigService::get_all(pool.get_ref(), redis.get_ref()).await?;
    Ok(HttpResponse::Ok().json(configs))
}

/// PUT /api/v1/admin/settings/configs - Bulk update system configs
pub async fn update_configs(
    pool: web::Data<PgPool>,
    redis: web::Data<RedisPool>,
    _admin: AdminUser,
    body: web::Json<BulkUpdateConfigRequest>,
) -> AppResult<HttpResponse> {
    SystemConfigService::bulk_set(pool.get_ref(), redis.get_ref(), &body.configs).await?;
    let configs = SystemConfigService::get_all(pool.get_ref(), redis.get_ref()).await?;
    Ok(HttpResponse::Ok().json(configs))
}

// ─── SMTP Settings ─────────────────────────────────────────

use crate::config::AppConfig;
use crate::services::{SmtpService, SmtpSettings};

/// SMTP test request
#[derive(Debug, serde::Deserialize)]
pub struct SmtpTestRequest {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_tls: bool,
    pub use_ssl: bool,
    #[serde(default)]
    pub test_email: Option<String>,
}

/// SMTP status response
#[derive(Debug, serde::Serialize)]
pub struct SmtpStatusResponse {
    pub configured: bool,
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub from_email: String,
    pub from_name: String,
    pub use_tls: bool,
    pub use_ssl: bool,
}

/// GET /api/v1/admin/settings/smtp - Get SMTP configuration status
pub async fn get_smtp_config(
    config: web::Data<AppConfig>,
    _admin: AdminUser,
) -> AppResult<HttpResponse> {
    let settings = SmtpSettings::from_config(&config.smtp);
    
    Ok(HttpResponse::Ok().json(SmtpStatusResponse {
        configured: settings.is_configured(),
        enabled: settings.enabled,
        host: settings.host,
        port: settings.port,
        from_email: settings.from_email,
        from_name: settings.from_name,
        use_tls: settings.use_tls,
        use_ssl: settings.use_ssl,
    }))
}

/// POST /api/v1/admin/settings/smtp/test - Test SMTP connection
pub async fn test_smtp_connection(
    _admin: AdminUser,
    body: web::Json<SmtpTestRequest>,
) -> AppResult<HttpResponse> {
    let settings = SmtpSettings {
        enabled: body.enabled,
        host: body.host.clone(),
        port: body.port,
        username: body.username.clone(),
        password: body.password.clone(),
        from_email: body.from_email.clone(),
        from_name: body.from_name.clone(),
        use_tls: body.use_tls,
        use_ssl: body.use_ssl,
    };

    SmtpService::test_connection(&settings).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "SMTP connection successful"
    })))
}

/// POST /api/v1/admin/settings/smtp/send-test - Send test email
pub async fn send_test_email(
    _admin: AdminUser,
    body: web::Json<SmtpTestRequest>,
) -> AppResult<HttpResponse> {
    let test_email = body.test_email.as_ref()
        .ok_or_else(|| crate::error::AppError::BadRequest("test_email is required".to_string()))?;

    let settings = SmtpSettings {
        enabled: true, // Force enabled for test
        host: body.host.clone(),
        port: body.port,
        username: body.username.clone(),
        password: body.password.clone(),
        from_email: body.from_email.clone(),
        from_name: body.from_name.clone(),
        use_tls: body.use_tls,
        use_ssl: body.use_ssl,
    };

    SmtpService::send_test_email(&settings, test_email).await?;
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": format!("Test email sent to {}", test_email)
    })))
}
