//! Package Registry 内部 API Handlers
//!
//! 这些端点由 gitfox-workhorse 调用，用于处理 Docker 和 npm 注册表的数据库操作。
//! 使用 X-GitFox-Shell-Token 进行认证保护。

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use log::{debug, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::{
    Package, PackageType, PackageFile,
    DockerManifest, DockerBlob, DockerUploadSession,
    NpmPackageMetadata,
};

/// 验证内部 API token
fn verify_internal_token(req: &HttpRequest, config: &Config) -> AppResult<()> {
    let token = req
        .headers()
        .get("X-GitFox-Shell-Token")
        .and_then(|v| v.to_str().ok());

    match token {
        Some(t) if t == config.shell_secret => Ok(()),
        _ => {
            warn!("Invalid or missing internal API token for registry");
            Err(AppError::Unauthorized("Invalid internal token".to_string()))
        }
    }
}

// ============================================================================
// Docker Registry 内部 API
// ============================================================================

/// 创建 Docker 上传会话请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateUploadSessionRequest {
    pub project_id: i64,
    pub user_id: String,  // UUID string
    pub digest: Option<String>,
    pub temp_path: String,
}

/// 创建 Docker 上传会话响应
#[derive(Debug, serde::Serialize)]
pub struct CreateUploadSessionResponse {
    pub uuid: String,
    pub expires_at: chrono::DateTime<Utc>,
}

/// POST /api/internal/registry/docker/upload-session
/// 创建 Docker blob 上传会话
pub async fn create_upload_session(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<CreateUploadSessionRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let session_uuid = Uuid::new_v4().to_string();
    let user_uuid = Uuid::parse_str(&body.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id UUID".to_string()))?;
    
    let expires_at = Utc::now() + Duration::hours(24);

    sqlx::query(
        r#"
        INSERT INTO docker_upload_sessions 
            (uuid, project_id, user_id, digest, temp_path, started_at, expires_at)
        VALUES ($1, $2, $3, $4, $5, NOW(), $6)
        "#
    )
    .bind(&session_uuid)
    .bind(body.project_id)
    .bind(user_uuid)
    .bind(&body.digest)
    .bind(&body.temp_path)
    .bind(expires_at)
    .execute(pool.get_ref())
    .await?;

    info!("Created Docker upload session: {} for project {}", session_uuid, body.project_id);

    Ok(HttpResponse::Created().json(CreateUploadSessionResponse {
        uuid: session_uuid,
        expires_at,
    }))
}

/// GET /api/internal/registry/docker/upload-session/{uuid}
/// 获取 Docker 上传会话
pub async fn get_upload_session(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let uuid = path.into_inner();

    let session = sqlx::query_as::<_, DockerUploadSession>(
        r#"
        SELECT id, uuid, project_id, user_id, digest, uploaded_bytes, temp_path, started_at, expires_at
        FROM docker_upload_sessions
        WHERE uuid = $1 AND expires_at > NOW()
        "#
    )
    .bind(&uuid)
    .fetch_optional(pool.get_ref())
    .await?;

    match session {
        Some(s) => Ok(HttpResponse::Ok().json(s)),
        None => Err(AppError::NotFound("Upload session not found or expired".to_string())),
    }
}

/// 更新上传会话进度请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateUploadProgressRequest {
    pub uploaded_bytes: i64,
}

/// PATCH /api/internal/registry/docker/upload-session/{uuid}
/// 更新 Docker 上传会话进度
pub async fn update_upload_session(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<String>,
    body: web::Json<UpdateUploadProgressRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let uuid = path.into_inner();

    let result = sqlx::query(
        r#"
        UPDATE docker_upload_sessions
        SET uploaded_bytes = $1
        WHERE uuid = $2 AND expires_at > NOW()
        "#
    )
    .bind(body.uploaded_bytes)
    .bind(&uuid)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Upload session not found or expired".to_string()));
    }

    Ok(HttpResponse::Ok().finish())
}

/// DELETE /api/internal/registry/docker/upload-session/{uuid}
/// 删除 Docker 上传会话
pub async fn delete_upload_session(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let uuid = path.into_inner();

    sqlx::query("DELETE FROM docker_upload_sessions WHERE uuid = $1")
        .bind(&uuid)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

/// 创建 Docker Blob 请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateDockerBlobRequest {
    pub project_id: i64,
    pub digest: String,
    pub media_type: Option<String>,
    pub size: i64,
    pub file_path: String,
}

/// POST /api/internal/registry/docker/blob
/// 创建 Docker blob 记录
pub async fn create_docker_blob(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<CreateDockerBlobRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    // 检查是否已存在（去重）
    let existing = sqlx::query_as::<_, DockerBlob>(
        r#"
        SELECT id, project_id, digest, media_type, size, file_path, created_at
        FROM docker_blobs
        WHERE project_id = $1 AND digest = $2
        "#
    )
    .bind(body.project_id)
    .bind(&body.digest)
    .fetch_optional(pool.get_ref())
    .await?;

    if let Some(blob) = existing {
        debug!("Blob already exists: {} for project {}", body.digest, body.project_id);
        return Ok(HttpResponse::Ok().json(blob));
    }

    // 创建新 blob
    let blob = sqlx::query_as::<_, DockerBlob>(
        r#"
        INSERT INTO docker_blobs (project_id, digest, media_type, size, file_path, created_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        RETURNING id, project_id, digest, media_type, size, file_path, created_at
        "#
    )
    .bind(body.project_id)
    .bind(&body.digest)
    .bind(&body.media_type)
    .bind(body.size)
    .bind(&body.file_path)
    .fetch_one(pool.get_ref())
    .await?;

    info!("Created Docker blob: {} for project {}", body.digest, body.project_id);

    Ok(HttpResponse::Created().json(blob))
}

/// GET /api/internal/registry/docker/blob/{project_id}/{digest}
/// 获取 Docker blob 记录
pub async fn get_docker_blob(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, digest) = path.into_inner();

    let blob = sqlx::query_as::<_, DockerBlob>(
        r#"
        SELECT id, project_id, digest, media_type, size, file_path, created_at
        FROM docker_blobs
        WHERE project_id = $1 AND digest = $2
        "#
    )
    .bind(project_id)
    .bind(&digest)
    .fetch_optional(pool.get_ref())
    .await?;

    match blob {
        Some(b) => Ok(HttpResponse::Ok().json(b)),
        None => Err(AppError::NotFound("Blob not found".to_string())),
    }
}

/// DELETE /api/internal/registry/docker/blob/{project_id}/{digest}
/// 删除 Docker blob 记录
/// 
/// 删除前检查引用计数，如果 blob 仍被 manifest 引用则返回 409 Conflict。
/// 成功删除后返回 blob 信息（包含 file_path），由调用方负责删除物理文件。
pub async fn delete_docker_blob(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, digest) = path.into_inner();

    // 首先查找 blob
    let blob = sqlx::query_as::<_, DockerBlob>(
        r#"
        SELECT id, project_id, digest, media_type, size, file_path, created_at
        FROM docker_blobs
        WHERE project_id = $1 AND digest = $2
        "#
    )
    .bind(project_id)
    .bind(&digest)
    .fetch_optional(pool.get_ref())
    .await?;

    let blob = match blob {
        Some(b) => b,
        None => return Err(AppError::NotFound("Blob not found".to_string())),
    };

    // 检查引用计数：查询 docker_manifest_blobs 中是否有 manifest 引用此 blob
    let reference_count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) as count
        FROM docker_manifest_blobs
        WHERE blob_id = $1
        "#
    )
    .bind(blob.id)
    .fetch_one(pool.get_ref())
    .await?;

    if reference_count.0 > 0 {
        warn!(
            "Cannot delete blob {} (id={}): still referenced by {} manifest(s)",
            digest, blob.id, reference_count.0
        );
        return Err(AppError::Conflict(format!(
            "Blob is still referenced by {} manifest(s)",
            reference_count.0
        )));
    }

    // 无引用，可以安全删除
    sqlx::query("DELETE FROM docker_blobs WHERE id = $1")
        .bind(blob.id)
        .execute(pool.get_ref())
        .await?;

    info!(
        "Deleted Docker blob: {} (id={}) for project {}",
        digest, blob.id, project_id
    );

    // 返回被删除的 blob 信息，调用方可以用 file_path 删除物理文件
    Ok(HttpResponse::Ok().json(blob))
}

/// 创建 Docker Manifest 请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateDockerManifestRequest {
    pub project_id: i64,
    pub image_name: String,  // e.g., "nginx"
    pub tag: String,         // e.g., "latest" or digest
    pub digest: String,
    pub media_type: String,
    pub schema_version: i32,
    pub config_digest: Option<String>,
    pub total_size: i64,
    pub manifest_json: serde_json::Value,
    pub blob_digests: Vec<String>,  // 关联的 blob digests
}

