use actix_web::{web, HttpResponse};
use sqlx::PgPool;


use crate::error::AppError;
use crate::middleware::auth::AuthenticatedUser;
use crate::models::namespace::{
    AccessLevel, CreateGroupRequest, UpdateGroupRequest, AddGroupMemberRequest,
    Group, GroupMember, NamespaceVisibility, NamespaceInfo,
};

/// Path resolve response - returns what type of entity a path refers to
#[derive(Debug, serde::Serialize)]
pub struct PathResolveResponse {
    /// Type of the entity: "project", "group", "user", or "not_found"
    #[serde(rename = "type")]
    pub entity_type: String,
}

/// Resolve a path to determine if it's a project or namespace (group/user)
/// GET /api/v1/resolve/{path:.*}
pub async fn resolve_path(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let full_path = path.into_inner();
    
    // 检查是否是项目（namespace/project 格式）
    if let Some(slash_pos) = full_path.rfind('/') {
        let namespace = &full_path[..slash_pos];
        let project_name = &full_path[slash_pos + 1..];
        
        // 先查项目
        let is_project = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM projects p
                JOIN namespaces n ON n.id = p.namespace_id
                WHERE n.path = $1 AND p.name = $2
            )
            "#
        )
        .bind(namespace)
        .bind(project_name)
        .fetch_one(pool.get_ref())
        .await
        .map_err(AppError::from)?;
        
        if is_project {
            return Ok(HttpResponse::Ok().json(PathResolveResponse {
                entity_type: "project".to_string(),
            }));
        }
    }
    
    // 检查是否是命名空间（group 或 user）
    let namespace_type = sqlx::query_scalar::<_, String>(
        r#"
        SELECT namespace_type::text FROM namespaces WHERE path = $1
        "#
    )
    .bind(&full_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    match namespace_type {
        Some(ns_type) => Ok(HttpResponse::Ok().json(PathResolveResponse {
            entity_type: ns_type,
        })),
        None => Ok(HttpResponse::Ok().json(PathResolveResponse {
            entity_type: "not_found".to_string(),
        })),
    }
}

/// Namespace option for project creation
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct NamespaceOption {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub namespace_type: crate::models::namespace::NamespaceType,
    pub avatar_url: Option<String>,
}

/// List namespaces where the user can create projects
/// Returns user's own namespace + groups where user has Developer or higher access
pub async fn list_namespaces_for_project_creation(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let namespaces = sqlx::query_as::<_, NamespaceOption>(
        r#"
        SELECT n.id, n.name, n.path, n.namespace_type, n.avatar_url
        FROM namespaces n
        WHERE 
            -- User's own namespace
            (n.namespace_type = 'user' AND n.owner_id = $1)
            OR
            -- Groups where user has Developer (30) or higher access
            (n.namespace_type = 'group' AND EXISTS (
                SELECT 1 FROM group_members gm
                JOIN groups g ON g.id = gm.group_id
                WHERE g.namespace_id = n.id AND gm.user_id = $1 AND gm.access_level >= 30
            ))
        ORDER BY n.namespace_type DESC, n.name
        "#
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Ok().json(namespaces))
}

/// List all groups the current user has access to
pub async fn list_groups(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let groups = sqlx::query_as::<_, Group>(
        r#"
        SELECT g.* FROM groups g
        LEFT JOIN group_members gm ON g.id = gm.group_id
        WHERE g.visibility = 'public' 
           OR g.visibility = 'internal'
           OR gm.user_id = $1
        GROUP BY g.id
        ORDER BY g.name
        "#
    )
    .bind(auth.user_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Ok().json(groups))
}

