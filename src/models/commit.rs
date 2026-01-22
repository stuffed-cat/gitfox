use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::repository::DiffInfo;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Commit {
    pub id: Uuid,
    pub project_id: Uuid,
    pub sha: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committed_at: DateTime<Utc>,
    pub parent_shas: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CommitDetail {
    pub sha: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: DateTime<Utc>,
    pub committer_name: String,
    pub committer_email: String,
    pub committed_at: DateTime<Utc>,
    pub parent_shas: Vec<String>,
    pub stats: CommitStats,
    pub diffs: Vec<DiffInfo>,
}

#[derive(Debug, Serialize)]
pub struct CommitStats {
    pub additions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

#[derive(Debug, Deserialize)]
pub struct CommitListQuery {
    pub ref_name: Option<String>,
    pub path: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CompareQuery {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct CompareResult {
    pub commits: Vec<CommitDetail>,
    pub diffs: Vec<DiffInfo>,
    pub stats: CommitStats,
}
