use crate::error::{AppError, AppResult};
use crate::models::{RecoveryCodesResponse, TotpSetupResponse, TotpSetupState, UserRecoveryCode, UserTotp};
use crate::config::Config;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use deadpool_redis::Pool as RedisPool;
use qrcode::QrCode;
use qrcode::render::svg;
use rand::{thread_rng, Rng};
use sqlx::{PgPool, Row};
use totp_lite::{totp_custom, Sha1};
use webauthn_rs::prelude::*;

const TOTP_PERIOD: u64 = 30;
const TOTP_DIGITS: u32 = 6;
const RECOVERY_CODE_COUNT: usize = 10;
const RECOVERY_CODE_LENGTH: usize = 10;

/// Generate a new TOTP secret
pub fn generate_totp_secret() -> String {
    let mut rng = thread_rng();
    let secret: Vec<u8> = (0..20).map(|_| rng.gen()).collect();
    base32::encode(base32::Alphabet::RFC4648 { padding: false }, &secret)
}

/// Generate TOTP code from secret
pub fn generate_totp_code(secret: &str, time: u64) -> AppResult<String> {
    let secret_bytes = base32::decode(base32::Alphabet::RFC4648 { padding: false }, secret)
        .ok_or_else(|| AppError::BadRequest("Invalid TOTP secret".to_string()))?;
    
    let code = totp_custom::<Sha1>(TOTP_PERIOD, TOTP_DIGITS, &secret_bytes, time);
    Ok(format!("{:06}", code))
}

/// Verify TOTP code
pub fn verify_totp_code(secret: &str, code: &str) -> AppResult<bool> {
    let current_time = Utc::now().timestamp() as u64;
    
    // Check current period
    let current_code = generate_totp_code(secret, current_time)?;
    if current_code == code {
        return Ok(true);
    }
    
    // Check previous period (allow 30 second window)
    let prev_code = generate_totp_code(secret, current_time - TOTP_PERIOD)?;
    if prev_code == code {
        return Ok(true);
    }
    
    // Check next period (allow 30 second window)
    let next_code = generate_totp_code(secret, current_time + TOTP_PERIOD)?;
    if next_code == code {
        return Ok(true);
    }
    
    Ok(false)
}

/// Generate QR code for TOTP setup
pub fn generate_totp_qr_code(
    secret: &str,
    username: &str,
    issuer: &str,
) -> AppResult<String> {
    // Format: otpauth://totp/{issuer}:{username}?secret={secret}&issuer={issuer}
    let uri = format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        urlencoding::encode(issuer),
        urlencoding::encode(username),
        secret,
        urlencoding::encode(issuer),
        TOTP_DIGITS,
        TOTP_PERIOD
    );
    
    let code = QrCode::new(&uri)
        .map_err(|e| AppError::InternalError(format!("Failed to generate QR code: {}", e)))?;
    
    let svg_string = code.render::<svg::Color>()
        .min_dimensions(200, 200)
        .build();
    
    // Return as data URL
    use base64::{Engine as _, engine::general_purpose};
    Ok(format!("data:image/svg+xml;base64,{}", general_purpose::STANDARD.encode(&svg_string)))
}

