use chrono::Utc;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};
use crate::models::{
    CreateProjectRequest, MemberRole, Project, ProjectMember,
    ProjectStats, ProjectVisibility, UpdateProjectRequest, ProjectWithOwner,
};
use crate::models::namespace::AccessLevel;

pub struct ProjectService;

impl ProjectService {
    /// 创建项目 - 使用 BIGSERIAL 自增ID
    /// 支持在用户命名空间或组群命名空间下创建项目
    pub async fn create_project(
        pool: &PgPool,
        owner_id: i64,
        req: CreateProjectRequest,
    ) -> AppResult<ProjectWithOwner> {
        let now = Utc::now();
        let visibility = req.visibility.unwrap_or(ProjectVisibility::Private);

        // 获取实际使用的命名空间
        let (namespace_id, owner_name, owner_avatar): (i64, String, Option<String>) = 
            if let Some(ns_id) = req.namespace_id {
                // 使用指定的命名空间，需要验证权限
                let ns = sqlx::query_as::<_, (i64, String, Option<String>, String, Option<i64>)>(
                    r#"
                    SELECT n.id, n.path, n.avatar_url, n.namespace_type::text, n.owner_id
                    FROM namespaces n
                    WHERE n.id = $1
                    "#
                )
                .bind(ns_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Namespace not found".to_string()))?;

                let (ns_id, ns_path, ns_avatar, ns_type, ns_owner_id) = ns;

                // 如果是组群命名空间，检查用户权限
                if ns_type == "group" {
                    // 获取用户在组群中的权限
                    let access_level = sqlx::query_scalar::<_, i32>(
                        r#"
                        SELECT COALESCE(
                            (SELECT gm.access_level FROM group_members gm
                             JOIN groups g ON g.id = gm.group_id
                             WHERE g.namespace_id = $1 AND gm.user_id = $2),
                            0
                        )
                        "#
                    )
                    .bind(ns_id)
                    .bind(owner_id)
                    .fetch_one(pool)
                    .await?;

                    // 至少需要 Developer (30) 权限才能创建项目
                    if access_level < AccessLevel::Developer as i32 {
                        return Err(AppError::Forbidden(
                            "You don't have permission to create projects in this group".to_string()
                        ));
                    }
                } else if ns_type == "user" {
                    // 用户命名空间，检查是否是自己的
                    if ns_owner_id != Some(owner_id) {
                        return Err(AppError::Forbidden(
                            "You can only create projects in your own namespace".to_string()
                        ));
                    }
                }

                (ns_id, ns_path, ns_avatar)
            } else {
                // 没有指定命名空间，使用用户的命名空间
                let user_ns = sqlx::query_as::<_, (i64, String, Option<String>)>(
                    r#"
                    SELECT n.id, n.path, n.avatar_url
                    FROM namespaces n
                    WHERE n.namespace_type = 'user' AND n.owner_id = $1
                    "#
                )
                .bind(owner_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::NotFound("User namespace not found".to_string()))?;

                user_ns
            };

        // 检查同一命名空间下是否已存在同名项目
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM projects WHERE namespace_id = $1 AND LOWER(name) = LOWER($2)"
        )
        .bind(namespace_id)
        .bind(&req.name)
        .fetch_one(pool)
        .await?;

        if existing > 0 {
            return Err(AppError::Conflict("Project with this name already exists in this namespace".to_string()));
        }

        let project = sqlx::query_as::<_, ProjectWithOwner>(
            r#"
            INSERT INTO projects (name, description, visibility, owner_id, namespace_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            RETURNING id, name, description, visibility, owner_id, created_at, updated_at,
                $7 as owner_name,
                $8 as owner_avatar
            "#
        )
        .bind(&req.name)
        .bind(&req.description)
        .bind(visibility)
        .bind(owner_id)
        .bind(namespace_id)
        .bind(now)
        .bind(&owner_name)
        .bind(&owner_avatar)
        .fetch_one(pool)
        .await?;

        // 把 owner 添加为项目成员
        Self::add_member(pool, project.id, owner_id, MemberRole::Owner).await?;

        Ok(project)
    }

    pub async fn get_project_by_id(pool: &PgPool, id: i64) -> AppResult<Project> {
        sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Project not found".to_string()))
    }

    /// 通过 namespace/name 获取项目 (标准 GitLab/GitHub 风格)
    /// 支持用户命名空间和组群命名空间
    pub async fn get_project_by_owner_and_name(pool: &PgPool, owner: &str, name: &str) -> AppResult<ProjectWithOwner> {
        // 先尝试通过命名空间路径查找
        sqlx::query_as::<_, ProjectWithOwner>(
            r#"
            SELECT p.id, p.name, p.description, p.visibility, p.owner_id, p.created_at, p.updated_at,
                   n.path as owner_name, n.avatar_url as owner_avatar
            FROM projects p
            JOIN namespaces n ON p.namespace_id = n.id
            WHERE LOWER(n.path) = LOWER($1) AND LOWER(p.name) = LOWER($2)
            "#
        )
        .bind(owner)
        .bind(name)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Project '{}/{}' not found", owner, name)))
    }

    pub async fn list_projects(
        pool: &PgPool,
        user_id: Option<i64>,
        page: u32,
        per_page: u32,
    ) -> AppResult<Vec<ProjectWithOwner>> {
        let offset = (page.saturating_sub(1)) * per_page;

        let projects = if let Some(uid) = user_id {
            // Get projects the user can access:
            // 1. Public projects
            // 2. Projects the user owns
            // 3. Projects the user is a member of
            // 4. Projects in groups the user is a member of
            sqlx::query_as::<_, ProjectWithOwner>(
                r#"
                SELECT DISTINCT p.id, p.name, p.description, p.visibility, p.owner_id, p.created_at, p.updated_at,
                       n.path as owner_name, n.avatar_url as owner_avatar
                FROM projects p
                JOIN namespaces n ON p.namespace_id = n.id
                LEFT JOIN project_members pm ON p.id = pm.project_id AND pm.user_id = $1
                LEFT JOIN groups g ON n.namespace_type = 'group' AND n.id = g.namespace_id
                LEFT JOIN group_members gm ON g.id = gm.group_id AND gm.user_id = $1
                WHERE p.visibility = 'public'
                   OR p.owner_id = $1
                   OR pm.user_id IS NOT NULL
                   OR gm.user_id IS NOT NULL
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
                SELECT p.id, p.name, p.description, p.visibility, p.owner_id, p.created_at, p.updated_at,
                       n.path as owner_name, n.avatar_url as owner_avatar
                FROM projects p
                JOIN namespaces n ON p.namespace_id = n.id
                WHERE p.visibility = 'public'
                ORDER BY p.updated_at DESC
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
        id: i64,
        req: UpdateProjectRequest,
    ) -> AppResult<Project> {
        let project = sqlx::query_as::<_, Project>(
            r#"
            UPDATE projects
            SET name = COALESCE($2, name),
                description = COALESCE($3, description),
                visibility = COALESCE($4, visibility),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(id)
        .bind(req.name)
        .bind(req.description)
        .bind(req.visibility)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

        Ok(project)
    }

    pub async fn delete_project(pool: &PgPool, id: i64) -> AppResult<()> {
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
        project_id: i64,
        user_id: i64,
        role: MemberRole,
    ) -> AppResult<ProjectMember> {
        let now = Utc::now();

        let member = sqlx::query_as::<_, ProjectMember>(
            r#"
            INSERT INTO project_members (project_id, user_id, role, created_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (project_id, user_id) DO UPDATE SET role = $3
            RETURNING *
            "#
        )
        .bind(project_id)
        .bind(user_id)
        .bind(role)
        .bind(now)
        .fetch_one(pool)
        .await?;

        Ok(member)
    }

    pub async fn remove_member(pool: &PgPool, project_id: i64, user_id: i64) -> AppResult<()> {
        sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND user_id = $2")
            .bind(project_id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_project_members(pool: &PgPool, project_id: i64) -> AppResult<Vec<ProjectMember>> {
        let members = sqlx::query_as::<_, ProjectMember>(
            "SELECT * FROM project_members WHERE project_id = $1 ORDER BY created_at"
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(members)
    }

    pub async fn get_member_role(pool: &PgPool, project_id: i64, user_id: i64) -> AppResult<Option<MemberRole>> {
        let role = sqlx::query_scalar::<_, MemberRole>(
            "SELECT role FROM project_members WHERE project_id = $1 AND user_id = $2"
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(role)
    }

    pub async fn get_project_stats(pool: &PgPool, project_id: i64) -> AppResult<ProjectStats> {
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
