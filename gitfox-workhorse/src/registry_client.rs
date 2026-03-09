//! Registry API 客户端
//!
//! 用于调用 Main App 的内部 Registry API

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error};

/// Registry API 客户端
#[derive(Clone)]
pub struct RegistryApiClient {
    client: Client,
    base_url: String,
    shell_token: String,
}

impl RegistryApiClient {
    /// 创建新的客户端
    pub fn new(base_url: &str, shell_token: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            shell_token: shell_token.to_string(),
        }
    }

    /// 发送带认证的 GET 请求
    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, RegistryApiError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Registry API GET: {}", url);

        let resp = self.client
            .get(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            resp.json::<T>()
                .await
                .map_err(|e| RegistryApiError::Parse(e.to_string()))
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(RegistryApiError::NotFound)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("Registry API error: {} - {}", status, text);
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    /// 发送带认证的 POST 请求
    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, RegistryApiError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Registry API POST: {}", url);

        let resp = self.client
            .post(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .json(body)
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            resp.json::<T>()
                .await
                .map_err(|e| RegistryApiError::Parse(e.to_string()))
        } else if resp.status() == reqwest::StatusCode::CONFLICT {
            Err(RegistryApiError::Conflict)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("Registry API error: {} - {}", status, text);
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    /// 发送带认证的 PATCH 请求
    async fn patch<B: Serialize>(&self, path: &str, body: &B) -> Result<(), RegistryApiError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Registry API PATCH: {}", url);

        let resp = self.client
            .patch(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .json(body)
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(RegistryApiError::NotFound)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("Registry API error: {} - {}", status, text);
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    /// 发送带认证的 DELETE 请求
    async fn delete(&self, path: &str) -> Result<(), RegistryApiError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Registry API DELETE: {}", url);

        let resp = self.client
            .delete(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(RegistryApiError::NotFound)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("Registry API error: {} - {}", status, text);
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    /// 发送带认证的 DELETE 请求并返回响应体
    async fn delete_with_response<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, RegistryApiError> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Registry API DELETE (with response): {}", url);

        let resp = self.client
            .delete(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            resp.json::<T>()
                .await
                .map_err(|e| RegistryApiError::Parse(e.to_string()))
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(RegistryApiError::NotFound)
        } else if resp.status() == reqwest::StatusCode::CONFLICT {
            Err(RegistryApiError::Conflict)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            error!("Registry API error: {} - {}", status, text);
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    // ========================================================================
    // Docker Registry API
    // ========================================================================

    /// 创建 Docker 上传会话
    pub async fn create_upload_session(
        &self,
        project_id: i64,
        user_id: &str,
        digest: Option<&str>,
        temp_path: &str,
    ) -> Result<CreateUploadSessionResponse, RegistryApiError> {
        self.post(
            "/api/internal/registry/docker/upload-session",
            &CreateUploadSessionRequest {
                project_id,
                user_id: user_id.to_string(),
                digest: digest.map(String::from),
                temp_path: temp_path.to_string(),
            },
        )
        .await
    }

    /// 获取 Docker 上传会话
    pub async fn get_upload_session(&self, uuid: &str) -> Result<UploadSession, RegistryApiError> {
        self.get(&format!("/api/internal/registry/docker/upload-session/{}", uuid))
            .await
    }

    /// 更新上传会话进度
    pub async fn update_upload_progress(
        &self,
        uuid: &str,
        uploaded_bytes: i64,
    ) -> Result<(), RegistryApiError> {
        self.patch(
            &format!("/api/internal/registry/docker/upload-session/{}", uuid),
            &UpdateUploadProgressRequest { uploaded_bytes },
        )
        .await
    }

    /// 删除上传会话
    pub async fn delete_upload_session(&self, uuid: &str) -> Result<(), RegistryApiError> {
        self.delete(&format!("/api/internal/registry/docker/upload-session/{}", uuid))
            .await
    }

    /// 创建 Docker Blob 记录
    pub async fn create_docker_blob(
        &self,
        project_id: i64,
        digest: &str,
        media_type: Option<&str>,
        size: i64,
        file_path: &str,
    ) -> Result<DockerBlob, RegistryApiError> {
        self.post(
            "/api/internal/registry/docker/blob",
            &CreateDockerBlobRequest {
                project_id,
                digest: digest.to_string(),
                media_type: media_type.map(String::from),
                size,
                file_path: file_path.to_string(),
            },
        )
        .await
    }

    /// 获取 Docker Blob 记录
    pub async fn get_docker_blob(
        &self,
        project_id: i64,
        digest: &str,
    ) -> Result<DockerBlob, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/docker/blob/{}/{}",
            project_id, digest
        ))
        .await
    }

    /// 删除 Docker Blob 记录
    /// 
    /// 删除前会检查引用计数。如果 blob 仍被 manifest 引用，返回 Conflict 错误。
    /// 成功删除后返回 blob 信息（包含 file_path），调用方需负责删除物理文件。
    pub async fn delete_docker_blob(
        &self,
        project_id: i64,
        digest: &str,
    ) -> Result<DockerBlob, RegistryApiError> {
        self.delete_with_response(&format!(
            "/api/internal/registry/docker/blob/{}/{}",
            project_id, digest
        ))
        .await
    }

    /// 创建 Docker Manifest
    pub async fn create_docker_manifest(
        &self,
        request: &CreateDockerManifestRequest,
    ) -> Result<DockerManifest, RegistryApiError> {
        self.post("/api/internal/registry/docker/manifest", request)
            .await
    }

    /// 获取 Docker Manifest（按 tag）
    pub async fn get_docker_manifest_by_tag(
        &self,
        project_id: i64,
        image_name: &str,
        tag: &str,
    ) -> Result<DockerManifest, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/docker/manifest/{}/{}?tag={}",
            project_id, image_name, tag
        ))
        .await
    }

    /// 获取 Docker Manifest（按 digest）
    pub async fn get_docker_manifest_by_digest(
        &self,
        project_id: i64,
        image_name: &str,
        digest: &str,
    ) -> Result<DockerManifest, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/docker/manifest/{}/{}?digest={}",
            project_id,
            image_name,
            urlencoding::encode(digest)
        ))
        .await
    }

    /// 删除 Docker Manifest
    pub async fn delete_docker_manifest(
        &self,
        project_id: i64,
        image_name: &str,
        reference: &str,
    ) -> Result<(), RegistryApiError> {
        // 判断 reference 是 tag 还是 digest
        let query = if reference.starts_with("sha256:") {
            format!("digest={}", urlencoding::encode(reference))
        } else {
            format!("tag={}", reference)
        };
        self.delete(&format!(
            "/api/internal/registry/docker/manifest/{}/{}?{}",
            project_id, image_name, query
        ))
        .await
    }

    /// 列出 Docker 标签
    pub async fn list_docker_tags(
        &self,
        project_id: i64,
        image_name: &str,
    ) -> Result<DockerTagListResponse, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/docker/tags/{}/{}",
            project_id, image_name
        ))
        .await
    }

    /// 列出 Docker 仓库
    pub async fn list_docker_repositories(
        &self,
        project_id: i64,
    ) -> Result<DockerCatalogResponse, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/docker/catalog/{}",
            project_id
        ))
        .await
    }

    // ========================================================================
    // npm Registry API
    // ========================================================================

    /// 创建 npm 包
    pub async fn create_npm_package(
        &self,
        request: &CreateNpmPackageRequest,
    ) -> Result<NpmPackage, RegistryApiError> {
        self.post("/api/internal/registry/npm/package", request)
            .await
    }

    /// 按 scope 和 name 查找 npm 包
    /// 
    /// 在指定 scope (namespace) 下查找包，无需知道具体 project_id。
    /// 返回包所属的 project_id 和完整的包文档信息。
    pub async fn lookup_npm_package(
        &self,
        scope: &str,
        name: &str,
    ) -> Result<NpmPackageLookupResponse, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/npm/lookup/{}/{}",
            urlencoding::encode(scope),
            urlencoding::encode(name)
        ))
        .await
    }

    /// 获取 npm 包信息
    pub async fn get_npm_package(
        &self,
        project_id: i64,
        name: &str,
        version: Option<&str>,
        tag: Option<&str>,
    ) -> Result<NpmPackageInfoResponse, RegistryApiError> {
        let mut query = Vec::new();
        if let Some(v) = version {
            query.push(format!("version={}", urlencoding::encode(v)));
        }
        if let Some(t) = tag {
            query.push(format!("tag={}", t));
        }
        let query_str = if query.is_empty() {
            String::new()
        } else {
            format!("?{}", query.join("&"))
        };
        self.get(&format!(
            "/api/internal/registry/npm/package/{}/{}{}",
            project_id,
            urlencoding::encode(name),
            query_str
        ))
        .await
    }

    /// 获取 npm 包完整文档
    pub async fn get_npm_package_document(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<NpmPackageDocumentResponse, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/npm/package-doc/{}/{}",
            project_id,
            urlencoding::encode(name)
        ))
        .await
    }

    /// 删除 npm 包版本
    pub async fn delete_npm_package(
        &self,
        project_id: i64,
        name: &str,
        version: &str,
    ) -> Result<(), RegistryApiError> {
        self.delete(&format!(
            "/api/internal/registry/npm/package/{}/{}/{}",
            project_id,
            urlencoding::encode(name),
            urlencoding::encode(version)
        ))
        .await
    }

    /// 更新 npm dist-tag
    pub async fn update_npm_dist_tag(
        &self,
        project_id: i64,
        package_name: &str,
        tag: &str,
        version: &str,
    ) -> Result<(), RegistryApiError> {
        let url = format!("{}/api/internal/registry/npm/dist-tag", self.base_url);
        let resp = self.client
            .put(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .json(&UpdateNpmDistTagRequest {
                project_id,
                package_name: package_name.to_string(),
                tag: tag.to_string(),
                version: version.to_string(),
            })
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    /// 删除 npm dist-tag
    pub async fn delete_npm_dist_tag(
        &self,
        project_id: i64,
        package_name: &str,
        tag: &str,
    ) -> Result<(), RegistryApiError> {
        self.delete(&format!(
            "/api/internal/registry/npm/dist-tag/{}/{}/{}",
            project_id,
            urlencoding::encode(package_name),
            tag
        ))
        .await
    }

    /// 获取 npm 包的所有 dist-tags
    pub async fn get_npm_dist_tags(
        &self,
        project_id: i64,
        package_name: &str,
    ) -> Result<NpmDistTagsResponse, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/npm/dist-tags/{}/{}",
            project_id,
            urlencoding::encode(package_name)
        ))
        .await
    }

    /// 搜索 npm 包
    pub async fn search_npm_packages(
        &self,
        query: &str,
        limit: i32,
        offset: i32,
    ) -> Result<NpmSearchResponse, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/npm/search?q={}&limit={}&offset={}",
            urlencoding::encode(query),
            limit,
            offset
        ))
        .await
    }

    /// 验证 npm 用户凭据并生成 token
    pub async fn npm_login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<NpmLoginResponse, RegistryApiError> {
        self.post(
            "/api/internal/registry/npm/login",
            &NpmLoginRequest {
                username: username.to_string(),
                password: password.to_string(),
            },
        )
        .await
    }

    /// 验证 npm token 获取用户信息
    pub async fn npm_whoami(&self, token: &str) -> Result<NpmWhoamiResponse, RegistryApiError> {
        let url = format!("{}/api/internal/registry/npm/whoami", self.base_url);
        let resp = self.client
            .get(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            resp.json()
                .await
                .map_err(|e| RegistryApiError::Parse(e.to_string()))
        } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            Err(RegistryApiError::NotFound)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    // ========================================================================
    // 通用 API
    // ========================================================================

    /// 解析项目路径获取 project_id
    /// 解析 namespace 到默认 registry 项目
    pub async fn resolve_namespace_default_project(
        &self,
        namespace: &str,
    ) -> Result<ResolveProjectResponse, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/resolve-namespace/{}",
            namespace
        ))
        .await
    }

    pub async fn resolve_project(
        &self,
        namespace: &str,
        project: &str,
    ) -> Result<ResolveProjectResponse, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/resolve-project/{}/{}",
            namespace, project
        ))
        .await
    }

    // ========================================================================
    // Cargo Registry API
    // ========================================================================

    /// 获取 Cargo crate 索引条目
    pub async fn get_cargo_index(
        &self,
        namespace: &str,
        crate_name: &str,
    ) -> Result<Vec<CargoIndexEntry>, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/cargo/index/{}/{}",
            urlencoding::encode(namespace),
            urlencoding::encode(crate_name)
        ))
        .await
    }

    /// 创建 Cargo 包
    pub async fn create_cargo_package(
        &self,
        request: &CreateCargoPackageRequest,
    ) -> Result<CargoPackage, RegistryApiError> {
        self.post("/api/internal/registry/cargo/package", request)
            .await
    }

    /// 验证 Cargo token 并获取用户信息
    pub async fn verify_cargo_token(
        &self,
        token: &str,
    ) -> Result<CargoTokenInfo, RegistryApiError> {
        let url = format!("{}/api/internal/registry/cargo/verify-token", self.base_url);
        let resp = self.client
            .post(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .json(&CargoVerifyTokenRequest { token: token.to_string() })
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            resp.json()
                .await
                .map_err(|e| RegistryApiError::Parse(e.to_string()))
        } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            Err(RegistryApiError::Unauthorized)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    /// Yank 或 Unyank Cargo crate
    pub async fn yank_cargo_crate(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
        token: &str,
        yank: bool,
    ) -> Result<(), RegistryApiError> {
        let url = format!(
            "{}/api/internal/registry/cargo/yank/{}/{}/{}",
            self.base_url,
            urlencoding::encode(namespace),
            urlencoding::encode(name),
            urlencoding::encode(version)
        );
        let resp = self.client
            .post(&url)
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .json(&CargoYankRequest { token: token.to_string(), yank })
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(RegistryApiError::NotFound)
        } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED || resp.status() == reqwest::StatusCode::FORBIDDEN {
            Err(RegistryApiError::Unauthorized)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    /// 获取 Cargo crate 所有者
    pub async fn get_cargo_owners(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Vec<CargoOwner>, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/cargo/owners/{}/{}",
            urlencoding::encode(namespace),
            urlencoding::encode(name)
        ))
        .await
    }

    /// 添加或移除 Cargo crate 所有者
    pub async fn modify_cargo_owners(
        &self,
        namespace: &str,
        name: &str,
        users: &[String],
        token: &str,
        add: bool,
    ) -> Result<(), RegistryApiError> {
        let url = format!(
            "{}/api/internal/registry/cargo/owners/{}/{}",
            self.base_url,
            urlencoding::encode(namespace),
            urlencoding::encode(name)
        );
        let method = if add { "PUT" } else { "DELETE" };
        let builder = if add {
            self.client.put(&url)
        } else {
            self.client.delete(&url)
        };
        
        let resp = builder
            .header("X-GitFox-Shell-Token", &self.shell_token)
            .json(&CargoModifyOwnersRequest {
                token: token.to_string(),
                users: users.to_vec(),
            })
            .send()
            .await
            .map_err(|e| RegistryApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(RegistryApiError::NotFound)
        } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED || resp.status() == reqwest::StatusCode::FORBIDDEN {
            Err(RegistryApiError::Unauthorized)
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(RegistryApiError::Api(format!("{}: {}", status, text)))
        }
    }

    /// 记录 Cargo 下载
    pub async fn record_cargo_download(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
    ) -> Result<(), RegistryApiError> {
        self.post(
            &format!(
                "/api/internal/registry/cargo/download/{}/{}/{}",
                urlencoding::encode(namespace),
                urlencoding::encode(name),
                urlencoding::encode(version)
            ),
            &(),
        )
        .await
        .map(|_: serde_json::Value| ())
    }

    /// 搜索 Cargo crates
    pub async fn search_cargo_crates(
        &self,
        namespace: &str,
        query: &str,
        per_page: i32,
    ) -> Result<CargoSearchResult, RegistryApiError> {
        self.get(&format!(
            "/api/internal/registry/cargo/search/{}?q={}&per_page={}",
            urlencoding::encode(namespace),
            urlencoding::encode(query),
            per_page
        ))
        .await
    }

    /// 获取 Cargo crate 信息
    pub async fn get_cargo_crate_info(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<CargoCrateInfo, RegistryApiError> {
        let response: CargoCrateInfoResponse = self.get(&format!(
            "/api/internal/registry/cargo/crate/{}/{}",
            urlencoding::encode(namespace),
            urlencoding::encode(name)
        ))
        .await?;
        Ok(response.crate_info)
    }
}

