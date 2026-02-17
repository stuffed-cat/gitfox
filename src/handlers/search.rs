use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::AppResult;
use crate::models::project::ProjectWithOwner;
use crate::models::namespace::Group;
use crate::models::user::User;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub scope: Option<String>,  // all, projects, groups, users, issues, merge_requests
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub projects: Vec<ProjectSearchResult>,
    pub groups: Vec<GroupSearchResult>,
    pub users: Vec<UserSearchResult>,
    pub issues: Vec<IssueSearchResult>,
    pub merge_requests: Vec<MergeRequestSearchResult>,
}

#[derive(Debug, Serialize)]
pub struct ProjectSearchResult {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String,
    pub owner_id: String,
    pub owner_name: Option<String>,
    pub owner_avatar: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub stars_count: Option<i32>,
    pub forks_count: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct GroupSearchResult {
    pub id: String,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub visibility: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserSearchResult {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IssueSearchResult {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub namespace_path: String,
    pub iid: i64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub author_id: String,
    pub author_username: String,
    pub author_avatar: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct MergeRequestSearchResult {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub namespace_path: String,
    pub iid: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub source_branch: String,
    pub target_branch: String,
    pub author_id: String,
    pub author_username: String,
    pub author_avatar: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/v1/search - Global search
pub async fn search(
    pool: web::Data<PgPool>,
    query: web::Query<SearchQuery>,
) -> AppResult<HttpResponse> {
    let search_term = query.q.trim();
    if search_term.is_empty() {
        return Ok(HttpResponse::Ok().json(SearchResult {
            projects: vec![],
            groups: vec![],
            users: vec![],
            issues: vec![],
            merge_requests: vec![],
        }));
    }

    let scope = query.scope.as_deref().unwrap_or("all");
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // 使用 ILIKE 进行不区分大小写的模糊搜索
    let pattern = format!("%{}%", search_term);

    let mut result = SearchResult {
        projects: vec![],
        groups: vec![],
        users: vec![],
        issues: vec![],
        merge_requests: vec![],
    };

    // 搜索项目（只返回公开项目或内部项目，私有项目需要权限检查）
    if scope == "all" || scope == "projects" {
        let projects = sqlx::query_as::<_, ProjectWithOwner>(
            r#"
            SELECT 
              p.*,
              u.username as owner_name,
              u.avatar_url as owner_avatar,
              COALESCE((SELECT COUNT(*) FROM project_stars WHERE project_id = p.id), 0)::int as stars_count,
              COALESCE((SELECT COUNT(*) FROM projects WHERE forked_from_id = p.id), 0)::int as forks_count,
              p.forked_from_id
            FROM projects p
            JOIN users u ON p.owner_id = u.id
            WHERE (p.visibility = 'public' OR p.visibility = 'internal')
              AND (p.name ILIKE $1 OR p.description ILIKE $1)
            ORDER BY 
              CASE WHEN p.name ILIKE $1 THEN 1 ELSE 2 END,
              p.updated_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(&pattern)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(pool.as_ref())
        .await?;

        result.projects = projects.into_iter().map(|p| {
            ProjectSearchResult {
                id: p.id.to_string(),
                name: p.name,
                description: p.description,
                visibility: format!("{:?}", p.visibility).to_lowercase(),
                owner_id: p.owner_id.to_string(),
                owner_name: Some(p.owner_name),
                owner_avatar: p.owner_avatar,
                created_at: p.created_at,
                updated_at: p.updated_at,
                stars_count: Some(p.stars_count),
                forks_count: Some(p.forks_count),
            }
        }).collect();
    }

    // 搜索群组（只返回公开或内部的）
    if scope == "all" || scope == "groups" {
        let groups = sqlx::query_as::<_, Group>(
            r#"
            SELECT * FROM namespaces
            WHERE namespace_type = 'group'
              AND (visibility = 'public' OR visibility = 'internal')
              AND (name ILIKE $1 OR path ILIKE $1 OR description ILIKE $1)
            ORDER BY 
              CASE WHEN name ILIKE $1 THEN 1 ELSE 2 END,
              created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(&pattern)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(pool.as_ref())
        .await?;

        result.groups = groups.into_iter().map(|g| GroupSearchResult {
            id: g.id.to_string(),
            name: g.name,
            path: g.path,
            description: g.description,
            visibility: format!("{:?}", g.visibility).to_lowercase(),
            created_at: g.created_at,
        }).collect();
    }

    // 搜索用户（只返回激活的用户）
    if scope == "all" || scope == "users" {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE is_active = true
              AND (username ILIKE $1 OR display_name ILIKE $1 OR email ILIKE $1)
            ORDER BY 
              CASE WHEN username ILIKE $1 THEN 1 ELSE 2 END,
              created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(&pattern)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(pool.as_ref())
        .await?;

        result.users = users.into_iter().map(|u| UserSearchResult {
            id: u.id.to_string(),
            username: u.username,
            display_name: u.display_name,
            avatar_url: u.avatar_url,
        }).collect();
    }

    // 搜索 issues（只返回公开项目的 issues）
    if scope == "all" || scope == "issues" {
        #[derive(sqlx::FromRow)]
        struct IssueWithProjectAndAuthor {
            issue_id: i64,
            project_id: i64,
            project_name: String,
            namespace_path: String,
            iid: i64,
            title: String,
            description: Option<String>,
            state: String,
            author_id: i64,
            author_username: String,
            author_avatar: Option<String>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
        }

        let issues = sqlx::query_as::<_, IssueWithProjectAndAuthor>(
            r#"
            SELECT 
                i.id as issue_id,
                i.project_id,
                p.name as project_name,
                n.path as namespace_path,
                i.iid,
                i.title,
                i.description,
                i.state,
                i.author_id,
                u.username as author_username,
                u.avatar_url as author_avatar,
                i.created_at,
                i.updated_at
            FROM issues i
            JOIN projects p ON i.project_id = p.id
            JOIN namespaces n ON p.namespace_id = n.id
            JOIN users u ON i.author_id = u.id
            WHERE (p.visibility = 'public' OR p.visibility = 'internal')
              AND (i.title ILIKE $1 OR i.description ILIKE $1)
            ORDER BY 
                CASE WHEN i.title ILIKE $1 THEN 1 ELSE 2 END,
                i.updated_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(&pattern)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(pool.as_ref())
        .await?;

        result.issues = issues.into_iter().map(|i| IssueSearchResult {
            id: i.issue_id.to_string(),
            project_id: i.project_id.to_string(),
            project_name: i.project_name,
            namespace_path: i.namespace_path,
            iid: i.iid,
            title: i.title,
            description: i.description,
            state: i.state,
            author_id: i.author_id.to_string(),
            author_username: i.author_username,
            author_avatar: i.author_avatar,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }).collect();
    }

    // 搜索 merge requests（只返回公开项目的 MRs）
    if scope == "all" || scope == "merge_requests" {
        #[derive(sqlx::FromRow)]
        struct MergeRequestWithProjectAndAuthor {
            mr_id: i64,
            project_id: i64,
            project_name: String,
            namespace_path: String,
            iid: i64,
            title: String,
            description: Option<String>,
            status: String,
            source_branch: String,
            target_branch: String,
            author_id: i64,
            author_username: String,
            author_avatar: Option<String>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
        }

        let merge_requests = sqlx::query_as::<_, MergeRequestWithProjectAndAuthor>(
            r#"
            SELECT 
                mr.id as mr_id,
                mr.project_id,
                p.name as project_name,
                n.path as namespace_path,
                mr.iid,
                mr.title,
                mr.description,
                LOWER(mr.status::text) as status,
                mr.source_branch,
                mr.target_branch,
                mr.author_id,
                u.username as author_username,
                u.avatar_url as author_avatar,
                mr.created_at,
                mr.updated_at
            FROM merge_requests mr
            JOIN projects p ON mr.project_id = p.id
            JOIN namespaces n ON p.namespace_id = n.id
            JOIN users u ON mr.author_id = u.id
            WHERE (p.visibility = 'public' OR p.visibility = 'internal')
              AND (mr.title ILIKE $1 OR mr.description ILIKE $1)
            ORDER BY 
                CASE WHEN mr.title ILIKE $1 THEN 1 ELSE 2 END,
                mr.updated_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(&pattern)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(pool.as_ref())
        .await?;

        result.merge_requests = merge_requests.into_iter().map(|mr| MergeRequestSearchResult {
            id: mr.mr_id.to_string(),
            project_id: mr.project_id.to_string(),
            project_name: mr.project_name,
            namespace_path: mr.namespace_path,
            iid: mr.iid,
            title: mr.title,
            description: mr.description,
            status: mr.status,
            source_branch: mr.source_branch,
            target_branch: mr.target_branch,
            author_id: mr.author_id.to_string(),
            author_username: mr.author_username,
            author_avatar: mr.author_avatar,
            created_at: mr.created_at,
            updated_at: mr.updated_at,
        }).collect();
    }

    Ok(HttpResponse::Ok().json(result))
}