/// POST /api/internal/registry/docker/manifest
/// 创建或更新 Docker manifest
pub async fn create_docker_manifest(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<CreateDockerManifestRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let mut tx = pool.begin().await?;

    // 查找或创建 package
    let package = sqlx::query_as::<_, Package>(
        r#"
        SELECT id, project_id, name, package_type, version, status, metadata, created_at, updated_at
        FROM packages
        WHERE project_id = $1 AND package_type = 'docker' AND name = $2 AND version = $3
        "#
    )
    .bind(body.project_id)
    .bind(&body.image_name)
    .bind(&body.tag)
    .fetch_optional(&mut *tx)
    .await?;

    let package_id = if let Some(pkg) = package {
        // 更新现有包的更新时间
        sqlx::query("UPDATE packages SET updated_at = NOW() WHERE id = $1")
            .bind(pkg.id)
            .execute(&mut *tx)
            .await?;
        pkg.id
    } else {
        // 创建新包
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO packages (project_id, name, package_type, version, status, metadata, created_at, updated_at)
            VALUES ($1, $2, 'docker', $3, 'default', '{}', NOW(), NOW())
            RETURNING id
            "#
        )
        .bind(body.project_id)
        .bind(&body.image_name)
        .bind(&body.tag)
        .fetch_one(&mut *tx)
        .await?;
        row.0
    };

    // 删除旧的 manifest（如果存在相同 tag）
    sqlx::query("DELETE FROM docker_manifests WHERE package_id = $1")
        .bind(package_id)
        .execute(&mut *tx)
        .await?;

    // 创建新 manifest
    let manifest = sqlx::query_as::<_, DockerManifest>(
        r#"
        INSERT INTO docker_manifests 
            (package_id, digest, media_type, schema_version, config_digest, total_size, manifest_json, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        RETURNING id, package_id, digest, media_type, schema_version, config_digest, total_size, manifest_json, created_at
        "#
    )
    .bind(package_id)
    .bind(&body.digest)
    .bind(&body.media_type)
    .bind(body.schema_version)
    .bind(&body.config_digest)
    .bind(body.total_size)
    .bind(&body.manifest_json)
    .fetch_one(&mut *tx)
    .await?;

    // 关联 blobs
    for blob_digest in &body.blob_digests {
        // 查找 blob
        let blob = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM docker_blobs WHERE project_id = $1 AND digest = $2"
        )
        .bind(body.project_id)
        .bind(blob_digest)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((blob_id,)) = blob {
            // 忽略重复错误
            let _ = sqlx::query(
                "INSERT INTO docker_manifest_blobs (manifest_id, blob_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            )
            .bind(manifest.id)
            .bind(blob_id)
            .execute(&mut *tx)
            .await;
        }
    }

    tx.commit().await?;

    info!(
        "Created/updated Docker manifest: {}:{} (digest: {}) for project {}",
        body.image_name, body.tag, body.digest, body.project_id
    );

    Ok(HttpResponse::Created().json(manifest))
}

/// Docker Manifest 查询参数
#[derive(Debug, serde::Deserialize)]
pub struct DockerManifestQuery {
    /// 按 tag 查询（可选）
    pub tag: Option<String>,
    /// 按 digest 查询（可选）
    pub digest: Option<String>,
}

/// GET /api/internal/registry/docker/manifest/{project_id}/{image_name}
/// 获取 Docker manifest
pub async fn get_docker_manifest(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String)>,
    query: web::Query<DockerManifestQuery>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, image_name) = path.into_inner();

    // 构建查询条件
    let manifest = if let Some(ref digest) = query.digest {
        // 按 digest 查询
        sqlx::query_as::<_, DockerManifest>(
            r#"
            SELECT m.id, m.package_id, m.digest, m.media_type, m.schema_version, 
                   m.config_digest, m.total_size, m.manifest_json, m.created_at
            FROM docker_manifests m
            JOIN packages p ON m.package_id = p.id
            WHERE p.project_id = $1 AND p.name = $2 AND p.package_type = 'docker' AND m.digest = $3
            "#
        )
        .bind(project_id)
        .bind(&image_name)
        .bind(digest)
        .fetch_optional(pool.get_ref())
        .await?
    } else if let Some(ref tag) = query.tag {
        // 按 tag 查询
        sqlx::query_as::<_, DockerManifest>(
            r#"
            SELECT m.id, m.package_id, m.digest, m.media_type, m.schema_version, 
                   m.config_digest, m.total_size, m.manifest_json, m.created_at
            FROM docker_manifests m
            JOIN packages p ON m.package_id = p.id
            WHERE p.project_id = $1 AND p.name = $2 AND p.package_type = 'docker' AND p.version = $3
            "#
        )
        .bind(project_id)
        .bind(&image_name)
        .bind(tag)
        .fetch_optional(pool.get_ref())
        .await?
    } else {
        return Err(AppError::BadRequest("Either tag or digest must be specified".to_string()));
    };

    match manifest {
        Some(m) => Ok(HttpResponse::Ok().json(m)),
        None => Err(AppError::NotFound("Manifest not found".to_string())),
    }
}

/// DELETE /api/internal/registry/docker/manifest/{project_id}/{image_name}
/// 删除 Docker manifest
pub async fn delete_docker_manifest(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String)>,
    query: web::Query<DockerManifestQuery>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, image_name) = path.into_inner();

    // 构建删除条件
    let result = if let Some(ref digest) = query.digest {
        sqlx::query(
            r#"
            DELETE FROM packages
            WHERE id IN (
                SELECT p.id FROM packages p
                JOIN docker_manifests m ON m.package_id = p.id
                WHERE p.project_id = $1 AND p.name = $2 AND p.package_type = 'docker' AND m.digest = $3
            )
            "#
        )
        .bind(project_id)
        .bind(&image_name)
        .bind(digest)
        .execute(pool.get_ref())
        .await?
    } else if let Some(ref tag) = query.tag {
        sqlx::query(
            "DELETE FROM packages WHERE project_id = $1 AND name = $2 AND package_type = 'docker' AND version = $3"
        )
        .bind(project_id)
        .bind(&image_name)
        .bind(tag)
        .execute(pool.get_ref())
        .await?
    } else {
        return Err(AppError::BadRequest("Either tag or digest must be specified".to_string()));
    };

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Manifest not found".to_string()));
    }

    info!("Deleted Docker manifest: {} for project {}", image_name, project_id);

    Ok(HttpResponse::NoContent().finish())
}

/// Docker Tag 列表响应
#[derive(Debug, serde::Serialize)]
pub struct DockerTagListResponse {
    pub name: String,
    pub tags: Vec<String>,
}

/// GET /api/internal/registry/docker/tags/{project_id}/{image_name}
/// 获取 Docker 镜像的标签列表
pub async fn list_docker_tags(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, image_name) = path.into_inner();

    let tags = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT version
        FROM packages
        WHERE project_id = $1 AND package_type = 'docker' AND name = $2 AND status = 'default'
        ORDER BY created_at DESC
        "#
    )
    .bind(project_id)
    .bind(&image_name)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(DockerTagListResponse {
        name: image_name,
        tags: tags.into_iter().map(|(t,)| t).collect(),
    }))
}

/// Docker 仓库目录响应
#[derive(Debug, serde::Serialize)]
pub struct DockerCatalogResponse {
    pub repositories: Vec<String>,
}

/// GET /api/internal/registry/docker/catalog/{project_id}
/// 获取项目下的 Docker 镜像列表
pub async fn list_docker_repositories(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<i64>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let project_id = path.into_inner();

    let repos = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT DISTINCT name
        FROM packages
        WHERE project_id = $1 AND package_type = 'docker' AND status = 'default'
        ORDER BY name
        "#
    )
    .bind(project_id)
    .fetch_all(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(DockerCatalogResponse {
        repositories: repos.into_iter().map(|(r,)| r).collect(),
    }))
}

// ============================================================================
// npm Registry 内部 API
// ============================================================================

/// 创建/更新 npm 包请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateNpmPackageRequest {
    pub project_id: i64,
    pub name: String,
    pub version: String,
    pub dist_tag: String,
    pub tarball_sha512: Option<String>,
    pub readme: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub repository: Option<serde_json::Value>,
    pub dependencies: Option<serde_json::Value>,
    pub dev_dependencies: Option<serde_json::Value>,
    pub peer_dependencies: Option<serde_json::Value>,
    pub file_path: String,
    pub file_size: i64,
}

