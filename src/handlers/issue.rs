//! Issue handlers

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use sqlx::{FromRow, PgPool};

use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthenticatedUser;
use crate::models::issue::{
    CreateIssueRequest, Issue, IssueAuthor, IssueWithAuthor, UpdateIssueRequest,
};
use crate::services::ProjectService;

/// List issues for a project
pub async fn list_issues(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String)>,
    query: web::Query<ListIssuesQuery>,
    _req: HttpRequest,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();

    // Get project
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;

    let state_filter = query.state.as_deref().unwrap_or("open");

    let issues = sqlx::query_as::<_, IssueRow>(
        r#"
        SELECT 
            i.id, i.project_id, i.iid, i.title, i.description, i.state,
            i.labels, i.due_date, i.weight, i.confidential,
            i.author_id, i.assignee_id,
            i.created_at, i.updated_at, i.closed_at,
            u.username as author_username, u.display_name as author_display_name,
            a.username as assignee_username, a.display_name as assignee_display_name,
            (SELECT COUNT(*) FROM issue_comments WHERE issue_id = i.id) as comment_count
        FROM issues i
        JOIN users u ON i.author_id = u.id
        LEFT JOIN users a ON i.assignee_id = a.id
        WHERE i.project_id = $1 AND ($2 = 'all' OR i.state = $2)
        ORDER BY i.created_at DESC
        LIMIT $3 OFFSET $4
        "#
    )
    .bind(project.id)
    .bind(state_filter)
    .bind(query.per_page.unwrap_or(20) as i64)
    .bind(((query.page.unwrap_or(1) - 1) * query.per_page.unwrap_or(20)) as i64)
    .fetch_all(pool.get_ref())
    .await?;

    let issues_with_author: Vec<IssueWithAuthor> = issues
        .into_iter()
        .map(|i| IssueWithAuthor {
            id: i.id,
            project_id: i.project_id,
            iid: i.iid,
            title: i.title,
            description: i.description,
            state: i.state,
            labels: i.labels,
            due_date: i.due_date,
            weight: i.weight,
            confidential: i.confidential,
            author: IssueAuthor {
                id: i.author_id,
                username: i.author_username,
                display_name: i.author_display_name,
                avatar_url: None,
            },
            assignee: i.assignee_id.map(|aid| IssueAuthor {
                id: aid,
                username: i.assignee_username.unwrap_or_default(),
                display_name: i.assignee_display_name,
                avatar_url: None,
            }),
            comment_count: i.comment_count.unwrap_or(0),
            created_at: i.created_at,
            updated_at: i.updated_at,
            closed_at: i.closed_at,
        })
        .collect();

    Ok(HttpResponse::Ok().json(issues_with_author))
}

/// Get a single issue
pub async fn get_issue(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
    _req: HttpRequest,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, iid) = path.into_inner();

    // Get project
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;

    let issue = sqlx::query_as::<_, IssueRow>(
        r#"
        SELECT 
            i.id, i.project_id, i.iid, i.title, i.description, i.state,
            i.labels, i.due_date, i.weight, i.confidential,
            i.author_id, i.assignee_id,
            i.created_at, i.updated_at, i.closed_at,
            u.username as author_username, u.display_name as author_display_name,
            a.username as assignee_username, a.display_name as assignee_display_name,
            (SELECT COUNT(*) FROM issue_comments WHERE issue_id = i.id) as comment_count
        FROM issues i
        JOIN users u ON i.author_id = u.id
        LEFT JOIN users a ON i.assignee_id = a.id
        WHERE i.project_id = $1 AND i.iid = $2
        "#
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::not_found("Issue not found"))?;

    let issue_with_author = IssueWithAuthor {
        id: issue.id,
        project_id: issue.project_id,
        iid: issue.iid,
        title: issue.title,
        description: issue.description,
        state: issue.state,
        labels: issue.labels,
        due_date: issue.due_date,
        weight: issue.weight,
        confidential: issue.confidential,
        author: IssueAuthor {
            id: issue.author_id,
            username: issue.author_username,
            display_name: issue.author_display_name,
            avatar_url: None,
        },
        assignee: issue.assignee_id.map(|aid| IssueAuthor {
            id: aid,
            username: issue.assignee_username.unwrap_or_default(),
            display_name: issue.assignee_display_name,
            avatar_url: None,
        }),
        comment_count: issue.comment_count.unwrap_or(0),
        created_at: issue.created_at,
        updated_at: issue.updated_at,
        closed_at: issue.closed_at,
    };

    Ok(HttpResponse::Ok().json(issue_with_author))
}

