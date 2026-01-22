use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub visibility: ProjectVisibility,
    pub owner_id: Uuid,
    pub default_branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "project_visibility", rename_all = "lowercase")]
pub enum ProjectVisibility {
    Public,
    Private,
    Internal,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProjectRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    pub visibility: Option<ProjectVisibility>,
    pub default_branch: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<ProjectVisibility>,
    pub default_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectMember {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: MemberRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "member_role", rename_all = "lowercase")]
pub enum MemberRole {
    Owner,
    Maintainer,
    Developer,
    Reporter,
    Guest,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role: MemberRole,
}

#[derive(Debug, Serialize)]
pub struct ProjectStats {
    pub commits_count: i64,
    pub branches_count: i64,
    pub tags_count: i64,
    pub merge_requests_count: i64,
    pub members_count: i64,
}

/// Project with owner info for API responses
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectWithOwner {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub visibility: ProjectVisibility,
    pub owner_id: Uuid,
    pub default_branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner_name: String,
    pub owner_avatar: Option<String>,
}
