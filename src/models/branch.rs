use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;


use super::repository::CommitInfo;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Branch {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub commit_sha: String,
    pub is_protected: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BranchInfo {
    pub name: String,
    pub commit: CommitInfo,
    pub is_protected: bool,
    pub is_default: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
    pub ref_name: String,
}

#[derive(Debug, Deserialize)]
pub struct ProtectBranchRequest {
    pub is_protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BranchProtectionRule {
    pub id: i64,
    pub project_id: i64,
    pub branch_pattern: String,
    pub require_review: bool,
    pub required_reviewers: i32,
    pub require_ci_pass: bool,
    pub allow_force_push: bool,
    pub allow_deletion: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProtectionRuleRequest {
    pub branch_pattern: String,
    pub require_review: Option<bool>,
    pub required_reviewers: Option<i32>,
    pub require_ci_pass: Option<bool>,
    pub allow_force_push: Option<bool>,
    pub allow_deletion: Option<bool>,
}
