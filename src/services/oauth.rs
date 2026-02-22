use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::error::{AppError, AppResult};
use crate::models::OAuthApplication;
use crate::config::Config;

pub struct OAuthService;

impl OAuthService {
    /// Auto-configure WebIDE OAuth2 application on startup if WebIDE is enabled
    /// Creates a trusted application with skip_authorization=true for seamless integration
    pub async fn auto_configure_webide_oauth(pool: &PgPool, config: &Config) -> AppResult<()> {
        // Check if webide is enabled
        let webide_enabled: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT value FROM system_configs WHERE key = 'webide_enabled'"
        )
        .fetch_optional(pool)
        .await?;

        let is_enabled = webide_enabled
            .and_then(|v| {
                // Try as bool first (for checkbox values), then as string
                v.as_bool().or_else(|| v.as_str().map(|s| s == "true"))
            })
            .unwrap_or(false);

        if !is_enabled {
            log::info!("WebIDE not enabled, skipping OAuth2 auto-configuration");
            return Ok(());
        }

        let base_url = config.base_url.trim_end_matches('/');
        let webide_client_id = &config.webide_client_id;
        let redirect_uri = format!("{}{}", base_url, config.webide_redirect_uri_path);

        // 用固定的 client_id 识别 WebIDE 应用
        let existing = sqlx::query_as::<_, OAuthApplication>(
            "SELECT * FROM oauth_applications WHERE uid = $1"
        )
        .bind(webide_client_id)
        .fetch_optional(pool)
        .await?;

        if let Some(app) = existing {
            // 检查 redirect_uri 和 scopes 是否需要更新
            let current_uris: Vec<String> = serde_json::from_value(app.redirect_uris.clone())
                .map_err(|e| AppError::InternalError(format!("Failed to parse redirect_uris: {}", e)))?;
            
            let current_scopes: Vec<String> = serde_json::from_value(app.scopes.clone())
                .map_err(|e| AppError::InternalError(format!("Failed to parse scopes: {}", e)))?;
            
            // 期望的 scopes
            let expected_scopes = vec![
                "openid".to_string(),
                "api".to_string(),
                "read_user".to_string(),
                "read_repository".to_string(),
                "write_repository".to_string(),
                "read_api".to_string(),
                "write_api".to_string(),
            ];
            
            let uris_match = current_uris.len() == 1 && current_uris[0] == redirect_uri;
            let scopes_match = {
                let mut current = current_scopes.clone();
                let mut expected = expected_scopes.clone();
                current.sort();
                expected.sort();
                current == expected
            };
            
            // 如果都匹配，无需更新
            if uris_match && scopes_match {
                log::info!("WebIDE OAuth2 application already up-to-date");
                return Ok(());
            }

            // 更新 redirect_uri 和 scopes
            let new_uris = vec![redirect_uri.clone()];
            let new_uris_json = serde_json::to_value(&new_uris)
                .map_err(|e| AppError::InternalError(format!("JSON serialization error: {}", e)))?;
            let new_scopes_json = serde_json::to_value(&expected_scopes)
                .map_err(|e| AppError::InternalError(format!("JSON serialization error: {}", e)))?;

            sqlx::query(
                "UPDATE oauth_applications SET redirect_uris = $1, scopes = $2, updated_at = NOW() WHERE id = $3"
            )
            .bind(&new_uris_json)
            .bind(&new_scopes_json)
            .bind(app.id)
            .execute(pool)
            .await?;

            log::info!("✓ Updated WebIDE OAuth2 application: redirect_uri={}, scopes={:?}", redirect_uri, expected_scopes);
            return Ok(());
        }

        log::info!("Creating WebIDE OAuth2 application...");

        // Find system admin user (first admin in database)
        let admin_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM users WHERE role = 'admin' ORDER BY created_at ASC LIMIT 1"
        )
        .fetch_optional(pool)
        .await?;

        let owner_id = admin_id.ok_or_else(|| {
            AppError::InternalError("No admin user found for WebIDE OAuth2 application".to_string())
        })?;

        // Generate client secret (client_id 从配置读取)
        let client_secret = Self::generate_client_secret();
        let client_secret_hash = Self::hash_secret(&client_secret);

        // WebIDE redirect URI (从配置读取)
        let redirect_uris = vec![redirect_uri.clone()];

        // Full API access for WebIDE
        let scopes = vec![
            "openid".to_string(),
            "api".to_string(),
            "read_user".to_string(),
            "read_repository".to_string(),
            "write_repository".to_string(),
            "read_api".to_string(),
            "write_api".to_string(),
        ];

        let redirect_uris_json = serde_json::to_value(&redirect_uris)
            .map_err(|e| AppError::InternalError(format!("JSON serialization error: {}", e)))?;
        let scopes_json = serde_json::to_value(&scopes)
            .map_err(|e| AppError::InternalError(format!("JSON serialization error: {}", e)))?;

        let app = sqlx::query_as::<_, OAuthApplication>(
            r#"
            INSERT INTO oauth_applications 
                (owner_id, name, uid, secret_hash, redirect_uris, scopes, 
                 description, homepage_url, confidential, trusted, skip_authorization, 
                 created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
            RETURNING *
            "#
        )
        .bind(owner_id)
        .bind("GitFox WebIDE")
        .bind(webide_client_id)
        .bind(&client_secret_hash)
        .bind(&redirect_uris_json)
        .bind(&scopes_json)
        .bind("GitFox 内置的 VS Code Web IDE，用于在线代码编辑")
        .bind(Some(format!("{}{}", base_url, config.webide_redirect_uri_path.trim_end_matches("/oauth/callback"))))
        .bind(false) // confidential: false (public client, uses PKCE)
        .bind(true)  // trusted: true (first-party application)
        .bind(true)  // skip_authorization: true (seamless login)
        .fetch_one(pool)
        .await?;

        log::info!(
            "✓ WebIDE OAuth2 application created: client_id={}, name={}",
            webide_client_id,
            app.name
        );
        log::info!("  Client Secret (save this, it won't be shown again): {}", client_secret);
        log::info!("  Redirect URIs: {:?}", redirect_uris);

        Ok(())
    }

    fn generate_client_secret() -> String {
        let mut rng = rand::thread_rng();
        let bytes: [u8; 32] = rng.gen();
        format!("gfcs_{}", hex::encode(bytes))
    }

    fn hash_secret(secret: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hex::encode(hasher.finalize())
    }
}