/// POST /api/internal/registry/npm/package
/// 创建 npm 包
pub async fn create_npm_package(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<CreateNpmPackageRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    // 检查版本是否已存在
    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM packages WHERE project_id = $1 AND package_type = 'npm' AND name = $2 AND version = $3"
    )
    .bind(body.project_id)
    .bind(&body.name)
    .bind(&body.version)
    .fetch_optional(pool.get_ref())
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(format!(
            "Package {} version {} already exists",
            body.name, body.version
        )));
    }

    let mut tx = pool.begin().await?;

    // 创建 package
    let package = sqlx::query_as::<_, Package>(
        r#"
        INSERT INTO packages (project_id, name, package_type, version, status, metadata, created_at, updated_at)
        VALUES ($1, $2, 'npm', $3, 'default', '{}', NOW(), NOW())
        RETURNING id, project_id, name, package_type, version, status, metadata, created_at, updated_at
        "#
    )
    .bind(body.project_id)
    .bind(&body.name)
    .bind(&body.version)
    .fetch_one(&mut *tx)
    .await?;

    // 创建 package_file
    let file_name = format!("{}-{}.tgz", body.name.replace('/', "-"), body.version);
    sqlx::query(
        r#"
        INSERT INTO package_files (package_id, file_name, file_path, file_size, file_sha256, created_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        "#
    )
    .bind(package.id)
    .bind(&file_name)
    .bind(&body.file_path)
    .bind(body.file_size)
    .bind(body.tarball_sha512.as_deref().unwrap_or(""))
    .execute(&mut *tx)
    .await?;

    // 创建 npm_package_metadata
    sqlx::query(
        r#"
        INSERT INTO npm_package_metadata 
            (package_id, dist_tag, tarball_sha512, npm_readme, npm_keywords, npm_license, 
             npm_repository, npm_dependencies, npm_dev_dependencies, npm_peer_dependencies, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        "#
    )
    .bind(package.id)
    .bind(&body.dist_tag)
    .bind(&body.tarball_sha512)
    .bind(&body.readme)
    .bind(&body.keywords)
    .bind(&body.license)
    .bind(&body.repository)
    .bind(&body.dependencies)
    .bind(&body.dev_dependencies)
    .bind(&body.peer_dependencies)
    .execute(&mut *tx)
    .await?;

    // 更新 dist-tag
    sqlx::query(
        r#"
        INSERT INTO npm_dist_tags (project_id, package_name, tag, version, updated_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (project_id, package_name, tag) 
        DO UPDATE SET version = EXCLUDED.version, updated_at = NOW()
        "#
    )
    .bind(body.project_id)
    .bind(&body.name)
    .bind(&body.dist_tag)
    .bind(&body.version)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!(
        "Created npm package: {}@{} for project {}",
        body.name, body.version, body.project_id
    );

    Ok(HttpResponse::Created().json(package))
}

/// npm 包信息响应
#[derive(Debug, serde::Serialize)]
pub struct NpmPackageInfoResponse {
    pub package: Package,
    pub metadata: Option<NpmPackageMetadata>,
    pub file: Option<PackageFile>,
}

/// GET /api/internal/registry/npm/package/{project_id}/{name}
/// 获取 npm 包信息
pub async fn get_npm_package(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String)>,
    query: web::Query<NpmPackageQuery>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, name) = path.into_inner();

    // 确定要查询的版本
    let version = if let Some(ref v) = query.version {
        v.clone()
    } else if let Some(ref tag) = query.tag {
        // 查找 dist-tag 对应的版本
        let tag_row = sqlx::query_as::<_, (String,)>(
            "SELECT version FROM npm_dist_tags WHERE project_id = $1 AND package_name = $2 AND tag = $3"
        )
        .bind(project_id)
        .bind(&name)
        .bind(tag)
        .fetch_optional(pool.get_ref())
        .await?;

        match tag_row {
            Some((v,)) => v,
            None => return Err(AppError::NotFound(format!("Tag {} not found for package {}", tag, name))),
        }
    } else {
        // 默认使用 latest
        let tag_row = sqlx::query_as::<_, (String,)>(
            "SELECT version FROM npm_dist_tags WHERE project_id = $1 AND package_name = $2 AND tag = 'latest'"
        )
        .bind(project_id)
        .bind(&name)
        .fetch_optional(pool.get_ref())
        .await?;

        match tag_row {
            Some((v,)) => v,
            None => return Err(AppError::NotFound(format!("Package {} not found", name))),
        }
    };

    // 查询包
    let package = sqlx::query_as::<_, Package>(
        "SELECT * FROM packages WHERE project_id = $1 AND package_type = 'npm' AND name = $2 AND version = $3"
    )
    .bind(project_id)
    .bind(&name)
    .bind(&version)
    .fetch_optional(pool.get_ref())
    .await?;

    let package = match package {
        Some(p) => p,
        None => return Err(AppError::NotFound(format!("Package {}@{} not found", name, version))),
    };

    // 查询元数据
    let metadata = sqlx::query_as::<_, NpmPackageMetadata>(
        "SELECT * FROM npm_package_metadata WHERE package_id = $1"
    )
    .bind(package.id)
    .fetch_optional(pool.get_ref())
    .await?;

    // 查询文件
    let file = sqlx::query_as::<_, PackageFile>(
        "SELECT * FROM package_files WHERE package_id = $1 LIMIT 1"
    )
    .bind(package.id)
    .fetch_optional(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().json(NpmPackageInfoResponse {
        package,
        metadata,
        file,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct NpmPackageQuery {
    pub version: Option<String>,
    pub tag: Option<String>,
}

/// npm 包完整文档响应（所有版本）
#[derive(Debug, serde::Serialize)]
pub struct NpmPackageDocumentResponse {
    pub name: String,
    pub dist_tags: std::collections::HashMap<String, String>,
    pub versions: Vec<NpmVersionSummary>,
}

#[derive(Debug, serde::Serialize)]
pub struct NpmVersionSummary {
    pub version: String,
    pub created_at: chrono::DateTime<Utc>,
    pub tarball_url: String,
    pub integrity: Option<String>,
}

/// npm 包查找响应
#[derive(Debug, serde::Serialize)]
pub struct NpmPackageLookupResponse {
    pub project_id: i64,
    pub name: String,
    pub dist_tags: std::collections::HashMap<String, String>,
    pub versions: Vec<NpmVersionSummary>,
}

/// GET /api/internal/registry/npm/lookup/{scope}/{name}
/// 按 scope 和 name 查找 npm 包
/// 
/// 在指定 scope (namespace) 下查找包，无需知道具体 project_id。
/// 这解决了"简化实现"的问题：允许 @scope/name 包属于 scope namespace 下的任何项目。
pub async fn lookup_npm_package(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (scope, name) = path.into_inner();
    let full_name = format!("@{}/{}", scope, name);

    // 查找 scope (namespace) 的 ID
    let namespace: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM namespaces WHERE path = $1"
    )
    .bind(&scope)
    .fetch_optional(pool.get_ref())
    .await?;

    let namespace_id = match namespace {
        Some((id,)) => id,
        None => return Err(AppError::NotFound(format!("Namespace {} not found", scope))),
    };

    // 在 namespace 下的所有项目中查找包
    // 使用 DISTINCT ON 获取每个版本
    let packages = sqlx::query_as::<_, (i64, i64, String, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT p.id, p.project_id, p.version, p.created_at
        FROM packages p
        INNER JOIN projects proj ON p.project_id = proj.id
        WHERE proj.namespace_id = $1 
          AND p.package_type = 'npm' 
          AND p.name = $2 
          AND p.status = 'default'
        ORDER BY p.created_at DESC
        "#
    )
    .bind(namespace_id)
    .bind(&full_name)
    .fetch_all(pool.get_ref())
    .await?;

    if packages.is_empty() {
        return Err(AppError::NotFound(format!("Package {} not found in namespace {}", full_name, scope)));
    }

    // 获取 project_id（所有版本应该属于同一个项目）
    let project_id = packages[0].1;

    // 查询 dist-tags
    let tags = sqlx::query_as::<_, (String, String)>(
        "SELECT tag, version FROM npm_dist_tags WHERE project_id = $1 AND package_name = $2"
    )
    .bind(project_id)
    .bind(&full_name)
    .fetch_all(pool.get_ref())
    .await?;

    let dist_tags: std::collections::HashMap<String, String> = tags.into_iter().collect();

    // 构建版本列表
    let mut versions = Vec::new();
    for (pkg_id, _proj_id, version, created_at) in packages {
        let metadata: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT tarball_sha512 FROM npm_package_metadata WHERE package_id = $1"
        )
        .bind(pkg_id)
        .fetch_optional(pool.get_ref())
        .await?;

        versions.push(NpmVersionSummary {
            version,
            created_at,
            tarball_url: String::new(), // Workhorse 会填充实际 URL
            integrity: metadata.and_then(|m| m.0),
        });
    }

    Ok(HttpResponse::Ok().json(NpmPackageLookupResponse {
        project_id,
        name: full_name,
        dist_tags,
        versions,
    }))
}

/// GET /api/internal/registry/npm/package-doc/{project_id}/{name}
/// 获取 npm 包完整文档（所有版本）
pub async fn get_npm_package_document(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, name) = path.into_inner();

    // 查询所有版本
    let packages = sqlx::query_as::<_, Package>(
        r#"
        SELECT * FROM packages 
        WHERE project_id = $1 AND package_type = 'npm' AND name = $2 AND status = 'default'
        ORDER BY created_at DESC
        "#
    )
    .bind(project_id)
    .bind(&name)
    .fetch_all(pool.get_ref())
    .await?;

    if packages.is_empty() {
        return Err(AppError::NotFound(format!("Package {} not found", name)));
    }

    // 查询 dist-tags
    let tags = sqlx::query_as::<_, (String, String)>(
        "SELECT tag, version FROM npm_dist_tags WHERE project_id = $1 AND package_name = $2"
    )
    .bind(project_id)
    .bind(&name)
    .fetch_all(pool.get_ref())
    .await?;

    let dist_tags: std::collections::HashMap<String, String> = tags.into_iter().collect();

    // 构建版本列表
    let mut versions = Vec::new();
    for pkg in packages {
        let metadata = sqlx::query_as::<_, NpmPackageMetadata>(
            "SELECT * FROM npm_package_metadata WHERE package_id = $1"
        )
        .bind(pkg.id)
        .fetch_optional(pool.get_ref())
        .await?;

        versions.push(NpmVersionSummary {
            version: pkg.version,
            created_at: pkg.created_at,
            tarball_url: String::new(), // Workhorse 会填充实际 URL
            integrity: metadata.and_then(|m| m.tarball_sha512),
        });
    }

    Ok(HttpResponse::Ok().json(NpmPackageDocumentResponse {
        name,
        dist_tags,
        versions,
    }))
}

/// DELETE /api/internal/registry/npm/package/{project_id}/{name}/{version}
/// 删除 npm 包版本
pub async fn delete_npm_package(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, name, version) = path.into_inner();

    let result = sqlx::query(
        "DELETE FROM packages WHERE project_id = $1 AND package_type = 'npm' AND name = $2 AND version = $3"
    )
    .bind(project_id)
    .bind(&name)
    .bind(&version)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Package {}@{} not found", name, version)));
    }

    // 清理 dist-tags 中指向此版本的标签
    sqlx::query(
        "DELETE FROM npm_dist_tags WHERE project_id = $1 AND package_name = $2 AND version = $3"
    )
    .bind(project_id)
    .bind(&name)
    .bind(&version)
    .execute(pool.get_ref())
    .await?;

    info!("Deleted npm package: {}@{} from project {}", name, version, project_id);

    Ok(HttpResponse::NoContent().finish())
}

