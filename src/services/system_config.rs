use deadpool_redis::Pool as RedisPool;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::error::{AppError, AppResult};

const REDIS_CONFIG_PREFIX: &str = "gitfox:config:";
const REDIS_CONFIG_TTL: i64 = 300; // 5 minutes

/// A single system configuration entry.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SystemConfig {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Update request for a single config entry.
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub key: String,
    pub value: serde_json::Value,
}

/// Bulk update request.
#[derive(Debug, Deserialize)]
pub struct BulkUpdateConfigRequest {
    pub configs: Vec<UpdateConfigRequest>,
}

pub struct SystemConfigService;

impl SystemConfigService {
    // ─── Read ──────────────────────────────────────────────

    /// Get a single config value. Redis first, fallback to PG.
    pub async fn get(
        pool: &PgPool,
        redis: &RedisPool,
        key: &str,
    ) -> AppResult<serde_json::Value> {
        // Try Redis first
        if let Ok(mut conn) = redis.get().await {
            let cache_key = format!("{}{}", REDIS_CONFIG_PREFIX, key);
            if let Ok(cached) = conn.get::<_, String>(&cache_key).await {
                if let Ok(val) = serde_json::from_str(&cached) {
                    return Ok(val);
                }
            }
        }

        // Fallback to database
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT value FROM system_configs WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Config key '{}' not found", key)))?;

        // Write-back to Redis
        Self::cache_set(redis, key, &row).await;

        Ok(row)
    }

    /// Get all config entries as a map.
    pub async fn get_all(
        pool: &PgPool,
        _redis: &RedisPool,
    ) -> AppResult<HashMap<String, serde_json::Value>> {
        let rows = sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_configs")
            .fetch_all(pool)
            .await?;

        let map: HashMap<String, serde_json::Value> =
            rows.into_iter().map(|r| (r.key, r.value)).collect();

        Ok(map)
    }

    // ─── Write ─────────────────────────────────────────────

    /// Set a single config value. Updates DB then invalidates Redis cache.
    pub async fn set(
        pool: &PgPool,
        redis: &RedisPool,
        key: &str,
        value: &serde_json::Value,
    ) -> AppResult<()> {
        sqlx::query(
            r#"INSERT INTO system_configs (key, value, updated_at)
               VALUES ($1, $2, NOW())
               ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()"#,
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;

        // Update Redis cache
        Self::cache_set(redis, key, value).await;

        Ok(())
    }

    /// Bulk update multiple config entries in a single transaction.
    pub async fn bulk_set(
        pool: &PgPool,
        redis: &RedisPool,
        configs: &[UpdateConfigRequest],
    ) -> AppResult<()> {
        let mut tx = pool.begin().await?;

        for item in configs {
            sqlx::query(
                r#"INSERT INTO system_configs (key, value, updated_at)
                   VALUES ($1, $2, NOW())
                   ON CONFLICT (key) DO UPDATE SET value = $2, updated_at = NOW()"#,
            )
            .bind(&item.key)
            .bind(&item.value)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Invalidate all changed keys in Redis
        for item in configs {
            Self::cache_set(redis, &item.key, &item.value).await;
        }

        Ok(())
    }

    // ─── Cache helpers ─────────────────────────────────────

    async fn cache_set(redis: &RedisPool, key: &str, value: &serde_json::Value) {
        if let Ok(mut conn) = redis.get().await {
            let cache_key = format!("{}{}", REDIS_CONFIG_PREFIX, key);
            if let Ok(serialized) = serde_json::to_string(value) {
                let _: Result<(), _> = conn
                    .set_ex::<_, _, ()>(&cache_key, &serialized, REDIS_CONFIG_TTL as u64)
                    .await;
            }
        }
    }
}
