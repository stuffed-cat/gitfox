use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub visibility: ProjectVisibility,
    pub owner_id: i64,
    pub namespace_id: Option<i64>,
    pub default_branch: String,
    pub archived: bool,
    pub issues_enabled: bool,
    pub merge_requests_enabled: bool,
    pub pipelines_enabled: bool,
    pub packages_enabled: bool,
    pub wiki_enabled: bool,
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
    /// Optional namespace_id (group's namespace). If not provided, uses user's namespace.
    pub namespace_id: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<ProjectVisibility>,
    pub default_branch: Option<String>,
    pub archived: Option<bool>,
    pub issues_enabled: Option<bool>,
    pub merge_requests_enabled: Option<bool>,
    pub pipelines_enabled: Option<bool>,
    pub packages_enabled: Option<bool>,
    pub wiki_enabled: Option<bool>,
    /// Transfer project to a different namespace
    pub namespace_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectMember {
    pub id: i64,
    pub project_id: i64,
    pub user_id: i64,
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
    pub user_id: i64,
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
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub visibility: ProjectVisibility,
    pub owner_id: i64,
    pub namespace_id: Option<i64>,
    pub default_branch: String,
    pub archived: bool,
    pub issues_enabled: bool,
    pub merge_requests_enabled: bool,
    pub pipelines_enabled: bool,
    pub packages_enabled: bool,
    pub wiki_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub owner_name: String,
    pub owner_avatar: Option<String>,
    #[serde(default)]
    pub stars_count: i32,
    #[serde(default)]
    pub forks_count: i32,
    pub forked_from_id: Option<i64>,
    pub forked_from_namespace: Option<String>,
    pub forked_from_name: Option<String>,
}

/// Project star record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectStar {
    pub id: i64,
    pub project_id: i64,
    pub user_id: i64,
    pub created_at: DateTime<Utc>,
}

/// Project fork record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectFork {
    pub id: i64,
    pub source_project_id: i64,
    pub forked_project_id: i64,
    pub forked_by: i64,
    pub created_at: DateTime<Utc>,
}

/// Fork project request
#[derive(Debug, Deserialize)]
pub struct ForkProjectRequest {
    /// Target namespace for the forked project (user namespace or group)
    pub namespace_id: Option<i64>,
    /// Optional new name for the forked project
    pub name: Option<String>,
    /// Optional description for the forked project
    pub description: Option<String>,
    /// Visibility level for the forked project
    pub visibility: Option<ProjectVisibility>,
    /// Which branches to include: "all" or "default"
    #[serde(default)]
    pub branches: Option<String>,
}

/// Fork divergence information (commits ahead/behind upstream)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkDivergence {
    /// Number of commits ahead of upstream
    pub ahead: usize,
    /// Number of commits behind upstream
    pub behind: usize,
    /// Default branch of current fork
    pub fork_branch: String,
    /// Default branch of upstream
    pub upstream_branch: String,
}