/// Registry API 错误类型
#[derive(Debug)]
pub enum RegistryApiError {
    Network(String),
    Api(String),
    Parse(String),
    NotFound,
    Conflict,
    Unauthorized,
}

impl std::fmt::Display for RegistryApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "Network error: {}", e),
            Self::Api(e) => write!(f, "API error: {}", e),
            Self::Parse(e) => write!(f, "Parse error: {}", e),
            Self::NotFound => write!(f, "Not found"),
            Self::Conflict => write!(f, "Conflict"),
            Self::Unauthorized => write!(f, "Unauthorized"),
        }
    }
}

impl std::error::Error for RegistryApiError {}

// ============================================================================
// 请求/响应类型
// ============================================================================

#[derive(Debug, Serialize)]
pub struct CreateUploadSessionRequest {
    pub project_id: i64,
    pub user_id: String,
    pub digest: Option<String>,
    pub temp_path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUploadSessionResponse {
    pub uuid: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct UpdateUploadProgressRequest {
    pub uploaded_bytes: i64,
}

#[derive(Debug, Deserialize)]
pub struct UploadSession {
    pub id: i64,
    pub uuid: String,
    pub project_id: i64,
    pub user_id: String,
    pub digest: Option<String>,
    pub uploaded_bytes: i64,
    pub temp_path: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreateDockerBlobRequest {
    pub project_id: i64,
    pub digest: String,
    pub media_type: Option<String>,
    pub size: i64,
    pub file_path: String,
}

#[derive(Debug, Deserialize)]
pub struct DockerBlob {
    pub id: i64,
    pub project_id: i64,
    pub digest: String,
    pub media_type: Option<String>,
    pub size: i64,
    pub file_path: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreateDockerManifestRequest {
    pub project_id: i64,
    pub image_name: String,
    pub tag: String,
    pub digest: String,
    pub media_type: String,
    pub schema_version: i32,
    pub config_digest: Option<String>,
    pub total_size: i64,
    pub manifest_json: serde_json::Value,
    pub blob_digests: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DockerManifest {
    pub id: i64,
    pub package_id: i64,
    pub digest: String,
    pub media_type: String,
    pub schema_version: i32,
    pub config_digest: Option<String>,
    pub total_size: i64,
    pub manifest_json: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct DockerTagListResponse {
    pub name: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DockerCatalogResponse {
    pub repositories: Vec<String>,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
pub struct NpmPackage {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NpmPackageInfoResponse {
    pub package: NpmPackage,
    pub metadata: Option<NpmPackageMetadata>,
    pub file: Option<PackageFile>,
}

#[derive(Debug, Deserialize)]
pub struct NpmPackageMetadata {
    pub id: i64,
    pub package_id: i64,
    pub dist_tag: String,
    pub tarball_sha512: Option<String>,
    pub npm_readme: Option<String>,
    pub npm_keywords: Option<Vec<String>>,
    pub npm_license: Option<String>,
    pub npm_repository: Option<serde_json::Value>,
    pub npm_dependencies: Option<serde_json::Value>,
    pub npm_dev_dependencies: Option<serde_json::Value>,
    pub npm_peer_dependencies: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PackageFile {
    pub id: i64,
    pub package_id: i64,
    pub file_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub file_sha256: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NpmPackageDocumentResponse {
    pub name: String,
    pub dist_tags: std::collections::HashMap<String, String>,
    pub versions: Vec<NpmVersionSummary>,
}

/// npm 包查找响应
/// 
/// 包含包所属的 project_id，用于按 scope+name 查找包（无需预先知道 project_id）
#[derive(Debug, Deserialize)]
pub struct NpmPackageLookupResponse {
    pub project_id: i64,
    pub name: String,
    pub dist_tags: std::collections::HashMap<String, String>,
    pub versions: Vec<NpmVersionSummary>,
}

#[derive(Debug, Deserialize)]
pub struct NpmVersionSummary {
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub tarball_url: String,
    pub integrity: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateNpmDistTagRequest {
    pub project_id: i64,
    pub package_name: String,
    pub tag: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveProjectResponse {
    pub project_id: i64,
    pub project_name: String,
    pub namespace: String,
}

#[derive(Debug, Deserialize)]
pub struct NpmDistTagsResponse {
    pub tags: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct NpmSearchResponse {
    pub packages: Vec<NpmSearchResult>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct NpmSearchResult {
    pub name: String,
    pub scope: Option<String>,
    pub version: String,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub date: String,
    pub links: NpmSearchResultLinks,
    pub publisher: Option<NpmSearchPublisher>,
}

#[derive(Debug, Deserialize)]
pub struct NpmSearchResultLinks {
    pub npm: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub bugs: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NpmSearchPublisher {
    pub username: String,
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NpmLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct NpmLoginResponse {
    pub ok: bool,
    pub token: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct NpmWhoamiResponse {
    pub username: String,
}

// ============================================================================
// Cargo Registry 类型
// ============================================================================

/// Cargo 索引条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoIndexEntry {
    pub name: String,
    pub vers: String,
    pub deps: Vec<CargoIndexDependency>,
    pub cksum: String,
    pub features: std::collections::HashMap<String, Vec<String>>,
    #[serde(default)]
    pub yanked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features2: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<i32>,
}

/// Cargo 依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoIndexDependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
}

/// 创建 Cargo 包请求
#[derive(Debug, Serialize)]
pub struct CreateCargoPackageRequest {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub user_id: i64,
    pub deps: Vec<CargoIndexDependency>,
    pub features: std::collections::HashMap<String, Vec<String>>,
    pub cksum: String,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub readme: Option<String>,
    pub readme_file: Option<String>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub authors: Vec<String>,
    pub links: Option<String>,
    pub rust_version: Option<String>,
    pub file_path: String,
    pub file_size: i64,
}

/// Cargo 包响应（对应 Main App 的 Package 结构体）
#[derive(Debug, Deserialize)]
pub struct CargoPackage {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub package_type: String,
    pub version: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 验证 Cargo token 请求
#[derive(Debug, Serialize)]
pub struct CargoVerifyTokenRequest {
    pub token: String,
}

/// Cargo token 信息
#[derive(Debug, Deserialize)]
pub struct CargoTokenInfo {
    pub user_id: i64,
    pub username: String,
    pub scopes: Vec<String>,
}

/// Cargo yank 请求
#[derive(Debug, Serialize)]
pub struct CargoYankRequest {
    pub token: String,
    pub yank: bool,
}

/// Cargo 所有者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoOwner {
    pub id: i64,
    pub login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 修改 Cargo 所有者请求
#[derive(Debug, Serialize)]
pub struct CargoModifyOwnersRequest {
    pub token: String,
    pub users: Vec<String>,
}

/// Cargo 搜索结果
#[derive(Debug, Serialize, Deserialize)]
pub struct CargoSearchResult {
    pub crates: Vec<CargoCrateSummary>,
    pub total: i64,
}

/// Cargo crate 摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoCrateSummary {
    pub name: String,
    pub max_version: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    pub downloads: i64,
    pub recent_downloads: i64,
}

/// Cargo crate 详细信息响应包装
#[derive(Debug, Serialize, Deserialize)]
pub struct CargoCrateInfoResponse {
    #[serde(rename = "crate")]
    pub crate_info: CargoCrateInfo,
}

/// Cargo crate 详细信息
#[derive(Debug, Serialize, Deserialize)]
pub struct CargoCrateInfo {
    pub name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    #[serde(default)]
    pub keywords: Option<Vec<String>>,
    #[serde(default)]
    pub categories: Option<Vec<String>>,
    pub max_version: String,
    pub versions: Vec<CargoCrateVersion>,
    pub downloads: i64,
}

/// Cargo crate 版本信息
#[derive(Debug, Serialize, Deserialize)]
pub struct CargoCrateVersion {
    #[serde(alias = "num")]
    pub version: String,
    pub yanked: bool,
    pub downloads: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub rust_version: Option<String>,
}
