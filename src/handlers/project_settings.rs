//! Project settings handlers
//!
//! Handlers for project-level settings:
//! - Branch protection rules
//! - CI/CD variables
//! - Pipeline triggers
//! - Deploy keys
//! - Project access tokens

use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use sha2::{Sha256, Digest};
use rand::Rng;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

use crate::error::{AppError, AppResult};
use crate::middleware::AuthenticatedUser;
use crate::services::ProjectService;
use crate::models::project_settings::*;

// =========================================
// Branch Protection Rules
// =========================================

/// List all branch protection rules for a project
pub async fn list_branch_protections(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    // Check access (at least reporter)
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "reporter").await?;
    
    let rules = sqlx::query_as::<_, BranchProtectionRule>(
        "SELECT * FROM branch_protection_rules WHERE project_id = $1 ORDER BY branch_pattern"
    )
    .bind(project.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(rules))
}

/// Create a branch protection rule
pub async fn create_branch_protection(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
    body: web::Json<CreateBranchProtectionRequest>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    // Check access (maintainer or owner)
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    // Check if pattern already exists
    let existing = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM branch_protection_rules WHERE project_id = $1 AND branch_pattern = $2)"
    )
    .bind(project.id)
    .bind(&body.branch_pattern)
    .fetch_one(pool.get_ref())
    .await?;
    
    if existing {
        return Err(AppError::Conflict(format!(
            "Branch protection rule for pattern '{}' already exists",
            body.branch_pattern
        )));
    }
    
    let rule = sqlx::query_as::<_, BranchProtectionRule>(
        r#"
        INSERT INTO branch_protection_rules 
        (project_id, branch_pattern, require_review, required_reviewers, require_ci_pass, allow_force_push, allow_deletion)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(&body.branch_pattern)
    .bind(body.require_review)
    .bind(body.required_reviewers)
    .bind(body.require_ci_pass)
    .bind(body.allow_force_push)
    .bind(body.allow_deletion)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(rule))
}

/// Update a branch protection rule
pub async fn update_branch_protection(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
    body: web::Json<UpdateBranchProtectionRequest>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, rule_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    // Verify rule exists and belongs to project
    let mut rule = sqlx::query_as::<_, BranchProtectionRule>(
        "SELECT * FROM branch_protection_rules WHERE id = $1 AND project_id = $2"
    )
    .bind(rule_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Branch protection rule not found".to_string()))?;
    
    // Apply updates
    if let Some(v) = body.require_review { rule.require_review = v; }
    if let Some(v) = body.required_reviewers { rule.required_reviewers = v; }
    if let Some(v) = body.require_ci_pass { rule.require_ci_pass = v; }
    if let Some(v) = body.allow_force_push { rule.allow_force_push = v; }
    if let Some(v) = body.allow_deletion { rule.allow_deletion = v; }
    
    let updated = sqlx::query_as::<_, BranchProtectionRule>(
        r#"
        UPDATE branch_protection_rules
        SET require_review = $1, required_reviewers = $2, require_ci_pass = $3, 
            allow_force_push = $4, allow_deletion = $5
        WHERE id = $6
        RETURNING *
        "#
    )
    .bind(rule.require_review)
    .bind(rule.required_reviewers)
    .bind(rule.require_ci_pass)
    .bind(rule.allow_force_push)
    .bind(rule.allow_deletion)
    .bind(rule_id)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(updated))
}

/// Delete a branch protection rule
pub async fn delete_branch_protection(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, rule_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let deleted = sqlx::query(
        "DELETE FROM branch_protection_rules WHERE id = $1 AND project_id = $2"
    )
    .bind(rule_id)
    .bind(project.id)
    .execute(pool.get_ref())
    .await?;
    
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("Branch protection rule not found".to_string()));
    }
    
    Ok(HttpResponse::NoContent().finish())
}

// =========================================
// CI/CD Variables
// =========================================

/// Encryption key derivation (in production, use proper key management)
fn get_encryption_key(secret: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b"ci_variable_key");
    hasher.finalize().into()
}

/// Simple AES-256-GCM encryption
fn encrypt_value(value: &str, key: &[u8; 32]) -> Result<String, AppError> {
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use aes_gcm::aead::{Aead, KeyInit};
    
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, value.as_bytes())
        .map_err(|e| AppError::InternalError(format!("Encryption failed: {}", e)))?;
    
    // Format: base64(nonce || ciphertext)
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    
    Ok(URL_SAFE_NO_PAD.encode(&combined))
}

