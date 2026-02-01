//! Issue models

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Issue state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}

impl Default for IssueState {
    fn default() -> Self {
        Self::Open
    }
}

/// Issue from database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Issue {
    pub id: i64,
    pub project_id: i64,
    pub iid: i64,
    pub author_id: i64,
    pub assignee_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub labels: Vec<String>,
    pub milestone_id: Option<i64>,
    pub due_date: Option<NaiveDate>,
    pub weight: Option<i32>,
    pub confidential: bool,
    pub discussion_locked: bool,
    pub closed_at: Option<DateTime<Utc>>,
    pub closed_by_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Issue with author info for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueWithAuthor {
    pub id: i64,
    pub project_id: i64,
    pub iid: i64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub labels: Vec<String>,
    pub due_date: Option<NaiveDate>,
    pub weight: Option<i32>,
    pub confidential: bool,
    pub author: IssueAuthor,
    pub assignee: Option<IssueAuthor>,
    pub comment_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

/// Author info for issues
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IssueAuthor {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Request to create a new issue
#[derive(Debug, Clone, Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    pub description: Option<String>,
    pub assignee_id: Option<i64>,
    pub labels: Option<Vec<String>>,
    pub milestone_id: Option<i64>,
    pub due_date: Option<NaiveDate>,
    pub weight: Option<i32>,
    pub confidential: Option<bool>,
}

/// Request to update an issue
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateIssueRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub assignee_id: Option<i64>,
    pub labels: Option<Vec<String>>,
    pub milestone_id: Option<i64>,
    pub due_date: Option<NaiveDate>,
    pub weight: Option<i32>,
    pub confidential: Option<bool>,
    pub state: Option<String>,
}

/// Issue comment from database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IssueComment {
    pub id: i64,
    pub issue_id: i64,
    pub author_id: i64,
    pub body: String,
    pub system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Issue comment with author info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueCommentWithAuthor {
    pub id: i64,
    pub issue_id: i64,
    pub body: String,
    pub system: bool,
    pub author: IssueAuthor,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create an issue comment
#[derive(Debug, Clone, Deserialize)]
pub struct CreateIssueCommentRequest {
    pub body: String,
}

/// Issue label from database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IssueLabel {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a label
#[derive(Debug, Clone, Deserialize)]
pub struct CreateLabelRequest {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

/// Issue list query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct IssueListQuery {
    pub state: Option<String>,
    pub author_id: Option<i64>,
    pub assignee_id: Option<i64>,
    pub labels: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
