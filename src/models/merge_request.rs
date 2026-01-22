use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MergeRequest {
    pub id: Uuid,
    pub project_id: Uuid,
    pub iid: i64,
    pub title: String,
    pub description: Option<String>,
    pub source_branch: String,
    pub target_branch: String,
    pub status: MergeRequestStatus,
    pub author_id: Uuid,
    pub assignee_id: Option<Uuid>,
    pub merged_by: Option<Uuid>,
    pub merged_at: Option<DateTime<Utc>>,
    pub closed_by: Option<Uuid>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "merge_request_status", rename_all = "lowercase")]
pub enum MergeRequestStatus {
    Open,
    Merged,
    Closed,
    Draft,
}

#[derive(Debug, Deserialize)]
pub struct CreateMergeRequestRequest {
    pub title: String,
    pub description: Option<String>,
    pub source_branch: String,
    pub target_branch: String,
    pub assignee_id: Option<Uuid>,
    pub is_draft: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMergeRequestRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub target_branch: Option<String>,
    pub assignee_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MergeRequestComment {
    pub id: Uuid,
    pub merge_request_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub line_number: Option<i32>,
    pub file_path: Option<String>,
    pub parent_id: Option<Uuid>,
    pub is_resolved: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub line_number: Option<i32>,
    pub file_path: Option<String>,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MergeRequestReview {
    pub id: Uuid,
    pub merge_request_id: Uuid,
    pub reviewer_id: Uuid,
    pub status: ReviewStatus,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "review_status", rename_all = "lowercase")]
pub enum ReviewStatus {
    Pending,
    Approved,
    RequestChanges,
    Commented,
}

#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub status: ReviewStatus,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MergeRequestDetail {
    pub merge_request: MergeRequest,
    pub comments: Vec<MergeRequestComment>,
    pub reviews: Vec<MergeRequestReview>,
    pub can_merge: bool,
    pub has_conflicts: bool,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequestListQuery {
    pub status: Option<MergeRequestStatus>,
    pub author_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct MergeOptions {
    pub squash: Option<bool>,
    pub delete_source_branch: Option<bool>,
    pub merge_commit_message: Option<String>,
}