/// Simple AES-256-GCM decryption
fn decrypt_value(encrypted: &str, key: &[u8; 32]) -> Result<String, AppError> {
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use aes_gcm::aead::{Aead, KeyInit};
    
    let combined = URL_SAFE_NO_PAD.decode(encrypted)
        .map_err(|e| AppError::InternalError(format!("Invalid encrypted data: {}", e)))?;
    
    if combined.len() < 12 {
        return Err(AppError::InternalError("Invalid encrypted data length".to_string()));
    }
    
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| AppError::InternalError(format!("Decryption failed: {}", e)))?;
    
    String::from_utf8(plaintext)
        .map_err(|e| AppError::InternalError(format!("Invalid UTF-8: {}", e)))
}

/// List all CI variables for a project
pub async fn list_ci_variables(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    // Maintainer or owner can see variables
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let variables = sqlx::query_as::<_, CiVariable>(
        "SELECT * FROM ci_variables WHERE project_id = $1 ORDER BY key"
    )
    .bind(project.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    let response: Vec<CiVariableResponse> = variables.into_iter()
        .map(Into::into)
        .collect();
    
    Ok(HttpResponse::Ok().json(response))
}

/// Create a CI variable
pub async fn create_ci_variable(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
    body: web::Json<CreateCiVariableRequest>,
    config: web::Data<crate::config::Config>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    // Validate key format
    if !crate::models::project_settings::validators::CI_VAR_KEY_REGEX.is_match(&body.key) {
        return Err(AppError::BadRequest(
            "Variable key must start with uppercase letter and contain only uppercase letters, numbers, and underscores".to_string()
        ));
    }
    
    // Check for duplicate
    let existing = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM ci_variables WHERE project_id = $1 AND key = $2 AND environment_scope = $3)"
    )
    .bind(project.id)
    .bind(&body.key)
    .bind(&body.environment_scope)
    .fetch_one(pool.get_ref())
    .await?;
    
    if existing {
        return Err(AppError::Conflict(format!(
            "Variable '{}' already exists for scope '{}'",
            body.key, body.environment_scope
        )));
    }
    
    // Encrypt value
    let key = get_encryption_key(&config.jwt_secret);
    let encrypted_value = encrypt_value(&body.value, &key)?;
    
    let variable = sqlx::query_as::<_, CiVariable>(
        r#"
        INSERT INTO ci_variables 
        (project_id, key, value_encrypted, protected, masked, file, environment_scope)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(&body.key)
    .bind(&encrypted_value)
    .bind(body.protected)
    .bind(body.masked)
    .bind(body.file)
    .bind(&body.environment_scope)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(CiVariableResponse::from(variable)))
}

/// Update a CI variable
pub async fn update_ci_variable(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
    body: web::Json<UpdateCiVariableRequest>,
    config: web::Data<crate::config::Config>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, var_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    // Get existing variable
    let existing = sqlx::query_as::<_, CiVariable>(
        "SELECT * FROM ci_variables WHERE id = $1 AND project_id = $2"
    )
    .bind(var_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("CI variable not found".to_string()))?;
    
    // Prepare updated values
    let encrypted_value = if let Some(ref value) = body.value {
        let key = get_encryption_key(&config.jwt_secret);
        encrypt_value(value, &key)?
    } else {
        existing.value_encrypted.clone()
    };
    
    let variable = sqlx::query_as::<_, CiVariable>(
        r#"
        UPDATE ci_variables
        SET value_encrypted = $1, protected = $2, masked = $3, file = $4, 
            environment_scope = $5, updated_at = NOW()
        WHERE id = $6
        RETURNING *
        "#
    )
    .bind(&encrypted_value)
    .bind(body.protected.unwrap_or(existing.protected))
    .bind(body.masked.unwrap_or(existing.masked))
    .bind(body.file.unwrap_or(existing.file))
    .bind(body.environment_scope.as_ref().unwrap_or(&existing.environment_scope))
    .bind(var_id)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(CiVariableResponse::from(variable)))
}

