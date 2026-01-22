use chrono::Utc;
use slug::slugify;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{
    AddMemberRequest, CreateProjectRequest, MemberRole, Project, ProjectMember,
    ProjectStats, ProjectVisibility, UpdateProjectRequest, ProjectWithOwner,
};

pub struct ProjectService;

impl ProjectService {
    pub async fn create_project(
        pool: &PgPool,
        owner_id: Uuid,
        req: CreateProjectRequest,
    ) -> AppResult<Project> {
        let slug = slugify(&req.name);
        let id = Uuid::new_v4();
        let now = Utc::now();
        let visibility = req.visibility.unwrap_or(ProjectVisibility::Private);
        let default_branch = req.default_branch.unwrap_or_else(|| "main".to_string());

        // Check if slug exists
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM projects WHERE slug = $1"
        )
        .bind(&slug)
        .fetch_one(pool)
        .await?;

        if existing > 0 {
            return Err(AppError::Conflict("Project with this name already exists".to_string()));
        }

        let project = sqlx::query_as::<_, Project>(
            r#"
            INSERT INTO projects (id, name, slug, description, visibility, owner_id, default_branch, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
            RETURNING *
            "#
        )
        .bind(id)
        .bind(&req.name)
        .bind(&slug)
        .bind(&req.description)
        .bind(visibility)
        .bind(owner_id)
        .bind(&default_branch)
        .bind(now)
        .fetch_one(pool)
        .await?;

        // Add owner as project member
        Self::add_member(pool, project.id, owner_id, MemberRole::Owner).await?;

        Ok(project)
    }

    pub async fn get_project_by_id(pool: &PgPool, id: Uuid) -> AppResult<Project> {
        sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))
    }

    pub async fn get_project_by_slug(pool: &PgPool, slug: &str) -> AppResult<ProjectWithOwner> {
        sqlx::query_as::<_, ProjectWithOwner>(
            r#"
            SELECT p.*, u.username as owner_name, u.avatar_url as owner_avatar
            FROM projects p
            JOIN users u ON p.owner_id = u.id
            WHERE p.slug = $1
            "#
        )
        .bind(slug)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".to_string()))
    }

    /// GitLab/GitHub 风格：通过 owner/repo 获取项目
    pub async fn get_project_by_owner_and_slug(pool: &PgPool, owner: &str, repo: &str) -> AppResult<ProjectWithOwner> {
        sqlx::query_as::<_, ProjectWithOwner>(
            r#"
            SELECT p.*, u.username as owner_name, u.avatar_url as owner_avatar
            FROM projects p
            JOIN users u ON p.owner_id = u.id
            WHERE LOWER(u.username) = LOWER($1) AND LOWER(p.slug) = LOWER($2)
            "#
        )
        .bind(owner)
        .bind(repo)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Project '{}/{}' not found", owner, repo)))
    }

    pub async fn list_projects(
        pool: &PgPool,
        user_id: Option<Uuid>,
        page: u32,
        per_page: u32,
    ) -> AppResult<Vec<ProjectWithOwner>> {
        let offset = (page.saturating_sub(1)) * per_page;

        let projects = if let Some(uid) = user_id {
            sqlx::query_as::<_, ProjectWithOwner>(
                r#"
                SELECT p.*, u.username as owner_name, u.avatar_url as owner_avatar
                FROM projects p
                JOIN users u ON p.owner_id = u.id
                LEFT JOIN project_members pm ON p.id = pm.project_id
                WHERE p.visibility = 'public' OR pm.user_id = $1
                GROUP BY p.id, u.username, u.avatar_url
                ORDER BY p.updated_at DESC
                LIMIT $2 OFFSET $3
                "#
            )
            .bind(uid)
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, ProjectWithOwner>(
                r#"
                SELECT p.*, u.username as owner_name, u.avatar_url as owner_avatar
                FROM projects p
                JOIN users u ON p.owner_id = u.id
                WHERE visibility = 'public'
                ORDER BY updated_at DESC
                LIMIT $1 OFFSET $2
                "#
            )
            .bind(per_page as i64)
            .bind(offset as i64)
            .fetch_all(pool)
            .await?
        };

        Ok(projects)
    }

    pub async fn update_project(
        pool: &PgPool,
        id: Uuid,
        req: UpdateProjectRequest,
    ) -> AppResult<Project> {
        let project = sqlx::query_as::<_, Project>(
            r#"
            UPDATE projects
            SET name = COALESCE($2, name),
                description = COALESCE($3, description),
                visibility = COALESCE($4, visibility),
                default_branch = COALESCE($5, default_branch),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .bind(req.name)
        .bind(req.description)
        .bind(req.visibility)
        .bind(req.default_branch)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        Ok(project)
    }

    pub async fn delete_project(pool: &PgPool, id: Uuid) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Project not found".to_string()));
        }

        Ok(())
    }

    pub async fn add_member(
        pool: &PgPool,
        project_id: Uuid,
        user_id: Uuid,
        role: MemberRole,
    ) -> AppResult<ProjectMember> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let member = sqlx::query_as::<_, ProjectMember>(
            r#"
            INSERT INTO project_members (id, project_id, user_id, role, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (project_id, user_id) DO UPDATE SET role = $4
            RETURNING *
            "#
        )
        .bind(id)
        .bind(project_id)
        .bind(user_id)
        .bind(role)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(member)
    }

    pub async fn remove_member(pool: &PgPool, project_id: Uuid, user_id: Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND user_id = $2")
            .bind(project_id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_project_members(pool: &PgPool, project_id: Uuid) -> AppResult<Vec<ProjectMember>> {
        let members = sqlx::query_as::<_, ProjectMember>(
            "SELECT * FROM project_members WHERE project_id = $1 ORDER BY created_at"
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(members)
    }

    pub async fn get_member_role(pool: &PgPool, project_id: Uuid, user_id: Uuid) -> AppResult<Option<MemberRole>> {
        let role = sqlx::query_scalar::<_, MemberRole>(
            "SELECT role FROM project_members WHERE project_id = $1 AND user_id = $2"
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(role)
    }

    pub async fn get_project_stats(pool: &PgPool, project_id: Uuid) -> AppResult<ProjectStats> {
        let commits_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM commits WHERE project_id = $1"
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        let branches_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM branches WHERE project_id = $1"
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        let tags_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tags WHERE project_id = $1"
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        let merge_requests_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM merge_requests WHERE project_id = $1"
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        let members_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM project_members WHERE project_id = $1"
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        Ok(ProjectStats {
            commits_count,
            branches_count,
            tags_count,
            merge_requests_count,
            members_count,
        })
    }
}