/// 更新 npm dist-tag 请求
#[derive(Debug, serde::Deserialize)]
pub struct UpdateNpmDistTagRequest {
    pub project_id: i64,
    pub package_name: String,
    pub tag: String,
    pub version: String,
}

/// PUT /api/internal/registry/npm/dist-tag
/// 更新 npm dist-tag
pub async fn update_npm_dist_tag(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<UpdateNpmDistTagRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    // 验证版本存在
    let exists = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM packages WHERE project_id = $1 AND package_type = 'npm' AND name = $2 AND version = $3"
    )
    .bind(body.project_id)
    .bind(&body.package_name)
    .bind(&body.version)
    .fetch_optional(pool.get_ref())
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound(format!(
            "Package {}@{} not found",
            body.package_name, body.version
        )));
    }

    sqlx::query(
        r#"
        INSERT INTO npm_dist_tags (project_id, package_name, tag, version, updated_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (project_id, package_name, tag) 
        DO UPDATE SET version = EXCLUDED.version, updated_at = NOW()
        "#
    )
    .bind(body.project_id)
    .bind(&body.package_name)
    .bind(&body.tag)
    .bind(&body.version)
    .execute(pool.get_ref())
    .await?;

    Ok(HttpResponse::Ok().finish())
}

/// DELETE /api/internal/registry/npm/dist-tag/{project_id}/{package_name}/{tag}
/// 删除 npm dist-tag
pub async fn delete_npm_dist_tag(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, package_name, tag) = path.into_inner();

    // 不允许删除 latest 标签
    if tag == "latest" {
        return Err(AppError::BadRequest("Cannot delete 'latest' dist-tag".to_string()));
    }

    let result = sqlx::query(
        "DELETE FROM npm_dist_tags WHERE project_id = $1 AND package_name = $2 AND tag = $3"
    )
    .bind(project_id)
    .bind(&package_name)
    .bind(&tag)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Dist-tag {} not found", tag)));
    }

    Ok(HttpResponse::NoContent().finish())
}

// ============================================================================
// 通用 Package API
// ============================================================================

/// GET /api/internal/registry/packages/{project_id}
/// 获取项目的所有包列表
pub async fn list_project_packages(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<i64>,
    query: web::Query<ListPackagesQuery>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let project_id = path.into_inner();

    let packages = if let Some(ref pkg_type) = query.package_type {
        sqlx::query_as::<_, Package>(
            r#"
            SELECT * FROM packages 
            WHERE project_id = $1 AND package_type = $2 AND status = 'default'
            ORDER BY name, created_at DESC
            "#
        )
        .bind(project_id)
        .bind(pkg_type)
        .fetch_all(pool.get_ref())
        .await?
    } else {
        sqlx::query_as::<_, Package>(
            r#"
            SELECT * FROM packages 
            WHERE project_id = $1 AND status = 'default'
            ORDER BY package_type, name, created_at DESC
            "#
        )
        .bind(project_id)
        .fetch_all(pool.get_ref())
        .await?
    };

    Ok(HttpResponse::Ok().json(packages))
}

#[derive(Debug, serde::Deserialize)]
pub struct ListPackagesQuery {
    pub package_type: Option<PackageType>,
}

/// 解析项目路径获取 project_id
/// GET /api/internal/registry/resolve-project/{namespace}/{project}
pub async fn resolve_project(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, project) = path.into_inner();

    // 查找命名空间
    let ns = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM namespaces WHERE path = $1"
    )
    .bind(&namespace)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Namespace {} not found", namespace)))?;

    // 查找项目
    let project = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name FROM projects WHERE namespace_id = $1 AND name = $2"
    )
    .bind(ns.0)
    .bind(&project)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound("Project not found".to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "project_id": project.0,
        "project_name": project.1,
        "namespace": namespace
    })))
}

// ============================================================================
// npm 登录与认证 API
// ============================================================================

/// npm 登录请求
#[derive(Debug, serde::Deserialize)]
pub struct NpmLoginRequest {
    pub username: String,
    pub password: String,
}

/// npm 登录响应
#[derive(Debug, serde::Serialize)]
pub struct NpmLoginResponse {
    pub ok: bool,
    pub token: String,
    pub username: String,
}

/// POST /api/internal/registry/npm/login
/// 验证 npm 用户凭据并生成 token（使用 PAT 认证模式）
pub async fn npm_login(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<NpmLoginRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    // 查找用户
    let user = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, username, password_hash FROM users WHERE username = $1 OR email = $1"
    )
    .bind(&body.username)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    let (user_id, username, password_hash) = user;

    // 验证密码（使用 bcrypt）
    if !bcrypt::verify(&body.password, &password_hash)
        .map_err(|_| AppError::InternalError("Password verification failed".to_string()))? 
    {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 生成 npm 专用 PAT (使用固定前缀便于识别)
    // token 格式: gitfox-npm_<random>
    let token_value = format!("gitfox-npm_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let token_hash = {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token_value.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    // 在数据库中创建 PAT
    let expires_at = chrono::Utc::now() + chrono::Duration::days(365);
    sqlx::query(
        r#"
        INSERT INTO personal_access_tokens (user_id, name, token_hash, scopes, expires_at, created_at, last_used_at)
        VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
        "#
    )
    .bind(user_id)
    .bind(format!("npm-cli-{}", chrono::Utc::now().timestamp()))
    .bind(&token_hash)
    .bind(&["read_registry", "write_registry"] as &[&str])
    .bind(expires_at)
    .execute(pool.get_ref())
    .await?;

    info!("Created npm PAT for user {} (id: {})", username, user_id);

    Ok(HttpResponse::Created().json(NpmLoginResponse {
        ok: true,
        token: token_value,
        username,
    }))
}

/// npm whoami 响应
#[derive(Debug, serde::Serialize)]
pub struct NpmWhoamiResponse {
    pub username: String,
}

/// GET /api/internal/registry/npm/whoami
/// 从 Authorization header 中的 token 解析用户信息
pub async fn npm_whoami(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    // 从 Authorization header 获取 token
    let auth_header = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid Authorization format".to_string()))?;

    // 计算 token hash
    let token_hash = {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    // 查找 PAT 及关联用户
    let result = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT u.username 
        FROM personal_access_tokens pat
        JOIN users u ON pat.user_id = u.id
        WHERE pat.token_hash = $1 
          AND (pat.expires_at IS NULL OR pat.expires_at > NOW())
        "#
    )
    .bind(&token_hash)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid or expired token".to_string()))?;

    // 更新最后使用时间
    sqlx::query("UPDATE personal_access_tokens SET last_used_at = NOW() WHERE token_hash = $1")
        .bind(&token_hash)
        .execute(pool.get_ref())
        .await?;

    Ok(HttpResponse::Ok().json(NpmWhoamiResponse {
        username: result.0,
    }))
}

/// GET /api/internal/registry/npm/dist-tags/{project_id}/{package_name}
/// 获取 npm 包的所有 dist-tags
pub async fn get_npm_dist_tags(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(i64, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (project_id, package_name) = path.into_inner();

    let tags = sqlx::query_as::<_, (String, String)>(
        "SELECT tag, version FROM npm_dist_tags WHERE project_id = $1 AND package_name = $2"
    )
    .bind(project_id)
    .bind(&package_name)
    .fetch_all(pool.get_ref())
    .await?;

    let tags: std::collections::HashMap<String, String> = tags.into_iter().collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "tags": tags
    })))
}