/// Delete a CI variable
pub async fn delete_ci_variable(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, var_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let deleted = sqlx::query(
        "DELETE FROM ci_variables WHERE id = $1 AND project_id = $2"
    )
    .bind(var_id)
    .bind(project.id)
    .execute(pool.get_ref())
    .await?;
    
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("CI variable not found".to_string()));
    }
    
    Ok(HttpResponse::NoContent().finish())
}

// =========================================
// Pipeline Triggers
// =========================================

/// Generate a secure random token
fn generate_trigger_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 24] = rng.gen();
    format!("glptt-{}", URL_SAFE_NO_PAD.encode(&bytes))
}

/// List all pipeline triggers for a project
pub async fn list_pipeline_triggers(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let triggers = sqlx::query_as::<_, PipelineTrigger>(
        "SELECT * FROM pipeline_triggers WHERE project_id = $1 ORDER BY created_at DESC"
    )
    .bind(project.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(triggers))
}

/// Create a pipeline trigger
pub async fn create_pipeline_trigger(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
    body: web::Json<CreatePipelineTriggerRequest>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    // Generate token
    let token = generate_trigger_token();
    let token_preview = token.chars().take(12).collect::<String>() + "...";
    
    // Hash token for storage
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = hex::encode(hasher.finalize());
    
    let trigger = sqlx::query_as::<_, PipelineTrigger>(
        r#"
        INSERT INTO pipeline_triggers (project_id, description, token_hash, token_preview, created_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(&body.description)
    .bind(&token_hash)
    .bind(&token_preview)
    .bind(auth.user_id)
    .fetch_one(pool.get_ref())
    .await?;
    
    // Return full token only at creation time
    Ok(HttpResponse::Created().json(CreatePipelineTriggerResponse {
        id: trigger.id,
        description: trigger.description,
        token, // Full token, only shown once
        token_preview: trigger.token_preview,
        created_at: trigger.created_at,
    }))
}

/// Delete a pipeline trigger
pub async fn delete_pipeline_trigger(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, trigger_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let deleted = sqlx::query(
        "DELETE FROM pipeline_triggers WHERE id = $1 AND project_id = $2"
    )
    .bind(trigger_id)
    .bind(project.id)
    .execute(pool.get_ref())
    .await?;
    
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("Pipeline trigger not found".to_string()));
    }
    
    Ok(HttpResponse::NoContent().finish())
}

// =========================================
// Deploy Keys
// =========================================

/// Parse SSH public key and extract type and fingerprint
fn parse_ssh_key(key: &str) -> Result<(String, String, String), AppError> {
    let parts: Vec<&str> = key.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(AppError::BadRequest("Invalid SSH key format".to_string()));
    }
    
    let key_type = parts[0].to_string();
    let key_data = parts[1];
    
    // Validate key type
    if !matches!(key_type.as_str(), "ssh-rsa" | "ssh-ed25519" | "ecdsa-sha2-nistp256" | "ecdsa-sha2-nistp384" | "ecdsa-sha2-nistp521") {
        return Err(AppError::BadRequest(format!("Unsupported key type: {}", key_type)));
    }
    
    // Decode and hash for fingerprint
    let decoded = base64::engine::general_purpose::STANDARD.decode(key_data)
        .map_err(|_| AppError::BadRequest("Invalid SSH key data".to_string()))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&decoded);
    let fingerprint = format!("SHA256:{}", URL_SAFE_NO_PAD.encode(&hasher.finalize()));
    
    Ok((key_type, key.to_string(), fingerprint))
}

/// List all deploy keys for a project
pub async fn list_deploy_keys(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let keys = sqlx::query_as::<_, DeployKey>(
        "SELECT * FROM deploy_keys WHERE project_id = $1 ORDER BY created_at DESC"
    )
    .bind(project.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(keys))
}

/// Create a deploy key
pub async fn create_deploy_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
    body: web::Json<CreateDeployKeyRequest>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    // Parse and validate SSH key
    let (key_type, public_key, fingerprint) = parse_ssh_key(&body.key)?;
    
    // Check for duplicate fingerprint in this project
    let existing = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM deploy_keys WHERE project_id = $1 AND fingerprint = $2)"
    )
    .bind(project.id)
    .bind(&fingerprint)
    .fetch_one(pool.get_ref())
    .await?;
    
    if existing {
        return Err(AppError::Conflict("This deploy key already exists in this project".to_string()));
    }
    
    let deploy_key = sqlx::query_as::<_, DeployKey>(
        r#"
        INSERT INTO deploy_keys (project_id, title, key_type, public_key, fingerprint, can_push, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(&body.title)
    .bind(&key_type)
    .bind(&public_key)
    .bind(&fingerprint)
    .bind(body.can_push)
    .bind(auth.user_id)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Created().json(deploy_key))
}

