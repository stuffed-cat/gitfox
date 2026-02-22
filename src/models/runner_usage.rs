use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Runner usage record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRunnerUsage {
    pub id: i64,
    pub user_id: i64,
    pub job_id: i64,
    pub minutes_used: i32,
    pub month: String, // YYYY-MM format
    pub created_at: DateTime<Utc>,
}

/// Monthly usage summary
#[derive(Debug, Serialize, Deserialize)]
pub struct RunnerUsageSummary {
    pub user_id: i64,
    pub month: String,
    pub total_minutes_used: i64,
    pub quota_minutes: i32, // 0 = unlimited
    pub is_pro: bool,
    pub quota_exceeded: bool,
}

/// Request to record runner usage
#[derive(Debug, Deserialize)]
pub struct RecordRunnerUsageRequest {
    pub job_id: i64,
    pub minutes_used: i32,
}
