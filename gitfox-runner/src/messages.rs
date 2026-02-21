use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Messages sent from Runner to Server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunnerMessage {
    /// Register runner with server
    #[serde(rename = "register")]
    Register {
        token: String,
        name: String,
        tags: Vec<String>,
        executor: String,
    },

    /// Job status update
    #[serde(rename = "job_update")]
    JobUpdate {
        job_id: i64,
        status: JobStatus,
        exit_code: Option<i32>,
        error_message: Option<String>,
    },

    /// Job log output
    #[serde(rename = "job_log")]
    JobLog { job_id: i64, output: String },

    /// Heartbeat
    #[serde(rename = "heartbeat")]
    Heartbeat,

    /// Request job
    #[serde(rename = "request_job")]
    RequestJob,
}

/// Messages sent from Server to Runner
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Registration successful
    #[serde(rename = "registered")]
    Registered { runner_id: i64 },

    /// Registration failed
    #[serde(rename = "error")]
    Error { message: String },

    /// Assign job to runner
    #[serde(rename = "job_assigned")]
    JobAssigned { job: Job },

    /// No jobs available
    #[serde(rename = "no_jobs")]
    NoJobs,

    /// Server acknowledgment
    #[serde(rename = "ack")]
    Ack,

    /// Configuration update notification
    #[serde(rename = "config_update")]
    ConfigUpdate {
        tags: Option<Vec<String>>,
        description: Option<String>,
        run_untagged: Option<bool>,
        maximum_timeout: Option<i32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub pipeline_id: i64,
    pub project_id: i64,
    pub name: String,
    pub stage: String,
    pub script: Vec<String>,
    pub variables: HashMap<String, String>,
    pub artifacts: Option<ArtifactsConfig>,
    pub cache: Option<CacheConfig>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<i32>,
    pub allow_failure: bool,
    pub when: WhenCondition,
    pub repository_url: String,
    pub ref_name: String,
    pub commit_sha: String,
    pub before_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactsConfig {
    pub paths: Vec<String>,
    pub expire_in: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub paths: Vec<String>,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max: i32,
    pub when: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Success,
    Failed,
    Canceled,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WhenCondition {
    OnSuccess,
    OnFailure,
    Always,
    Manual,
}