/// Update a deploy key
pub async fn update_deploy_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
    body: web::Json<UpdateDeployKeyRequest>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, key_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let existing = sqlx::query_as::<_, DeployKey>(
        "SELECT * FROM deploy_keys WHERE id = $1 AND project_id = $2"
    )
    .bind(key_id)
    .bind(project.id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Deploy key not found".to_string()))?;
    
    let deploy_key = sqlx::query_as::<_, DeployKey>(
        r#"
        UPDATE deploy_keys
        SET title = $1, can_push = $2
        WHERE id = $3
        RETURNING *
        "#
    )
    .bind(body.title.as_ref().unwrap_or(&existing.title))
    .bind(body.can_push.unwrap_or(existing.can_push))
    .bind(key_id)
    .fetch_one(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(deploy_key))
}

/// Delete a deploy key
pub async fn delete_deploy_key(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, key_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let deleted = sqlx::query(
        "DELETE FROM deploy_keys WHERE id = $1 AND project_id = $2"
    )
    .bind(key_id)
    .bind(project.id)
    .execute(pool.get_ref())
    .await?;
    
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("Deploy key not found".to_string()));
    }
    
    Ok(HttpResponse::NoContent().finish())
}

// =========================================
// Project Access Tokens
// =========================================

/// Generate a project access token
fn generate_project_access_token() -> String {
    use crate::models::personal_access_token::PAT_PREFIX;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 20] = rng.gen();
    format!("{}{}", PAT_PREFIX, URL_SAFE_NO_PAD.encode(&bytes))
}

/// List all project access tokens
pub async fn list_project_access_tokens(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let tokens = sqlx::query_as::<_, ProjectAccessToken>(
        "SELECT * FROM project_access_tokens WHERE project_id = $1 AND is_active = true ORDER BY created_at DESC"
    )
    .bind(project.id)
    .fetch_all(pool.get_ref())
    .await?;
    
    Ok(HttpResponse::Ok().json(tokens))
}

/// Create a project access token
pub async fn create_project_access_token(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String)>,
    body: web::Json<CreateProjectAccessTokenRequest>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    // Validate role
    if !matches!(body.role.as_str(), "maintainer" | "developer" | "reporter") {
        return Err(AppError::BadRequest("Invalid role. Must be maintainer, developer, or reporter".to_string()));
    }
    
    // Generate token
    let token = generate_project_access_token();
    let token_preview = token.chars().take(16).collect::<String>() + "...";
    
    // Hash token for storage
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = hex::encode(hasher.finalize());
    
    let access_token = sqlx::query_as::<_, ProjectAccessToken>(
        r#"
        INSERT INTO project_access_tokens 
        (project_id, name, token_hash, token_preview, scopes, role, expires_at, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(&body.name)
    .bind(&token_hash)
    .bind(&token_preview)
    .bind(&body.scopes)
    .bind(&body.role)
    .bind(&body.expires_at)
    .bind(auth.user_id)
    .fetch_one(pool.get_ref())
    .await?;
    
    // Return full token only at creation time
    Ok(HttpResponse::Created().json(CreateProjectAccessTokenResponse {
        id: access_token.id,
        name: access_token.name,
        token, // Full token, only shown once
        scopes: access_token.scopes,
        role: access_token.role,
        expires_at: access_token.expires_at,
        created_at: access_token.created_at,
    }))
}

/// Revoke a project access token
pub async fn revoke_project_access_token(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    path: web::Path<(String, String, i64)>,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, token_id) = path.into_inner();
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;
    
    ProjectService::check_project_access(pool.get_ref(), project.id, auth.user_id, "maintainer").await?;
    
    let updated = sqlx::query(
        "UPDATE project_access_tokens SET is_active = false WHERE id = $1 AND project_id = $2 AND is_active = true"
    )
    .bind(token_id)
    .bind(project.id)
    .execute(pool.get_ref())
    .await?;
    
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("Project access token not found".to_string()));
    }
    
    Ok(HttpResponse::NoContent().finish())
}