/// Generate recovery codes
pub fn generate_recovery_codes() -> Vec<String> {
    let mut rng = thread_rng();
    let mut codes = Vec::new();
    
    for _ in 0..RECOVERY_CODE_COUNT {
        let code: String = (0..RECOVERY_CODE_LENGTH)
            .map(|_| {
                const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        codes.push(code);
    }
    
    codes
}

/// Hash a recovery code
pub fn hash_recovery_code(code: &str) -> AppResult<String> {
    hash(code, DEFAULT_COST)
        .map_err(|e| AppError::InternalError(format!("Failed to hash recovery code: {}", e)))
}

/// Verify a recovery code
pub fn verify_recovery_code(code: &str, hash: &str) -> AppResult<bool> {
    verify(code, hash)
        .map_err(|e| AppError::InternalError(format!("Failed to verify recovery code: {}", e)))
}

/// Setup TOTP for a user (store in Redis, not DB)
pub async fn setup_totp(
    pool: &PgPool,
    redis: &RedisPool,
    user_id: i64,
    username: &str,
) -> AppResult<TotpSetupResponse> {
    // Check if TOTP is already enabled
    let already_enabled = sqlx::query_scalar::<_, bool>(
        "SELECT enabled FROM user_totp WHERE user_id = $1 AND enabled = true"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(false);
    
    if already_enabled {
        return Err(AppError::BadRequest("TOTP is already enabled".to_string()));
    }
    
    // Generate new secret
    let secret = generate_totp_secret();
    
    // Generate QR code
    let qr_code = generate_totp_qr_code(&secret, username, "GitFox")?;
    
    // Generate backup codes
    let codes = generate_recovery_codes();
    
    // Store setup state in Redis (5 minute expiration, not in DB yet)
    let state_key = format!("totp:setup:{}:{}", user_id, Utc::now().timestamp());
    let state = TotpSetupState {
        secret: secret.clone(),
        recovery_codes: codes.clone(),
    };
    
    let state_json = serde_json::to_string(&state)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize TOTP state: {}", e)))?;
    
    let mut conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;
    
    redis::cmd("SETEX")
        .arg(&state_key)
        .arg(300) // 5 minutes
        .arg(&state_json)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to store TOTP state in Redis: {}", e)))?;
    
    Ok(TotpSetupResponse {
        state_key,
        secret,
        qr_code,
        backup_codes: codes,
    })
}

/// Enable TOTP after verifying code (read from Redis, write to DB)
pub async fn enable_totp(
    pool: &PgPool,
    redis: &RedisPool,
    user_id: i64,
    state_key: &str,
    code: &str,
) -> AppResult<()> {
    // Get setup state from Redis
    let mut conn = redis.get().await
        .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;
    
    let state_json: Option<String> = redis::cmd("GET")
        .arg(state_key)
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to read TOTP state from Redis: {}", e)))?;
    
    let state_json = state_json.ok_or_else(|| 
        AppError::BadRequest("TOTP setup session expired or not found. Please start setup again.".to_string())
    )?;
    
    let state: TotpSetupState = serde_json::from_str(&state_json)
        .map_err(|e| AppError::InternalError(format!("Failed to deserialize TOTP state: {}", e)))?;
    
    // Verify TOTP code
    if !verify_totp_code(&state.secret, code)? {
        return Err(AppError::Unauthorized("Invalid TOTP code".to_string()));
    }
    
    // Check if TOTP is already enabled (edge case: concurrent requests)
    let already_enabled = sqlx::query_scalar::<_, bool>(
        "SELECT enabled FROM user_totp WHERE user_id = $1 AND enabled = true"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(false);
    
    if already_enabled {
        return Err(AppError::BadRequest("TOTP is already enabled".to_string()));
    }
    
    // Delete any old unverified TOTP records (cleanup)
    sqlx::query("DELETE FROM user_totp WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    
    // Insert new TOTP record (enabled immediately)
    sqlx::query(
        "INSERT INTO user_totp (user_id, secret, enabled, verified_at, last_used_at) 
         VALUES ($1, $2, true, NOW(), NOW())"
    )
    .bind(user_id)
    .bind(&state.secret)
    .execute(pool)
    .await?;
    
    // Store recovery codes (hashed)
    for code in &state.recovery_codes {
        let code_hash = hash_recovery_code(code)?;
        sqlx::query(
            "INSERT INTO user_recovery_codes (user_id, code_hash) VALUES ($1, $2)"
        )
        .bind(user_id)
        .bind(&code_hash)
        .execute(pool)
        .await?;
    }
    
    // Enable 2FA for user
    sqlx::query("UPDATE users SET two_factor_enabled = true WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    
    // Delete Redis state (one-time use)
    redis::cmd("DEL")
        .arg(state_key)
        .query_async::<_, ()>(&mut conn)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to delete TOTP state from Redis: {}", e)))?;
    
    Ok(())
}

/// Disable TOTP
pub async fn disable_totp(
    pool: &PgPool,
    user_id: i64,
    code: &str,
) -> AppResult<()> {
    // Get TOTP secret
    let totp = sqlx::query_as::<_, UserTotp>(
        "SELECT * FROM user_totp WHERE user_id = $1 AND enabled = true"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("TOTP not enabled".to_string()))?;
    
    // Verify code
    if !verify_totp_code(&totp.secret, code)? {
        return Err(AppError::Unauthorized("Invalid TOTP code".to_string()));
    }
    
    // Delete TOTP
    sqlx::query("DELETE FROM user_totp WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    
    // Delete recovery codes
    sqlx::query("DELETE FROM user_recovery_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    
    // Check if any other 2FA methods are enabled
    let webauthn_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_webauthn_credentials WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    // Disable 2FA if no other methods are enabled
    if webauthn_count == 0 {
        sqlx::query("UPDATE users SET two_factor_enabled = false WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
    }
    
    Ok(())
}

/// Verify TOTP during login
pub async fn verify_totp_login(
    pool: &PgPool,
    user_id: i64,
    code: &str,
) -> AppResult<bool> {
    // Get TOTP secret
    let totp = sqlx::query_as::<_, UserTotp>(
        "SELECT * FROM user_totp WHERE user_id = $1 AND enabled = true"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("TOTP not enabled".to_string()))?;
    
    // Verify code
    let valid = verify_totp_code(&totp.secret, code)?;
    
    if valid {
        // Update last used timestamp
        sqlx::query("UPDATE user_totp SET last_used_at = NOW() WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
    }
    
    Ok(valid)
}

/// Use a recovery code
pub async fn use_recovery_code(
    pool: &PgPool,
    user_id: i64,
    code: &str,
) -> AppResult<bool> {
    // Get all unused recovery codes for the user
    let codes = sqlx::query_as::<_, UserRecoveryCode>(
        "SELECT * FROM user_recovery_codes WHERE user_id = $1 AND used = false"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    
    // Try to match the code
    for recovery_code in codes {
        if verify_recovery_code(code, &recovery_code.code_hash)? {
            // Mark as used
            sqlx::query(
                "UPDATE user_recovery_codes SET used = true, used_at = NOW() WHERE id = $1"
            )
            .bind(recovery_code.id)
            .execute(pool)
            .await?;
            
            return Ok(true);
        }
    }
    
    Ok(false)
}

/// Regenerate recovery codes
pub async fn regenerate_recovery_codes(
    pool: &PgPool,
    user_id: i64,
) -> AppResult<RecoveryCodesResponse> {
    // Delete old recovery codes
    sqlx::query("DELETE FROM user_recovery_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    
    // Generate new recovery codes
    let codes = generate_recovery_codes();
    
    // Store recovery codes (hashed)
    for code in &codes {
        let code_hash = hash_recovery_code(code)?;
        sqlx::query(
            "INSERT INTO user_recovery_codes (user_id, code_hash) VALUES ($1, $2)"
        )
        .bind(user_id)
        .bind(&code_hash)
        .execute(pool)
        .await?;
    }
    
    Ok(RecoveryCodesResponse { codes })
}

/// Create WebAuthn instance from config
pub fn create_webauthn(config: &Config) -> AppResult<Webauthn> {
    let rp_id = config.webauthn_rp_id.clone();
    let rp_origin = Url::parse(&config.webauthn_origin)
        .map_err(|e| AppError::InternalError(format!("Invalid WebAuthn origin URL: {}", e)))?;
    
    let builder = WebauthnBuilder::new(&rp_id, &rp_origin)
        .map_err(|e| AppError::InternalError(format!("Failed to create WebAuthn builder: {}", e)))?;
    
    let builder = builder.rp_name(&config.webauthn_rp_name);
    
    let webauthn = builder.build()
        .map_err(|e| AppError::InternalError(format!("Failed to build WebAuthn: {}", e)))?;
    
    Ok(webauthn)
}

/// Start WebAuthn registration
pub fn start_webauthn_registration(
    webauthn: &Webauthn,
    user_id: i64,
    username: &str,
    existing_credentials: Vec<CredentialID>,
) -> AppResult<(CreationChallengeResponse, PasskeyRegistration)> {
    // Convert i64 user_id to Uuid for WebAuthn
    // Use a deterministic UUID based on user_id
    let user_id_bytes = user_id.to_le_bytes();
    let mut uuid_bytes = [0u8; 16];
    uuid_bytes[0..8].copy_from_slice(&user_id_bytes);
    let user_unique_id = Uuid::from_bytes(uuid_bytes);
    
    let (ccr, reg_state) = webauthn
        .start_passkey_registration(
            user_unique_id,
            username,
            username,
            Some(existing_credentials),
        )
        .map_err(|e| AppError::InternalError(format!("Failed to start passkey registration: {}", e)))?;
    
    Ok((ccr, reg_state))
}

/// Finish WebAuthn registration
pub async fn finish_webauthn_registration(
    pool: &PgPool,
    webauthn: &Webauthn,
    user_id: i64,
    reg: &RegisterPublicKeyCredential,
    reg_state: &PasskeyRegistration,
    credential_name: &str,
) -> AppResult<Vec<String>> {
    let passkey = webauthn
        .finish_passkey_registration(reg, reg_state)
        .map_err(|e| AppError::BadRequest(format!("Failed to finish passkey registration: {}", e)))?;
    
    // Store credential in database
    let credential_id = passkey.cred_id().clone();
    let public_key = serde_json::to_vec(&passkey)
        .map_err(|e| AppError::InternalError(format!("Failed to serialize passkey: {}", e)))?;
    
    sqlx::query(
        "INSERT INTO user_webauthn_credentials (user_id, credential_id, public_key, counter, name) 
         VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(user_id)
    .bind(credential_id.as_slice())
    .bind(&public_key)
    .bind(0i64) // Initial counter
    .bind(credential_name)
    .execute(pool)
    .await?;
    
    // Enable 2FA for user
    sqlx::query("UPDATE users SET two_factor_enabled = true WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    
    // Check if user already has recovery codes
    let existing_codes_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_recovery_codes WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    // If no recovery codes exist, generate new ones
    let recovery_codes = if existing_codes_count == 0 {
        let codes = generate_recovery_codes();
        
        // Store recovery codes (hashed)
        for code in &codes {
            let code_hash = hash_recovery_code(code)?;
            sqlx::query(
                "INSERT INTO user_recovery_codes (user_id, code_hash) VALUES ($1, $2)"
            )
            .bind(user_id)
            .bind(&code_hash)
            .execute(pool)
            .await?;
        }
        
        codes
    } else {
        // Return empty vector if codes already exist (user won't see them again)
        Vec::new()
    };
    
    Ok(recovery_codes)
}

/// Get user's WebAuthn credentials
pub async fn get_user_passkeys(
    pool: &PgPool,
    user_id: i64,
) -> AppResult<Vec<Passkey>> {
    let rows = sqlx::query("SELECT public_key FROM user_webauthn_credentials WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await?;
    
    let mut passkeys = Vec::new();
    for row in rows {
        let public_key: Vec<u8> = row.try_get("public_key")?;
        let passkey: Passkey = serde_json::from_slice(&public_key)
            .map_err(|e| AppError::InternalError(format!("Failed to deserialize passkey: {}", e)))?;
        passkeys.push(passkey);
    }
    
    Ok(passkeys)
}

/// Get all WebAuthn credentials (for usernameless/discoverable login)
pub async fn get_all_passkeys(
    pool: &PgPool,
) -> AppResult<Vec<Passkey>> {
    let rows = sqlx::query("SELECT public_key FROM user_webauthn_credentials")
        .fetch_all(pool)
        .await?;
    
    let mut passkeys = Vec::new();
    for row in rows {
        let public_key: Vec<u8> = row.try_get("public_key")?;
        let passkey: Passkey = serde_json::from_slice(&public_key)
            .map_err(|e| AppError::InternalError(format!("Failed to deserialize passkey: {}", e)))?;
        passkeys.push(passkey);
    }
    
    Ok(passkeys)
}

/// Start WebAuthn authentication
pub fn start_webauthn_authentication(
    webauthn: &Webauthn,
    passkeys: Vec<Passkey>,
) -> AppResult<(RequestChallengeResponse, PasskeyAuthentication)> {
    let (rcr, auth_state) = webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|e| AppError::InternalError(format!("Failed to start passkey authentication: {}", e)))?;
    
    Ok((rcr, auth_state))
}

/// Finish WebAuthn authentication
pub async fn finish_webauthn_authentication(
    pool: &PgPool,
    webauthn: &Webauthn,
    user_id: i64,
    auth: &PublicKeyCredential,
    auth_state: &PasskeyAuthentication,
) -> AppResult<bool> {
    let auth_result = webauthn
        .finish_passkey_authentication(auth, auth_state)
        .map_err(|e| AppError::BadRequest(format!("Failed to finish passkey authentication: {}", e)))?;
    
    // Update counter in database
    let credential_id = auth_result.cred_id();
    let new_counter = auth_result.counter();
    
    sqlx::query(
        "UPDATE user_webauthn_credentials 
         SET counter = $1, last_used_at = NOW() 
         WHERE user_id = $2 AND credential_id = $3"
    )
    .bind(new_counter as i64)
    .bind(user_id)
    .bind(credential_id.as_slice())
    .execute(pool)
    .await?;
    
    Ok(true)
}

/// Finish WebAuthn authentication and return user_id (for direct Passkey login)
pub async fn finish_webauthn_authentication_and_get_user(
    pool: &PgPool,
    webauthn: &Webauthn,
    auth: &PublicKeyCredential,
    auth_state: &PasskeyAuthentication,
) -> AppResult<i64> {
    let auth_result = webauthn
        .finish_passkey_authentication(auth, auth_state)
        .map_err(|e| AppError::BadRequest(format!("Failed to finish passkey authentication: {}", e)))?;
    
    // Get user_id from credential_id
    let credential_id = auth_result.cred_id();
    let new_counter = auth_result.counter();
    
    let user_id: i64 = sqlx::query_scalar(
        "UPDATE user_webauthn_credentials 
         SET counter = $1, last_used_at = NOW() 
         WHERE credential_id = $2
         RETURNING user_id"
    )
    .bind(new_counter as i64)
    .bind(credential_id.as_slice())
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::Unauthorized("Invalid credential".to_string()))?;
    
    Ok(user_id)
}

/// Delete WebAuthn credential
pub async fn delete_webauthn_credential(
    pool: &PgPool,
    user_id: i64,
    credential_id: i64,
) -> AppResult<()> {
    sqlx::query(
        "DELETE FROM user_webauthn_credentials WHERE id = $1 AND user_id = $2"
    )
    .bind(credential_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    
    // Check if any other 2FA methods are enabled
    let totp_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_totp WHERE user_id = $1 AND enabled = true"
    )
    .fetch_one(pool)
    .await?;
    
    let webauthn_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_webauthn_credentials WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    // If no 2FA methods remain, disable 2FA and delete recovery codes
    if totp_count == 0 && webauthn_count == 0 {
        sqlx::query("UPDATE users SET two_factor_enabled = false WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
        
        // Delete all recovery codes (they're no longer needed)
        sqlx::query("DELETE FROM user_recovery_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
    }
    
    Ok(())
}