/// Create a new issue
pub async fn create_issue(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String)>,
    body: web::Json<CreateIssueRequest>,
    user: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let (namespace, project_name) = path.into_inner();

    // Get project
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;

    // Get next iid for project
    let next_iid: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(iid), 0) + 1 FROM issues WHERE project_id = $1"
    )
    .bind(project.id)
    .fetch_one(pool.get_ref())
    .await?;

    let issue = sqlx::query_as::<_, Issue>(
        r#"
        INSERT INTO issues (
            project_id, iid, author_id, assignee_id, title, description, 
            state, labels, milestone_id, due_date, weight, confidential
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'open', $7, $8, $9, $10, $11)
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(next_iid)
    .bind(user.user_id)
    .bind(body.assignee_id)
    .bind(&body.title)
    .bind(&body.description)
    .bind(&body.labels.clone().unwrap_or_default() as &[String])
    .bind(body.milestone_id)
    .bind(body.due_date)
    .bind(body.weight)
    .bind(body.confidential.unwrap_or(false))
    .fetch_one(pool.get_ref())
    .await?;

    // Get author info
    let author = sqlx::query_as::<_, IssueAuthor>(
        r#"SELECT id, username, display_name, NULL::text as avatar_url FROM users WHERE id = $1"#
    )
    .bind(user.user_id)
    .fetch_one(pool.get_ref())
    .await?;

    let issue_with_author = IssueWithAuthor {
        id: issue.id,
        project_id: issue.project_id,
        iid: issue.iid,
        title: issue.title,
        description: issue.description,
        state: issue.state,
        labels: issue.labels,
        due_date: issue.due_date,
        weight: issue.weight,
        confidential: issue.confidential,
        author,
        assignee: None,
        comment_count: 0,
        created_at: issue.created_at,
        updated_at: issue.updated_at,
        closed_at: issue.closed_at,
    };

    Ok(HttpResponse::Created().json(issue_with_author))
}

/// Update an issue
pub async fn update_issue(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
    body: web::Json<UpdateIssueRequest>,
    _user: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, iid) = path.into_inner();

    // Get project
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;

    // Update issue
    let now = Utc::now();
    let closed_at = if body.state.as_deref() == Some("closed") {
        Some(now)
    } else {
        None
    };

    let issue = sqlx::query_as::<_, Issue>(
        r#"
        UPDATE issues SET
            title = COALESCE($3, title),
            description = COALESCE($4, description),
            assignee_id = COALESCE($5, assignee_id),
            labels = COALESCE($6, labels),
            milestone_id = COALESCE($7, milestone_id),
            due_date = COALESCE($8, due_date),
            weight = COALESCE($9, weight),
            confidential = COALESCE($10, confidential),
            state = COALESCE($11, state),
            closed_at = COALESCE($12, closed_at),
            updated_at = NOW()
        WHERE project_id = $1 AND iid = $2
        RETURNING *
        "#
    )
    .bind(project.id)
    .bind(iid)
    .bind(&body.title)
    .bind(&body.description)
    .bind(body.assignee_id)
    .bind(body.labels.as_deref())
    .bind(body.milestone_id)
    .bind(body.due_date)
    .bind(body.weight)
    .bind(body.confidential)
    .bind(&body.state)
    .bind(closed_at)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::not_found("Issue not found"))?;

    // Get author
    let author = sqlx::query_as::<_, IssueAuthor>(
        "SELECT id, username, display_name, NULL::text as avatar_url FROM users WHERE id = $1"
    )
    .bind(issue.author_id)
    .fetch_one(pool.get_ref())
    .await?;

    // Get assignee if exists
    let assignee = if let Some(aid) = issue.assignee_id {
        sqlx::query_as::<_, IssueAuthor>(
            "SELECT id, username, display_name, NULL::text as avatar_url FROM users WHERE id = $1"
        )
        .bind(aid)
        .fetch_optional(pool.get_ref())
        .await?
    } else {
        None
    };

    let issue_with_author = IssueWithAuthor {
        id: issue.id,
        project_id: issue.project_id,
        iid: issue.iid,
        title: issue.title,
        description: issue.description,
        state: issue.state,
        labels: issue.labels,
        due_date: issue.due_date,
        weight: issue.weight,
        confidential: issue.confidential,
        author,
        assignee,
        comment_count: 0,
        created_at: issue.created_at,
        updated_at: issue.updated_at,
        closed_at: issue.closed_at,
    };

    Ok(HttpResponse::Ok().json(issue_with_author))
}

