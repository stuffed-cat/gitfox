use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::Rng;
use sqlx::PgPool;


use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::models::{Claims, CreateUserRequest, LoginRequest, LoginResponse, User, UserInfo, UserRole, TokenScope};

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
        let now = Utc::now();

        // Normal registration always creates Developer users.
        // Initial admin is seeded on server startup via INITIAL_ADMIN_* env vars.
        let role = UserRole::Developer;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, display_name, role, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, true, $6, $6)
            RETURNING *
            "#
        )
        .bind(&req.username)
        .bind(&req.email)
        .bind(&password_hash)
        .bind(&req.display_name)
        .bind(role)
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
            scopes: TokenScope::Full, // JWT has full access
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

    pub async fn get_user_by_id(pool: &PgPool, id: i64) -> AppResult<User> {
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
        id: i64,
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

    pub async fn update_user_profile(
        pool: &PgPool,
        id: i64,
        display_name: Option<String>,
        avatar_url: Option<String>,
        status_emoji: Option<String>,
        status_message: Option<String>,
        busy: Option<bool>,
        clear_status_after: Option<String>,
    ) -> AppResult<User> {
        // Calculate status_clear_at based on clear_status_after value
        let status_clear_at = match clear_status_after.as_deref() {
            Some("never") | None => None,
            Some("30m") => Some(Utc::now() + Duration::minutes(30)),
            Some("1h") => Some(Utc::now() + Duration::hours(1)),
            Some("4h") => Some(Utc::now() + Duration::hours(4)),
            Some("today") => {
                let now = Utc::now();
                // Calculate end of today in UTC
                let next_day = now + Duration::days(1);
                Some(next_day.date_naive().and_hms_opt(0, 0, 0)
                    .unwrap_or_default()
                    .and_utc())
            },
            Some("1w") => Some(Utc::now() + Duration::days(7)),
            _ => None,
        };

        let now = Utc::now();
        let status_was_updated = status_message.is_some() || status_emoji.is_some();
        
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET display_name = COALESCE($2, display_name),
                avatar_url = COALESCE($3, avatar_url),
                status_emoji = COALESCE($4, status_emoji),
                status_message = COALESCE($5, status_message),
                busy = COALESCE($6, busy),
                status_set_at = CASE WHEN $7::BOOLEAN THEN NOW() ELSE status_set_at END,
                status_clear_at = COALESCE($8::TIMESTAMPTZ, status_clear_at),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .bind(display_name)
        .bind(avatar_url)
        .bind(status_emoji)
        .bind(status_message)
        .bind(busy)
        .bind(status_was_updated)
        .bind(status_clear_at)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        Ok(user)
    }

    pub async fn change_password(
        pool: &PgPool,
        id: i64,
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

    pub async fn delete_user(pool: &PgPool, id: i64) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("User not found".to_string()));
        }

        Ok(())
    }

    /// Seed the initial admin user on server startup.
    /// Only creates the admin if:
    /// 1. No admin user exists in the database
    /// 2. INITIAL_ADMIN_USERNAME, INITIAL_ADMIN_EMAIL, INITIAL_ADMIN_PASSWORD are all set
    /// This is idempotent — once an admin exists, it does nothing.
    pub async fn seed_initial_admin(pool: &PgPool, config: &crate::config::AppConfig) -> AppResult<()> {
        // Check if any admin already exists
        let admin_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE role = 'admin')"
        )
        .fetch_one(pool)
        .await?;

        if admin_exists {
            log::debug!("Admin user already exists, skipping seed");
            return Ok(());
        }

        // All three env vars must be set
        let (username, email, password) = match (
            &config.initial_admin_username,
            &config.initial_admin_email,
            &config.initial_admin_password,
        ) {
            (Some(u), Some(e), Some(p)) => (u.clone(), e.clone(), p.clone()),
            _ => {
                log::warn!(
                    "No admin user exists and INITIAL_ADMIN_USERNAME/EMAIL/PASSWORD are not all set. \
                     Please set these environment variables and restart, or create an admin via the API."
                );
                return Ok(());
            }
        };

        // Check if username or email already taken (as non-admin)
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE username = $1 OR email = $2"
        )
        .bind(&username)
        .bind(&email)
        .fetch_one(pool)
        .await?;

        if existing > 0 {
            // User exists but is not admin — promote them
            sqlx::query("UPDATE users SET role = 'admin', updated_at = NOW() WHERE username = $1 OR email = $2")
                .bind(&username)
                .bind(&email)
                .execute(pool)
                .await?;
            log::info!("Promoted existing user '{}' to admin", username);
            return Ok(());
        }

        // Create the admin user
        let password_hash = hash(&password, DEFAULT_COST)?;
        let now = Utc::now();

        sqlx::query(
            r#"INSERT INTO users (username, email, password_hash, display_name, role, is_active, created_at, updated_at)
               VALUES ($1, $2, $3, $4, 'admin', true, $5, $5)"#
        )
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(&username)
        .bind(now)
        .execute(pool)
        .await?;

        log::info!("Created initial admin user '{}'", username);
        Ok(())
    }

    // ─── Email Confirmation ────────────────────────────────

    /// Generate a secure random token
    fn generate_token() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        hex::encode(bytes)
    }

    /// Generate email confirmation token for a user
    pub async fn generate_email_confirmation_token(pool: &PgPool, user_id: i64) -> AppResult<String> {
        let token = Self::generate_token();
        let now = Utc::now();

        sqlx::query(
            r#"UPDATE users 
               SET email_confirmation_token = $2, 
                   email_confirmation_sent_at = $3,
                   updated_at = $3
               WHERE id = $1"#
        )
        .bind(user_id)
        .bind(&token)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(token)
    }

    /// Confirm email with token
    pub async fn confirm_email(pool: &PgPool, token: &str) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email_confirmation_token = $1"
        )
        .bind(token)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired confirmation token".to_string()))?;

        // Check if token is expired (24 hours)
        if let Some(sent_at) = user.email_confirmation_sent_at {
            if Utc::now() - sent_at > Duration::hours(24) {
                return Err(AppError::BadRequest("Confirmation token has expired".to_string()));
            }
        }

        let updated_user = sqlx::query_as::<_, User>(
            r#"UPDATE users 
               SET email_confirmed = true, 
                   email_confirmation_token = NULL,
                   updated_at = NOW()
               WHERE id = $1
               RETURNING *"#
        )
        .bind(user.id)
        .fetch_one(pool)
        .await?;

        Ok(updated_user)
    }

    /// Resend email confirmation token
    pub async fn resend_email_confirmation(pool: &PgPool, user_id: i64) -> AppResult<String> {
        let user = Self::get_user_by_id(pool, user_id).await?;
        
        if user.email_confirmed {
            return Err(AppError::BadRequest("Email is already confirmed".to_string()));
        }

        // Check rate limiting (minimum 1 minute between sends)
        if let Some(sent_at) = user.email_confirmation_sent_at {
            if Utc::now() - sent_at < Duration::minutes(1) {
                return Err(AppError::BadRequest("Please wait before requesting another confirmation email".to_string()));
            }
        }

        Self::generate_email_confirmation_token(pool, user_id).await
    }

    // ─── Password Reset ────────────────────────────────────

    /// Generate password reset token for a user by email
    pub async fn generate_password_reset_token(pool: &PgPool, email: &str) -> AppResult<(User, String)> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1 AND is_active = true"
        )
        .bind(email)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("No active user found with this email".to_string()))?;

        // Check rate limiting (minimum 1 minute between sends)
        if let Some(sent_at) = user.password_reset_sent_at {
            if Utc::now() - sent_at < Duration::minutes(1) {
                return Err(AppError::BadRequest("Please wait before requesting another password reset".to_string()));
            }
        }

        let token = Self::generate_token();
        let now = Utc::now();

        sqlx::query(
            r#"UPDATE users 
               SET password_reset_token = $2, 
                   password_reset_sent_at = $3,
                   updated_at = $3
               WHERE id = $1"#
        )
        .bind(user.id)
        .bind(&token)
        .bind(now)
        .execute(pool)
        .await?;

        Ok((user, token))
    }

    /// Verify password reset token and return user
    pub async fn verify_password_reset_token(pool: &PgPool, token: &str) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE password_reset_token = $1"
        )
        .bind(token)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid or expired reset token".to_string()))?;

        // Check if token is expired (24 hours)
        if let Some(sent_at) = user.password_reset_sent_at {
            if Utc::now() - sent_at > Duration::hours(24) {
                return Err(AppError::BadRequest("Password reset token has expired".to_string()));
            }
        }

        Ok(user)
    }

    /// Reset password with token
    pub async fn reset_password(pool: &PgPool, token: &str, new_password: &str) -> AppResult<User> {
        let user = Self::verify_password_reset_token(pool, token).await?;

        let new_hash = hash(new_password, DEFAULT_COST)?;

        let updated_user = sqlx::query_as::<_, User>(
            r#"UPDATE users 
               SET password_hash = $2, 
                   password_reset_token = NULL,
                   updated_at = NOW()
               WHERE id = $1
               RETURNING *"#
        )
        .bind(user.id)
        .bind(new_hash)
        .fetch_one(pool)
        .await?;

        Ok(updated_user)
    }
}