/// npm 搜索查询参数
#[derive(Debug, serde::Deserialize)]
pub struct NpmSearchQuery {
    pub q: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// npm 搜索结果项
#[derive(Debug, serde::Serialize)]
pub struct NpmSearchResultItem {
    pub name: String,
    pub scope: Option<String>,
    pub version: String,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub date: String,
    pub links: NpmSearchResultLinks,
    pub publisher: Option<NpmSearchPublisher>,
}

/// npm 搜索结果链接
#[derive(Debug, serde::Serialize)]
pub struct NpmSearchResultLinks {
    pub npm: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub bugs: Option<String>,
}

/// npm 搜索发布者
#[derive(Debug, serde::Serialize)]
pub struct NpmSearchPublisher {
    pub username: String,
    pub email: Option<String>,
}

/// GET /api/internal/registry/npm/search
/// 搜索 npm 包
pub async fn search_npm_packages(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    query: web::Query<NpmSearchQuery>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let search_term = query.q.as_deref().unwrap_or("");
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    // 搜索 npm 包（按名称、关键词）
    // 使用 DISTINCT ON 获取每个包的最新版本
    let packages = sqlx::query_as::<_, (i64, String, String, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT DISTINCT ON (p.name) p.id, p.name, p.version, p.created_at
        FROM packages p
        LEFT JOIN npm_package_metadata npm ON p.id = npm.package_id
        WHERE p.package_type = 'npm' 
          AND p.status = 'default'
          AND (
              $1 = '' 
              OR p.name ILIKE '%' || $1 || '%'
              OR EXISTS (
                  SELECT 1 FROM unnest(npm.npm_keywords) AS kw 
                  WHERE kw ILIKE '%' || $1 || '%'
              )
          )
        ORDER BY p.name, p.created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(search_term)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool.get_ref())
    .await?;

    // 获取总数
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(DISTINCT p.name)::bigint
        FROM packages p
        LEFT JOIN npm_package_metadata npm ON p.id = npm.package_id
        WHERE p.package_type = 'npm' 
          AND p.status = 'default'
          AND (
              $1 = '' 
              OR p.name ILIKE '%' || $1 || '%'
              OR EXISTS (
                  SELECT 1 FROM unnest(npm.npm_keywords) AS kw 
                  WHERE kw ILIKE '%' || $1 || '%'
              )
          )
        "#
    )
    .bind(search_term)
    .fetch_one(pool.get_ref())
    .await?;

    // 构建结果
    let mut results = Vec::new();
    for (pkg_id, name, version, created_at) in packages {
        // 获取元数据
        let metadata = sqlx::query_as::<_, (Option<Vec<String>>, Option<String>, Option<serde_json::Value>)>(
            "SELECT npm_keywords, npm_license, npm_repository FROM npm_package_metadata WHERE package_id = $1"
        )
        .bind(pkg_id)
        .fetch_optional(pool.get_ref())
        .await?;

        let (keywords, _license, repository) = metadata.unwrap_or((None, None, None));

        // 解析 scope
        let (scope, _pkg_name) = if name.starts_with('@') {
            let parts: Vec<&str> = name.splitn(2, '/').collect();
            if parts.len() == 2 {
                (Some(parts[0].trim_start_matches('@').to_string()), parts[1].to_string())
            } else {
                (None, name.clone())
            }
        } else {
            (None, name.clone())
        };

        // 提取 repository URL
        let repo_url = repository.as_ref()
            .and_then(|r| r.get("url"))
            .and_then(|u| u.as_str())
            .map(String::from);

        results.push(NpmSearchResultItem {
            name: name.clone(),
            scope,
            version,
            description: None, // npm_package_metadata 暂未存储 description
            keywords,
            date: created_at.to_rfc3339(),
            links: NpmSearchResultLinks {
                npm: None,
                homepage: None,
                repository: repo_url,
                bugs: None,
            },
            publisher: None, // 需要关联到发布者
        });
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "packages": results,
        "total": total.0
    })))
}

// ============================================================================
// Cargo Registry 内部 API
// ============================================================================

/// Cargo 索引条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CargoIndexEntry {
    pub name: String,
    pub vers: String,
    pub deps: Vec<CargoIndexDependency>,
    pub cksum: String,
    pub features: serde_json::Value,
    pub features2: Option<serde_json::Value>,
    pub yanked: bool,
    pub links: Option<String>,
    pub v: Option<i32>,
    pub rust_version: Option<String>,
}

/// Cargo 索引依赖
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CargoIndexDependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: String,
    pub registry: Option<String>,
    pub package: Option<String>,
}

/// GET /api/internal/registry/cargo/index/{namespace}/{crate_name}
/// 获取 Cargo 索引条目（所有版本）
pub async fn get_cargo_index(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, crate_name) = path.into_inner();

    // 获取 crate 的所有版本
    let versions = sqlx::query_as::<_, (String, String, bool, String, serde_json::Value)>(
        r#"
        SELECT p.version, cm.checksum, cm.yanked, cm.links, cm.features
        FROM packages p
        JOIN cargo_crate_metadata cm ON p.id = cm.package_id
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE p.package_type = 'cargo'
          AND p.name = $1
          AND ns.path = $2
          AND p.status = 'default'
        ORDER BY p.created_at ASC
        "#
    )
    .bind(&crate_name)
    .bind(&namespace)
    .fetch_all(pool.get_ref())
    .await?;

    if versions.is_empty() {
        return Err(AppError::NotFound(format!("Crate {} not found in namespace {}", crate_name, namespace)));
    }

    let mut entries = Vec::new();
    for (version, checksum, yanked, links, features) in versions {
        // 获取依赖
        let deps = sqlx::query_as::<_, (String, String, Vec<String>, bool, bool, Option<String>, String, Option<String>, Option<String>)>(
            r#"
            SELECT cd.name, cd.version_req, cd.features, cd.optional, cd.default_features,
                   cd.target, cd.kind, cd.registry, cd.package
            FROM cargo_dependencies cd
            JOIN packages p ON cd.package_id = p.id
            JOIN cargo_crate_metadata cm ON p.id = cm.package_id
            JOIN projects proj ON p.project_id = proj.id
            JOIN namespaces ns ON proj.namespace_id = ns.id
            WHERE p.name = $1 AND p.version = $2 AND ns.path = $3
            "#
        )
        .bind(&crate_name)
        .bind(&version)
        .bind(&namespace)
        .fetch_all(pool.get_ref())
        .await?;

        let cargo_deps: Vec<CargoIndexDependency> = deps.into_iter().map(|(name, req, feat, opt, def, target, kind, registry, pkg)| {
            CargoIndexDependency {
                name,
                req,
                features: feat,
                optional: opt,
                default_features: def,
                target,
                kind,
                registry,
                package: pkg,
            }
        }).collect();

        entries.push(CargoIndexEntry {
            name: crate_name.clone(),
            vers: version,
            deps: cargo_deps,
            cksum: checksum,
            features,
            features2: None,
            yanked,
            links: if links.is_empty() { None } else { Some(links) },
            v: Some(2),
            rust_version: None,
        });
    }

    // Cargo 索引格式：每行一个 JSON 对象
    let mut body = String::new();
    for entry in &entries {
        body.push_str(&serde_json::to_string(entry).unwrap_or_default());
        body.push('\n');
    }

    Ok(HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(body))
}

/// 创建 Cargo 包请求
#[derive(Debug, serde::Deserialize)]
pub struct CreateCargoPackageRequest {
    pub project_id: i64,
    pub user_id: String,
    pub name: String,
    pub version: String,
    pub checksum: String,
    pub features: serde_json::Value,
    pub links: Option<String>,
    pub rust_version: Option<String>,
    pub readme: Option<String>,
    pub readme_file: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub description: Option<String>,
    pub dependencies: Vec<CreateCargoDependency>,
    pub file_path: String,
    pub file_size: i64,
}

/// 创建 Cargo 依赖
#[derive(Debug, serde::Deserialize)]
pub struct CreateCargoDependency {
    pub name: String,
    pub version_req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: String,
    pub registry: Option<String>,
    pub package: Option<String>,
}

