use chrono::Utc;
use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};
use crate::models::{RunnerUsageSummary, UserRunnerUsage};
use crate::services::SystemConfigService;

pub struct RunnerUsageService;

impl RunnerUsageService {
    /// Get current month in YYYY-MM format
    fn current_month() -> String {
        Utc::now().format("%Y-%m").to_string()
    }

    /// Get user's runner quota based on pro status
    pub async fn get_user_quota(pool: &PgPool, redis: &RedisPool, user_id: i64) -> AppResult<i32> {
        // Check if user is pro
        let is_pro = sqlx::query_scalar::<_, bool>(
            "SELECT COALESCE(is_pro, false) FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let config_key = if is_pro {
            "pro_runner_quota_minutes"
        } else {
            "regular_runner_quota_minutes"
        };

        let quota_value = SystemConfigService::get(pool, redis, config_key).await?;
        let quota = quota_value
            .as_i64()
            .or_else(|| quota_value.as_str().and_then(|s| s.parse::<i64>().ok()))
            .unwrap_or(0) as i32;

        Ok(quota)
    }

    /// Get user's usage for current month
    pub async fn get_monthly_usage(pool: &PgPool, user_id: i64, month: &str) -> AppResult<i64> {
        let total = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT SUM(minutes_used) FROM user_runner_usage WHERE user_id = $1 AND month = $2"
        )
        .bind(user_id)
        .bind(month)
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok(total)
    }

    /// Get user's current month usage summary
    pub async fn get_usage_summary(
        pool: &PgPool,
        redis: &RedisPool,
        user_id: i64,
    ) -> AppResult<RunnerUsageSummary> {
        let month = Self::current_month();
        let total_minutes_used = Self::get_monthly_usage(pool, user_id, &month).await?;
        
        let is_pro = sqlx::query_scalar::<_, bool>(
            "SELECT COALESCE(is_pro, false) FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let quota_minutes = Self::get_user_quota(pool, redis, user_id).await?;
        
        let quota_exceeded = if quota_minutes == 0 {
            false // unlimited
        } else {
            total_minutes_used >= quota_minutes as i64
        };

        Ok(RunnerUsageSummary {
            user_id,
            month,
            total_minutes_used,
            quota_minutes,
            is_pro,
            quota_exceeded,
        })
    }

    /// Check if user can run a job (has quota available)
    pub async fn check_quota(
        pool: &PgPool,
        redis: &RedisPool,
        user_id: i64,
    ) -> AppResult<bool> {
        let quota = Self::get_user_quota(pool, redis, user_id).await?;
        
        // 0 means unlimited
        if quota == 0 {
            return Ok(true);
        }

        let month = Self::current_month();
        let used = Self::get_monthly_usage(pool, user_id, &month).await?;
        
        Ok(used < quota as i64)
    }

    /// Record runner usage after job completion
    pub async fn record_usage(
        pool: &PgPool,
        user_id: i64,
        job_id: i64,
        minutes_used: i32,
    ) -> AppResult<UserRunnerUsage> {
        let month = Self::current_month();

        let usage = sqlx::query_as::<_, UserRunnerUsage>(
            r#"
            INSERT INTO user_runner_usage (user_id, job_id, minutes_used, month, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (user_id, job_id) 
            DO UPDATE SET minutes_used = $3, created_at = NOW()
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(job_id)
        .bind(minutes_used)
        .bind(&month)
        .fetch_one(pool)
        .await?;

        Ok(usage)
    }

    /// Get user's usage history
    pub async fn get_usage_history(
        pool: &PgPool,
        user_id: i64,
        limit: i64,
    ) -> AppResult<Vec<UserRunnerUsage>> {
        let usage = sqlx::query_as::<_, UserRunnerUsage>(
            "SELECT * FROM user_runner_usage WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2"
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(usage)
    }

    /// Get monthly statistics for a user
    pub async fn get_monthly_stats(
        pool: &PgPool,
        user_id: i64,
        months: i32,
    ) -> AppResult<Vec<(String, i64)>> {
        let stats = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT month, SUM(minutes_used) as total
            FROM user_runner_usage
            WHERE user_id = $1
            GROUP BY month
            ORDER BY month DESC
            LIMIT $2
            "#
        )
        .bind(user_id)
        .bind(months)
        .fetch_all(pool)
        .await?;

        Ok(stats)
    }
}
