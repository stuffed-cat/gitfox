use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::models::{Claims, CreateUserRequest, LoginRequest, LoginResponse, User, UserInfo, UserRole};

pub struct UserService;

impl UserService {
    pub async fn create_user(pool: &PgPool, req: CreateUserRequest) -> AppResult<User> {
        // Check if user exists
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE username = $1 OR email = $2"
        )
        .bind(&req.username)
        .bind(&req.email)
        .fetch_one(pool)
        .await?;

        if existing > 0 {
            return Err(AppError::Conflict("Username or email already exists".to_string()));
        }

        let password_hash = hash(&req.password, DEFAULT_COST)?;
        let id = Uuid::new_v4();
        let now = Utc::now();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, password_hash, display_name, role, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, true, $7, $7)
            RETURNING *
            "#
        )
        .bind(id)
        .bind(&req.username)
        .bind(&req.email)
        .bind(&password_hash)
        .bind(&req.display_name)
        .bind(UserRole::Developer)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn login(pool: &PgPool, config: &AppConfig, req: LoginRequest) -> AppResult<LoginResponse> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1 AND is_active = true"
        )
        .bind(&req.username)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

        if !verify(&req.password, &user.password_hash)? {
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }

        let now = Utc::now();
        let exp = now + Duration::seconds(config.jwt_expiration);

        let claims = Claims {
            sub: user.username.clone(),
            user_id: user.id,
            role: user.role.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
        )?;

        Ok(LoginResponse {
            token,
            user: UserInfo::from(user),
        })
    }

    pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> AppResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    pub async fn get_user_by_username(pool: &PgPool, username: &str) -> AppResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    pub async fn list_users(pool: &PgPool, page: u32, per_page: u32) -> AppResult<Vec<User>> {
        let offset = (page.saturating_sub(1)) * per_page;

        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    pub async fn update_user(
        pool: &PgPool,
        id: Uuid,
        display_name: Option<String>,
        avatar_url: Option<String>,
    ) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET display_name = COALESCE($2, display_name),
                avatar_url = COALESCE($3, avatar_url),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .bind(display_name)
        .bind(avatar_url)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(user)
    }

    pub async fn change_password(
        pool: &PgPool,
        id: Uuid,
        old_password: &str,
        new_password: &str,
    ) -> AppResult<()> {
        let user = Self::get_user_by_id(pool, id).await?;

        if !verify(old_password, &user.password_hash)? {
            return Err(AppError::Unauthorized("Invalid old password".to_string()));
        }

        let new_hash = hash(new_password, DEFAULT_COST)?;

        sqlx::query("UPDATE users SET password_hash = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(new_hash)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn delete_user(pool: &PgPool, id: Uuid) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("User not found".to_string()));
        }

        Ok(())
    }
}