/// POST /api/internal/registry/cargo/package
/// 创建 Cargo 包
pub async fn create_cargo_package(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<CreateCargoPackageRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    // 检查版本是否已存在
    let existing = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM packages WHERE project_id = $1 AND package_type = 'cargo' AND name = $2 AND version = $3"
    )
    .bind(body.project_id)
    .bind(&body.name)
    .bind(&body.version)
    .fetch_optional(pool.get_ref())
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(format!(
            "Crate {} version {} already exists",
            body.name, body.version
        )));
    }

    let user_uuid = Uuid::parse_str(&body.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id UUID".to_string()))?;

    let mut tx = pool.begin().await?;

    // 创建 package
    let package = sqlx::query_as::<_, Package>(
        r#"
        INSERT INTO packages (project_id, name, package_type, version, status, metadata, created_at, updated_at)
        VALUES ($1, $2, 'cargo', $3, 'default', '{}', NOW(), NOW())
        RETURNING id, project_id, name, package_type, version, status, metadata, created_at, updated_at
        "#
    )
    .bind(body.project_id)
    .bind(&body.name)
    .bind(&body.version)
    .fetch_one(&mut *tx)
    .await?;

    // 创建 package_file
    let file_name = format!("{}-{}.crate", body.name, body.version);
    sqlx::query(
        r#"
        INSERT INTO package_files (package_id, file_name, file_path, file_size, file_sha256, created_at)
        VALUES ($1, $2, $3, $4, $5, NOW())
        "#
    )
    .bind(package.id)
    .bind(&file_name)
    .bind(&body.file_path)
    .bind(body.file_size)
    .bind(&body.checksum)
    .execute(&mut *tx)
    .await?;

    // 创建 cargo_crate_metadata
    sqlx::query(
        r#"
        INSERT INTO cargo_crate_metadata 
            (package_id, checksum, yanked, features, links, rust_version,
             readme, readme_file, keywords, categories, license, license_file,
             repository, homepage, documentation, description, published_by, created_at)
        VALUES ($1, $2, false, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, NOW())
        "#
    )
    .bind(package.id)
    .bind(&body.checksum)
    .bind(&body.features)
    .bind(body.links.as_deref().unwrap_or(""))
    .bind(&body.rust_version)
    .bind(&body.readme)
    .bind(&body.readme_file)
    .bind(&body.keywords)
    .bind(&body.categories)
    .bind(&body.license)
    .bind(&body.license_file)
    .bind(&body.repository)
    .bind(&body.homepage)
    .bind(&body.documentation)
    .bind(&body.description)
    .bind(user_uuid)
    .execute(&mut *tx)
    .await?;

    // 创建依赖
    for dep in &body.dependencies {
        sqlx::query(
            r#"
            INSERT INTO cargo_dependencies 
                (package_id, name, version_req, features, optional, default_features, target, kind, registry, package)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(package.id)
        .bind(&dep.name)
        .bind(&dep.version_req)
        .bind(&dep.features)
        .bind(dep.optional)
        .bind(dep.default_features)
        .bind(&dep.target)
        .bind(&dep.kind)
        .bind(&dep.registry)
        .bind(&dep.package)
        .execute(&mut *tx)
        .await?;
    }

    // 添加发布者为 owner（如果是第一个版本）
    let existing_owner = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT co.id
        FROM cargo_crate_owners co
        JOIN packages p ON co.crate_name = p.name
        WHERE p.project_id = $1 AND p.name = $2
        LIMIT 1
        "#
    )
    .bind(body.project_id)
    .bind(&body.name)
    .fetch_optional(&mut *tx)
    .await?;

    if existing_owner.is_none() {
        sqlx::query(
            r#"
            INSERT INTO cargo_crate_owners (project_id, crate_name, user_id, added_by, created_at)
            VALUES ($1, $2, $3, $3, NOW())
            "#
        )
        .bind(body.project_id)
        .bind(&body.name)
        .bind(user_uuid)
        .execute(&mut *tx)
        .await?;
    }

    // 记录审计日志
    sqlx::query(
        r#"
        INSERT INTO cargo_audit_log (project_id, crate_name, version, action, user_id, details, created_at)
        VALUES ($1, $2, $3, 'publish', $4, $5, NOW())
        "#
    )
    .bind(body.project_id)
    .bind(&body.name)
    .bind(&body.version)
    .bind(user_uuid)
    .bind(serde_json::json!({ "checksum": body.checksum, "file_size": body.file_size }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!(
        "Created Cargo crate: {}@{} for project {}",
        body.name, body.version, body.project_id
    );

    Ok(HttpResponse::Created().json(package))
}

/// Cargo Token 验证请求
#[derive(Debug, serde::Deserialize)]
pub struct VerifyCargoTokenRequest {
    pub token: String,
    pub project_id: i64,
    pub required_scope: String,  // "publish", "yank", "owners"
}

/// Cargo Token 验证响应
#[derive(Debug, serde::Serialize)]
pub struct VerifyCargoTokenResponse {
    pub valid: bool,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub scopes: Vec<String>,
}

/// POST /api/internal/registry/cargo/verify-token
/// 验证 Cargo token
pub async fn verify_cargo_token(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    body: web::Json<VerifyCargoTokenRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    // 查找有效的 Cargo token
    let token_info = sqlx::query_as::<_, (uuid::Uuid, String, Vec<String>)>(
        r#"
        SELECT ct.user_id, u.username, ct.scopes
        FROM cargo_tokens ct
        JOIN users u ON ct.user_id = u.id
        WHERE ct.token_hash = $1
          AND ct.revoked_at IS NULL
          AND (ct.expires_at IS NULL OR ct.expires_at > NOW())
        "#
    )
    .bind(&body.token)  // 前端应该已经计算了哈希
    .fetch_optional(pool.get_ref())
    .await?;

    // 如果没找到，尝试作为 PAT 验证
    let token_info = match token_info {
        Some(info) => Some(info),
        None => {
            use crate::models::personal_access_token::PAT_PREFIX;
            
            // 检查是否是 PAT (格式: gfpat_xxx)
            if body.token.starts_with(PAT_PREFIX) {
                // PAT token 直接哈希（包含前缀）
                use sha2::{Sha256, Digest};
                let hashed = format!("{:x}", Sha256::digest(body.token.as_bytes()));
                
                let pat_info = sqlx::query_as::<_, (uuid::Uuid, String, Vec<String>)>(
                    r#"
                    SELECT pat.user_id, u.username, pat.scopes
                    FROM personal_access_tokens pat
                    JOIN users u ON pat.user_id = u.id
                    WHERE pat.token_hash = $1
                      AND pat.revoked_at IS NULL
                      AND (pat.expires_at IS NULL OR pat.expires_at > NOW())
                    "#
                )
                .bind(&hashed)
                .fetch_optional(pool.get_ref())
                .await?;

                // 检查 PAT 是否有 write_registry scope
                if let Some((user_id, username, scopes)) = pat_info {
                    if scopes.contains(&"write_registry".to_string()) || scopes.contains(&"api".to_string()) {
                        Some((user_id, username, vec!["publish".to_string(), "yank".to_string(), "owners".to_string()]))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    match token_info {
        Some((user_id, username, scopes)) => {
            // 检查是否有所需的 scope
            let has_scope = scopes.iter().any(|s| s == &body.required_scope || s == "*");
            
            // 检查用户是否有项目权限
            let has_project_access = sqlx::query_as::<_, (i64,)>(
                r#"
                SELECT 1
                FROM project_members pm
                WHERE pm.project_id = $1 AND pm.user_id = $2
                  AND pm.role IN ('owner', 'maintainer', 'developer')
                UNION
                SELECT 1
                FROM projects p
                JOIN namespaces ns ON p.namespace_id = ns.id
                WHERE p.id = $1 AND ns.owner_id = $2
                "#
            )
            .bind(body.project_id)
            .bind(user_id)
            .fetch_optional(pool.get_ref())
            .await?;

            if has_scope && has_project_access.is_some() {
                // 更新最后使用时间
                let _ = sqlx::query(
                    "UPDATE cargo_tokens SET last_used_at = NOW() WHERE token_hash = $1"
                )
                .bind(&body.token)
                .execute(pool.get_ref())
                .await;

                Ok(HttpResponse::Ok().json(VerifyCargoTokenResponse {
                    valid: true,
                    user_id: Some(user_id.to_string()),
                    username: Some(username),
                    scopes,
                }))
            } else {
                Ok(HttpResponse::Ok().json(VerifyCargoTokenResponse {
                    valid: false,
                    user_id: None,
                    username: None,
                    scopes: vec![],
                }))
            }
        }
        None => {
            Ok(HttpResponse::Ok().json(VerifyCargoTokenResponse {
                valid: false,
                user_id: None,
                username: None,
                scopes: vec![],
            }))
        }
    }
}

/// Yank/Unyank Cargo crate 请求
#[derive(Debug, serde::Deserialize)]
pub struct YankCargoRequest {
    pub user_id: String,
}

/// PUT /api/internal/registry/cargo/yank/{namespace}/{name}/{version}
/// Yank Cargo crate 版本
pub async fn yank_cargo_crate(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String, String)>,
    body: web::Json<YankCargoRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, name, version) = path.into_inner();
    let user_uuid = Uuid::parse_str(&body.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id UUID".to_string()))?;

    // 更新 yanked 状态
    let result = sqlx::query(
        r#"
        UPDATE cargo_crate_metadata cm
        SET yanked = true
        FROM packages p
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE cm.package_id = p.id
          AND p.package_type = 'cargo'
          AND p.name = $1
          AND p.version = $2
          AND ns.path = $3
        "#
    )
    .bind(&name)
    .bind(&version)
    .bind(&namespace)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Crate {}@{} not found", name, version)));
    }

    // 获取 project_id 用于审计日志
    let project_id: (i64,) = sqlx::query_as(
        r#"
        SELECT p.project_id
        FROM packages p
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE p.package_type = 'cargo' AND p.name = $1 AND ns.path = $2
        LIMIT 1
        "#
    )
    .bind(&name)
    .bind(&namespace)
    .fetch_one(pool.get_ref())
    .await?;

    // 记录审计日志
    sqlx::query(
        r#"
        INSERT INTO cargo_audit_log (project_id, crate_name, version, action, user_id, created_at)
        VALUES ($1, $2, $3, 'yank', $4, NOW())
        "#
    )
    .bind(project_id.0)
    .bind(&name)
    .bind(&version)
    .bind(user_uuid)
    .execute(pool.get_ref())
    .await?;

    info!("Yanked Cargo crate: {}@{} in namespace {}", name, version, namespace);

    Ok(HttpResponse::Ok().json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/internal/registry/cargo/yank/{namespace}/{name}/{version}
/// Unyank Cargo crate 版本
pub async fn unyank_cargo_crate(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String, String)>,
    body: web::Json<YankCargoRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, name, version) = path.into_inner();
    let user_uuid = Uuid::parse_str(&body.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id UUID".to_string()))?;

    // 更新 yanked 状态
    let result = sqlx::query(
        r#"
        UPDATE cargo_crate_metadata cm
        SET yanked = false
        FROM packages p
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE cm.package_id = p.id
          AND p.package_type = 'cargo'
          AND p.name = $1
          AND p.version = $2
          AND ns.path = $3
        "#
    )
    .bind(&name)
    .bind(&version)
    .bind(&namespace)
    .execute(pool.get_ref())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Crate {}@{} not found", name, version)));
    }

    // 获取 project_id 用于审计日志
    let project_id: (i64,) = sqlx::query_as(
        r#"
        SELECT p.project_id
        FROM packages p
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE p.package_type = 'cargo' AND p.name = $1 AND ns.path = $2
        LIMIT 1
        "#
    )
    .bind(&name)
    .bind(&namespace)
    .fetch_one(pool.get_ref())
    .await?;

    // 记录审计日志
    sqlx::query(
        r#"
        INSERT INTO cargo_audit_log (project_id, crate_name, version, action, user_id, created_at)
        VALUES ($1, $2, $3, 'unyank', $4, NOW())
        "#
    )
    .bind(project_id.0)
    .bind(&name)
    .bind(&version)
    .bind(user_uuid)
    .execute(pool.get_ref())
    .await?;

    info!("Unyanked Cargo crate: {}@{} in namespace {}", name, version, namespace);

    Ok(HttpResponse::Ok().json(serde_json::json!({ "ok": true })))
}

/// Cargo Owner 响应
#[derive(Debug, serde::Serialize)]
pub struct CargoOwner {
    pub id: String,
    pub login: String,
    pub name: Option<String>,
}

/// GET /api/internal/registry/cargo/owners/{namespace}/{name}
/// 获取 Cargo crate owners
pub async fn get_cargo_owners(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, name) = path.into_inner();

    let owners = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>)>(
        r#"
        SELECT u.id, u.username, u.full_name
        FROM cargo_crate_owners co
        JOIN users u ON co.user_id = u.id
        JOIN packages p ON co.project_id = p.project_id AND co.crate_name = p.name
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE p.package_type = 'cargo'
          AND p.name = $1
          AND ns.path = $2
        "#
    )
    .bind(&name)
    .bind(&namespace)
    .fetch_all(pool.get_ref())
    .await?;

    let owners: Vec<CargoOwner> = owners.into_iter().map(|(id, login, name)| {
        CargoOwner {
            id: id.to_string(),
            login,
            name,
        }
    }).collect();

    Ok(HttpResponse::Ok().json(serde_json::json!({ "users": owners })))
}

/// 修改 Cargo Owners 请求
#[derive(Debug, serde::Deserialize)]
pub struct ModifyCargoOwnersRequest {
    pub user_id: String,  // 操作者
    pub users: Vec<String>,  // 要添加/移除的用户名
}

/// PUT /api/internal/registry/cargo/owners/{namespace}/{name}
/// 添加 Cargo crate owners
pub async fn add_cargo_owners(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String)>,
    body: web::Json<ModifyCargoOwnersRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, name) = path.into_inner();
    let operator_uuid = Uuid::parse_str(&body.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id UUID".to_string()))?;

    // 获取 project_id
    let project_info = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT DISTINCT p.project_id
        FROM packages p
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE p.package_type = 'cargo' AND p.name = $1 AND ns.path = $2
        "#
    )
    .bind(&name)
    .bind(&namespace)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Crate {} not found", name)))?;

    let project_id = project_info.0;

    let mut added = Vec::new();
    for username in &body.users {
        // 查找用户
        let user = sqlx::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(pool.get_ref())
        .await?;

        if let Some((user_id,)) = user {
            // 添加 owner（忽略重复）
            let result = sqlx::query(
                r#"
                INSERT INTO cargo_crate_owners (project_id, crate_name, user_id, added_by, created_at)
                VALUES ($1, $2, $3, $4, NOW())
                ON CONFLICT (project_id, crate_name, user_id) DO NOTHING
                "#
            )
            .bind(project_id)
            .bind(&name)
            .bind(user_id)
            .bind(operator_uuid)
            .execute(pool.get_ref())
            .await?;

            if result.rows_affected() > 0 {
                added.push(username.clone());
                
                // 记录审计日志
                sqlx::query(
                    r#"
                    INSERT INTO cargo_audit_log (project_id, crate_name, version, action, user_id, details, created_at)
                    VALUES ($1, $2, NULL, 'add_owner', $3, $4, NOW())
                    "#
                )
                .bind(project_id)
                .bind(&name)
                .bind(operator_uuid)
                .bind(serde_json::json!({ "added_user": username }))
                .execute(pool.get_ref())
                .await?;
            }
        }
    }

    info!("Added Cargo crate owners for {}: {:?}", name, added);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "ok": true,
        "msg": format!("Added {} owner(s)", added.len())
    })))
}

