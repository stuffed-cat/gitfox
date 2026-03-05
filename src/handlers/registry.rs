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
            // 通用
            .route("/packages/{project_id}", web::get().to(list_project_packages))
            .route("/resolve-project/{namespace}/{project}", web::get().to(resolve_project))
    );
}