/// Delete an issue
pub async fn delete_issue(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
    _user: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, iid) = path.into_inner();

    // Get project
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;

    let result = sqlx::query("DELETE FROM issues WHERE project_id = $1 AND iid = $2")
        .bind(project.id)
        .bind(iid)
        .execute(pool.get_ref())
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Issue not found"));
    }

    Ok(HttpResponse::NoContent().finish())
}

/// List notes (comments) for an issue
pub async fn list_issue_notes(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
    _req: HttpRequest,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, iid) = path.into_inner();

    // Get project
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;

    // Get issue
    let issue = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM issues WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::not_found("Issue not found"))?;

    let notes = sqlx::query_as::<_, IssueNoteWithAuthor>(
        r#"
        SELECT c.id, c.issue_id, c.author_id, c.body, c.system, 
               c.created_at, c.updated_at,
               u.username as author_username, u.display_name as author_display_name
        FROM issue_comments c
        JOIN users u ON c.author_id = u.id
        WHERE c.issue_id = $1
        ORDER BY c.created_at ASC
        "#
    )
    .bind(issue)
    .fetch_all(pool.get_ref())
    .await?;

    let notes_response: Vec<IssueNote> = notes
        .into_iter()
        .map(|n| IssueNote {
            id: n.id,
            body: n.body,
            system: n.system,
            author: IssueAuthor {
                id: n.author_id,
                username: n.author_username,
                display_name: n.author_display_name,
                avatar_url: None,
            },
            created_at: n.created_at,
            updated_at: n.updated_at,
        })
        .collect();

    Ok(HttpResponse::Ok().json(notes_response))
}

/// Add a note (comment) to an issue
pub async fn add_issue_note(
    pool: web::Data<PgPool>,
    path: web::Path<(String, String, i64)>,
    body: web::Json<CreateNoteRequest>,
    user: AuthenticatedUser,
) -> AppResult<HttpResponse> {
    let (namespace, project_name, iid) = path.into_inner();

    // Get project
    let project = ProjectService::get_project_by_owner_and_name(pool.get_ref(), &namespace, &project_name).await?;

    // Get issue
    let issue = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM issues WHERE project_id = $1 AND iid = $2"
    )
    .bind(project.id)
    .bind(iid)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::not_found("Issue not found"))?;

    // Insert note
    let note = sqlx::query_as::<_, IssueComment>(
        r#"
        INSERT INTO issue_comments (issue_id, author_id, body, system)
        VALUES ($1, $2, $3, false)
        RETURNING *
        "#
    )
    .bind(issue)
    .bind(user.user_id)
    .bind(&body.body)
    .fetch_one(pool.get_ref())
    .await?;

    // Get author
    let author = sqlx::query_as::<_, IssueAuthor>(
        "SELECT id, username, display_name, NULL::text as avatar_url FROM users WHERE id = $1"
    )
    .bind(user.user_id)
    .fetch_one(pool.get_ref())
    .await?;

    let note_response = IssueNote {
        id: note.id,
        body: note.body,
        system: note.system,
        author,
        created_at: note.created_at,
        updated_at: note.updated_at,
    };

    Ok(HttpResponse::Created().json(note_response))
}

// Query parameters for listing issues
#[derive(Debug, serde::Deserialize)]
pub struct ListIssuesQuery {
    pub state: Option<String>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

// Internal row types for queries
#[derive(Debug, FromRow)]
struct IssueRow {
    id: i64,
    project_id: i64,
    iid: i64,
    title: String,
    description: Option<String>,
    state: String,
    labels: Vec<String>,
    due_date: Option<chrono::NaiveDate>,
    weight: Option<i32>,
    confidential: bool,
    author_id: i64,
    assignee_id: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    author_username: String,
    author_display_name: Option<String>,
    assignee_username: Option<String>,
    assignee_display_name: Option<String>,
    comment_count: Option<i64>,
}

#[derive(Debug, FromRow)]
struct IssueNoteWithAuthor {
    id: i64,
    issue_id: i64,
    author_id: i64,
    body: String,
    system: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    author_username: String,
    author_display_name: Option<String>,
}

#[derive(Debug, FromRow)]
struct IssueComment {
    id: i64,
    issue_id: i64,
    author_id: i64,
    body: String,
    system: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Issue comment response
#[derive(Debug, serde::Serialize)]
pub struct IssueNote {
    pub id: i64,
    pub body: String,
    pub system: bool,
    pub author: IssueAuthor,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Request to create a note
#[derive(Debug, serde::Deserialize)]
pub struct CreateNoteRequest {
    pub body: String,
}