/// Create a new group
pub async fn create_group(
    pool: web::Data<PgPool>,
    auth: AuthenticatedUser,
    body: web::Json<CreateGroupRequest>,
) -> Result<HttpResponse, AppError> {
    // If parent_id is provided, verify that the user has permission to create subgroups
    // and construct full path (parent_path/new_path)
    let (parent_namespace_id, full_path): (Option<i64>, String) = if let Some(parent_id) = body.parent_id {
        let parent_group = sqlx::query_as::<_, Group>(
            "SELECT * FROM groups WHERE id = $1"
        )
        .bind(parent_id)
        .fetch_optional(pool.get_ref())
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::not_found("Parent group not found"))?;
        
        // Check permission: need at least Developer to create subgroups
        let access = get_user_group_access(pool.get_ref(), parent_id, auth.user_id).await?;
        if access < AccessLevel::Developer {
            return Err(AppError::forbidden("You don't have permission to create subgroups in this group"));
        }
        
        // Construct full path: parent_path/new_path
        let full_path = format!("{}/{}", parent_group.path, body.path);
        (Some(parent_group.namespace_id), full_path)
    } else {
        // Top-level group, use path as-is
        (None, body.path.clone())
    };
    
    // Check if full path is already taken
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM namespaces WHERE path = $1"
    )
    .bind(&full_path)
    .fetch_one(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    if existing > 0 {
        return Err(AppError::conflict("This path already exists"));
    }
    
    // For top-level groups, check if path conflicts with username
    if body.parent_id.is_none() {
        let user_exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE username = $1"
        )
        .bind(&full_path)
        .fetch_one(pool.get_ref())
        .await
        .map_err(AppError::from)?;
        
        if user_exists > 0 {
            return Err(AppError::conflict("Path conflicts with existing username"));
        }
    }
    
    let visibility = body.visibility.clone().unwrap_or(NamespaceVisibility::Private);
    
    // Create namespace first with full_path
    let namespace_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO namespaces (name, path, namespace_type, visibility, owner_id, parent_id)
        VALUES ($1, $2, 'group', $3, $4, $5)
        RETURNING id
        "#
    )
    .bind(&body.name)
    .bind(&full_path)  // Store full path
    .bind(&visibility)
    .bind(auth.user_id)
    .bind(parent_namespace_id)  // Link to parent's namespace_id
    .fetch_one(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    // Create group with full path
    let group = sqlx::query_as::<_, Group>(
        r#"
        INSERT INTO groups (namespace_id, name, path, description, visibility, parent_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(namespace_id)
    .bind(&body.name)
    .bind(&full_path)  // Store full path
    .bind(&body.description)
    .bind(&visibility)
    .bind(&body.parent_id)  // Link to parent group's id
    .fetch_one(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    // Add creator as owner
    sqlx::query(
        r#"
        INSERT INTO group_members (group_id, user_id, access_level)
        VALUES ($1, $2, $3)
        "#
    )
    .bind(group.id)
    .bind(auth.user_id)
    .bind(AccessLevel::Owner as i32)
    .execute(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Created().json(group))
}

/// Get a group by path
pub async fn get_group(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let group_path = path.into_inner();
    
    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE path = $1"
    )
    .bind(&group_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Group not found"))?;
    
    Ok(HttpResponse::Ok().json(group))
}

/// Update a group
pub async fn update_group(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    auth: AuthenticatedUser,
    body: web::Json<UpdateGroupRequest>,
) -> Result<HttpResponse, AppError> {
    let group_path = path.into_inner();
    
    // Check if user has permission to update
    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE path = $1"
    )
    .bind(&group_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Group not found"))?;
    
    // Check access level
    let access = get_user_group_access(&pool, group.id, auth.user_id).await?;
    if access < AccessLevel::Maintainer {
        return Err(AppError::forbidden("Insufficient permissions"));
    }
    
    let updated = sqlx::query_as::<_, Group>(
        r#"
        UPDATE groups SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            visibility = COALESCE($3, visibility),
            avatar_url = COALESCE($4, avatar_url),
            updated_at = NOW()
        WHERE id = $5
        RETURNING *
        "#
    )
    .bind(&body.name)
    .bind(&body.description)
    .bind(&body.visibility)
    .bind(&body.avatar_url)
    .bind(group.id)
    .fetch_one(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Ok().json(updated))
}

/// Delete a group
pub async fn delete_group(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let group_path = path.into_inner();
    
    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE path = $1"
    )
    .bind(&group_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Group not found"))?;
    
    // Only owner can delete
    let access = get_user_group_access(&pool, group.id, auth.user_id).await?;
    if access < AccessLevel::Owner {
        return Err(AppError::forbidden("Only group owner can delete the group"));
    }
    
    // Delete group (cascades to members, etc.)
    sqlx::query("DELETE FROM groups WHERE id = $1")
        .bind(group.id)
        .execute(pool.get_ref())
        .await
        .map_err(AppError::from)?;
    
    // Delete namespace
    sqlx::query("DELETE FROM namespaces WHERE id = $1")
        .bind(group.namespace_id)
        .execute(pool.get_ref())
        .await
        .map_err(AppError::from)?;
    
    Ok(HttpResponse::NoContent().finish())
}

/// List group members
pub async fn list_group_members(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let group_path = path.into_inner();
    
    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE path = $1"
    )
    .bind(&group_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Group not found"))?;
    
    let members = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = $1"
    )
    .bind(group.id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Ok().json(members))
}

/// Add a member to a group
pub async fn add_group_member(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    auth: AuthenticatedUser,
    body: web::Json<AddGroupMemberRequest>,
) -> Result<HttpResponse, AppError> {
    let group_path = path.into_inner();
    
    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE path = $1"
    )
    .bind(&group_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Group not found"))?;
    
    // Check permission
    let access = get_user_group_access(&pool, group.id, auth.user_id).await?;
    if access < AccessLevel::Maintainer {
        return Err(AppError::forbidden("Insufficient permissions"));
    }
    
    // Can't add with higher access than yourself
    if body.access_level > access {
        return Err(AppError::forbidden("Cannot grant higher access than your own"));
    }
    
    let member = sqlx::query_as::<_, GroupMember>(
        r#"
        INSERT INTO group_members (group_id, user_id, access_level, expires_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (group_id, user_id) DO UPDATE SET
            access_level = $3,
            expires_at = $4
        RETURNING *
        "#
    )
    .bind(group.id)
    .bind(body.user_id)
    .bind(body.access_level as i32)
    .bind(&body.expires_at)
    .fetch_one(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Created().json(member))
}

/// Remove a member from a group
pub async fn remove_group_member(
    pool: web::Data<PgPool>,
    path: web::Path<(String, i64)>,
    auth: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let (group_path, user_id) = path.into_inner();
    
    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE path = $1"
    )
    .bind(&group_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Group not found"))?;
    
    // Check permission
    let access = get_user_group_access(&pool, group.id, auth.user_id).await?;
    if access < AccessLevel::Maintainer {
        return Err(AppError::forbidden("Insufficient permissions"));
    }
    
    sqlx::query("DELETE FROM group_members WHERE group_id = $1 AND user_id = $2")
        .bind(group.id)
        .bind(user_id)
        .execute(pool.get_ref())
        .await
        .map_err(AppError::from)?;
    
    Ok(HttpResponse::NoContent().finish())
}

/// List projects in a group
pub async fn list_group_projects(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let group_path = path.into_inner();
    
    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE path = $1"
    )
    .bind(&group_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Group not found"))?;
    
    // Return ProjectWithOwner with namespace path as owner_name
    let projects = sqlx::query_as::<_, crate::models::project::ProjectWithOwner>(
        r#"
        SELECT p.id, p.name, p.description, p.visibility, p.owner_id, p.created_at, p.updated_at,
               n.path as owner_name, n.avatar_url as owner_avatar,
               p.stars_count, p.forks_count, p.forked_from_id,
               fn.path as forked_from_namespace, fp.name as forked_from_name
        FROM projects p
        JOIN namespaces n ON p.namespace_id = n.id
        LEFT JOIN projects fp ON p.forked_from_id = fp.id
        LEFT JOIN namespaces fn ON fp.namespace_id = fn.id
        WHERE p.namespace_id = $1 
        ORDER BY p.name
        "#
    )
    .bind(group.namespace_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Ok().json(projects))
}

/// List subgroups of a group
pub async fn list_subgroups(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let group_path = path.into_inner();
    
    let group = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE path = $1"
    )
    .bind(&group_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Group not found"))?;
    
    let subgroups = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE parent_id = $1 ORDER BY name"
    )
    .bind(group.id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Ok().json(subgroups))
}

/// List all namespaces (users + groups)
pub async fn list_namespaces(
    pool: web::Data<PgPool>,
    _auth: AuthenticatedUser,
) -> Result<HttpResponse, AppError> {
    let namespaces = sqlx::query_as::<_, NamespaceInfo>(
        r#"
        SELECT 
            id, name, path, path as full_path, namespace_type, avatar_url
        FROM namespaces
        WHERE visibility IN ('public', 'internal')
        ORDER BY name
        "#
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(AppError::from)?;
    
    Ok(HttpResponse::Ok().json(namespaces))
}

/// Get a namespace by path
pub async fn get_namespace(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let ns_path = path.into_inner();
    
    let namespace = sqlx::query_as::<_, NamespaceInfo>(
        r#"
        SELECT 
            id, name, path, path as full_path, namespace_type, avatar_url
        FROM namespaces
        WHERE path = $1
        "#
    )
    .bind(&ns_path)
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::from)?
    .ok_or_else(|| AppError::not_found("Namespace not found"))?;
    
    Ok(HttpResponse::Ok().json(namespace))
}

// Helper to get user's access level in a group
async fn get_user_group_access(
    pool: &PgPool,
    group_id: i64,
    user_id: i64,
) -> Result<AccessLevel, AppError> {
    let access = sqlx::query_scalar::<_, i32>(
        r#"
        SELECT COALESCE(
            (SELECT access_level::integer FROM group_members 
             WHERE group_id = $1 AND user_id = $2),
            0
        )
        "#
    )
    .bind(group_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)?;
    
    // Convert integer back to AccessLevel
    Ok(match access {
        50 => AccessLevel::Owner,
        40 => AccessLevel::Maintainer,
        30 => AccessLevel::Developer,
        20 => AccessLevel::Reporter,
        10 => AccessLevel::Guest,
        _ => AccessLevel::Guest,
    })
}
