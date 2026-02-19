use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;


#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Pipeline {
    pub id: i64,
    pub project_id: i64,
    pub ref_name: String,
    pub commit_sha: String,
    pub status: PipelineStatus,
    pub trigger_type: PipelineTriggerType,
    pub triggered_by: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "pipeline_status", rename_all = "lowercase")]
pub enum PipelineStatus {
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "pipeline_trigger_type", rename_all = "lowercase")]
pub enum PipelineTriggerType {
    Push,
    MergeRequest,
    Schedule,
    Manual,
    Api,
    Webhook,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineJob {
    pub id: i64,
    pub pipeline_id: i64,
    pub project_id: i64,
    pub runner_id: Option<i64>,
    pub name: String,
    pub stage: String,
    pub status: String,
    pub config: serde_json::Value,
    pub artifacts_path: Option<String>,
    pub coverage: Option<f32>,
    pub allow_failure: bool,
    pub when_condition: Option<String>,
    pub retry_count: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub error_message: Option<String>,
    
    // 计算字段（不在数据库中）
    #[sqlx(skip)]
    pub duration_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineJobLog {
    pub id: i64,
    pub job_id: i64,
    pub output: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TriggerPipelineRequest {
    pub ref_name: String,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PipelineListQuery {
    pub status: Option<PipelineStatus>,
    pub ref_name: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct PipelineDetail {
    pub pipeline: Pipeline,
    pub jobs: Vec<PipelineJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PipelineConfig {
    pub id: i64,
    pub project_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePipelineConfigRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfigParsed {
    pub stages: Vec<String>,
    pub jobs: Vec<JobConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    pub name: String,
    pub stage: String,
    pub script: Vec<String>,
    pub image: Option<String>,
    pub only: Option<Vec<String>>,
    pub except: Option<Vec<String>>,
    pub allow_failure: Option<bool>,
    pub needs: Option<Vec<String>>,
    pub artifacts: Option<ArtifactConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactConfig {
    pub paths: Vec<String>,
    pub expire_in: Option<String>,
}