/// DELETE /api/internal/registry/cargo/owners/{namespace}/{name}
/// 移除 Cargo crate owners
pub async fn remove_cargo_owners(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String)>,
    body: web::Json<ModifyCargoOwnersRequest>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, name) = path.into_inner();
    let operator_uuid = Uuid::parse_str(&body.user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id UUID".to_string()))?;

    // 获取 project_id
    let project_info = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT DISTINCT p.project_id
        FROM packages p
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE p.package_type = 'cargo' AND p.name = $1 AND ns.path = $2
        "#
    )
    .bind(&name)
    .bind(&namespace)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Crate {} not found", name)))?;

    let project_id = project_info.0;

    // 检查剩余 owner 数量
    let owner_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM cargo_crate_owners WHERE project_id = $1 AND crate_name = $2"
    )
    .bind(project_id)
    .bind(&name)
    .fetch_one(pool.get_ref())
    .await?;

    if owner_count.0 <= body.users.len() as i64 {
        return Err(AppError::BadRequest("Cannot remove all owners".to_string()));
    }

    let mut removed = Vec::new();
    for username in &body.users {
        // 查找用户
        let user = sqlx::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(pool.get_ref())
        .await?;

        if let Some((user_id,)) = user {
            let result = sqlx::query(
                "DELETE FROM cargo_crate_owners WHERE project_id = $1 AND crate_name = $2 AND user_id = $3"
            )
            .bind(project_id)
            .bind(&name)
            .bind(user_id)
            .execute(pool.get_ref())
            .await?;

            if result.rows_affected() > 0 {
                removed.push(username.clone());
                
                // 记录审计日志
                sqlx::query(
                    r#"
                    INSERT INTO cargo_audit_log (project_id, crate_name, version, action, user_id, details, created_at)
                    VALUES ($1, $2, NULL, 'remove_owner', $3, $4, NOW())
                    "#
                )
                .bind(project_id)
                .bind(&name)
                .bind(operator_uuid)
                .bind(serde_json::json!({ "removed_user": username }))
                .execute(pool.get_ref())
                .await?;
            }
        }
    }

    info!("Removed Cargo crate owners for {}: {:?}", name, removed);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "ok": true,
        "msg": format!("Removed {} owner(s)", removed.len())
    })))
}

/// POST /api/internal/registry/cargo/download/{namespace}/{name}/{version}
/// 记录 Cargo crate 下载
pub async fn record_cargo_download(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, name, version) = path.into_inner();

    // 获取 package_id
    let package = sqlx::query_as::<_, (i64,)>(
        r#"
        SELECT p.id
        FROM packages p
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE p.package_type = 'cargo'
          AND p.name = $1
          AND p.version = $2
          AND ns.path = $3
        "#
    )
    .bind(&name)
    .bind(&version)
    .bind(&namespace)
    .fetch_optional(pool.get_ref())
    .await?;

    if let Some((package_id,)) = package {
        // 更新下载统计（按日聚合）
        sqlx::query(
            r#"
            INSERT INTO cargo_download_stats (package_id, date, downloads)
            VALUES ($1, CURRENT_DATE, 1)
            ON CONFLICT (package_id, date) 
            DO UPDATE SET downloads = cargo_download_stats.downloads + 1
            "#
        )
        .bind(package_id)
        .execute(pool.get_ref())
        .await?;
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "ok": true })))
}

/// Cargo 搜索查询参数
#[derive(Debug, serde::Deserialize)]
pub struct CargoSearchQuery {
    pub q: Option<String>,
    pub per_page: Option<i64>,
}

/// Cargo 搜索结果项
#[derive(Debug, serde::Serialize)]
pub struct CargoCrateSummary {
    pub name: String,
    pub max_version: String,
    pub description: Option<String>,
}

/// GET /api/internal/registry/cargo/search/{namespace}
/// 搜索 Cargo crates
pub async fn search_cargo_crates(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<String>,
    query: web::Query<CargoSearchQuery>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let namespace = path.into_inner();
    let search_term = query.q.as_deref().unwrap_or("");
    let limit = query.per_page.unwrap_or(10).min(100);

    // 使用窗口函数获取每个 crate 的最新版本
    let crates = sqlx::query_as::<_, (String, String, Option<String>)>(
        r#"
        WITH latest_versions AS (
            SELECT DISTINCT ON (p.name) 
                p.name, p.version, cm.description
            FROM packages p
            JOIN projects proj ON p.project_id = proj.id
            JOIN namespaces ns ON proj.namespace_id = ns.id
            LEFT JOIN cargo_crate_metadata cm ON p.id = cm.package_id
            WHERE p.package_type = 'cargo'
              AND ns.path = $1
              AND p.status = 'default'
              AND ($2 = '' OR p.name ILIKE '%' || $2 || '%')
            ORDER BY p.name, p.created_at DESC
        )
        SELECT name, version, description
        FROM latest_versions
        LIMIT $3
        "#
    )
    .bind(&namespace)
    .bind(search_term)
    .bind(limit)
    .fetch_all(pool.get_ref())
    .await?;

    let results: Vec<CargoCrateSummary> = crates.into_iter().map(|(name, version, desc)| {
        CargoCrateSummary {
            name,
            max_version: version,
            description: desc,
        }
    }).collect();

    // Cargo API 格式
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "crates": results,
        "meta": {
            "total": results.len()
        }
    })))
}

/// Cargo crate 详情响应
#[derive(Debug, serde::Serialize)]
pub struct CargoCrateInfo {
    pub name: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub max_version: String,
    pub downloads: i64,
    pub versions: Vec<CargoCrateVersionInfo>,
}

/// Cargo crate 版本信息
#[derive(Debug, serde::Serialize)]
pub struct CargoCrateVersionInfo {
    pub num: String,
    pub yanked: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub downloads: i64,
}

/// GET /api/internal/registry/cargo/crate/{namespace}/{name}
/// 获取 Cargo crate 详情
pub async fn get_cargo_crate_info(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    config: web::Data<Config>,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    verify_internal_token(&req, &config)?;

    let (namespace, name) = path.into_inner();

    // 获取所有版本
    let versions = sqlx::query_as::<_, (i64, String, bool, chrono::DateTime<chrono::Utc>, Option<String>, Option<String>, Option<String>, Option<String>, Option<Vec<String>>, Option<Vec<String>>)>(
        r#"
        SELECT p.id, p.version, cm.yanked, p.created_at,
               cm.description, cm.license, cm.repository, cm.homepage,
               cm.keywords, cm.categories
        FROM packages p
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        JOIN cargo_crate_metadata cm ON p.id = cm.package_id
        WHERE p.package_type = 'cargo'
          AND p.name = $1
          AND ns.path = $2
          AND p.status = 'default'
        ORDER BY p.created_at DESC
        "#
    )
    .bind(&name)
    .bind(&namespace)
    .fetch_all(pool.get_ref())
    .await?;

    if versions.is_empty() {
        return Err(AppError::NotFound(format!("Crate {} not found", name)));
    }

    // 获取最新版本的元数据
    let (_, latest_version, _, _, description, license, repository, homepage, keywords, categories) = &versions[0];

    // 获取总下载量
    let total_downloads: (i64,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(cds.downloads), 0)::bigint
        FROM cargo_download_stats cds
        JOIN packages p ON cds.package_id = p.id
        JOIN projects proj ON p.project_id = proj.id
        JOIN namespaces ns ON proj.namespace_id = ns.id
        WHERE p.package_type = 'cargo' AND p.name = $1 AND ns.path = $2
        "#
    )
    .bind(&name)
    .bind(&namespace)
    .fetch_one(pool.get_ref())
    .await?;

    // 构建版本列表（带下载统计）
    let mut version_infos = Vec::new();
    for (pkg_id, version, yanked, created_at, _, _, _, _, _, _) in &versions {
        let downloads: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(downloads), 0)::bigint FROM cargo_download_stats WHERE package_id = $1"
        )
        .bind(pkg_id)
        .fetch_one(pool.get_ref())
        .await?;

        version_infos.push(CargoCrateVersionInfo {
            num: version.clone(),
            yanked: *yanked,
            created_at: *created_at,
            downloads: downloads.0,
        });
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "crate": CargoCrateInfo {
            name: name.clone(),
            description: description.clone(),
            license: license.clone(),
            repository: repository.clone(),
            homepage: homepage.clone(),
            documentation: None,
            keywords: keywords.clone(),
            categories: categories.clone(),
            max_version: latest_version.clone(),
            downloads: total_downloads.0,
            versions: version_infos,
        }
    })))
}

/// 配置路由
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/internal/registry")
            // Docker Registry
            .route("/docker/upload-session", web::post().to(create_upload_session))
            .route("/docker/upload-session/{uuid}", web::get().to(get_upload_session))
            .route("/docker/upload-session/{uuid}", web::patch().to(update_upload_session))
            .route("/docker/upload-session/{uuid}", web::delete().to(delete_upload_session))
            .route("/docker/blob", web::post().to(create_docker_blob))
            .route("/docker/blob/{project_id}/{digest}", web::get().to(get_docker_blob))
            .route("/docker/blob/{project_id}/{digest}", web::delete().to(delete_docker_blob))
            .route("/docker/manifest", web::post().to(create_docker_manifest))
            .route("/docker/manifest/{project_id}/{image_name}", web::get().to(get_docker_manifest))
            .route("/docker/manifest/{project_id}/{image_name}", web::delete().to(delete_docker_manifest))
            .route("/docker/tags/{project_id}/{image_name}", web::get().to(list_docker_tags))
            .route("/docker/catalog/{project_id}", web::get().to(list_docker_repositories))
            // npm Registry
            .route("/npm/package", web::post().to(create_npm_package))
            .route("/npm/package/{project_id}/{name}", web::get().to(get_npm_package))
            .route("/npm/lookup/{scope}/{name}", web::get().to(lookup_npm_package))
            .route("/npm/package-doc/{project_id}/{name}", web::get().to(get_npm_package_document))
            .route("/npm/package/{project_id}/{name}/{version}", web::delete().to(delete_npm_package))
            .route("/npm/dist-tag", web::put().to(update_npm_dist_tag))
            .route("/npm/dist-tag/{project_id}/{package_name}/{tag}", web::delete().to(delete_npm_dist_tag))
            .route("/npm/dist-tags/{project_id}/{package_name}", web::get().to(get_npm_dist_tags))
            .route("/npm/search", web::get().to(search_npm_packages))
            .route("/npm/login", web::post().to(npm_login))
            .route("/npm/whoami", web::get().to(npm_whoami))
            // Cargo Registry
            .route("/cargo/index/{namespace}/{crate_name}", web::get().to(get_cargo_index))
            .route("/cargo/package", web::post().to(create_cargo_package))
            .route("/cargo/verify-token", web::post().to(verify_cargo_token))
            .route("/cargo/yank/{namespace}/{name}/{version}", web::put().to(yank_cargo_crate))
            .route("/cargo/yank/{namespace}/{name}/{version}", web::delete().to(unyank_cargo_crate))
            .route("/cargo/owners/{namespace}/{name}", web::get().to(get_cargo_owners))
            .route("/cargo/owners/{namespace}/{name}", web::put().to(add_cargo_owners))
            .route("/cargo/owners/{namespace}/{name}", web::delete().to(remove_cargo_owners))
            .route("/cargo/download/{namespace}/{name}/{version}", web::post().to(record_cargo_download))
            .route("/cargo/search/{namespace}", web::get().to(search_cargo_crates))
            .route("/cargo/crate/{namespace}/{name}", web::get().to(get_cargo_crate_info))
            // 通用
            .route("/packages/{project_id}", web::get().to(list_project_packages))
            .route("/resolve-project/{namespace}/{project}", web::get().to(resolve_project))
    );
}
