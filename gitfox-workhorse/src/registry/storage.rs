//! Registry 存储层
//!
//! 负责包文件的本地存储，使用内容寻址存储。

use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use sha2::{Sha256, Digest};
use tracing::{debug, info, error};

/// Registry 存储
#[derive(Clone)]
pub struct RegistryStorage {
    /// 存储根目录
    base_path: PathBuf,
    /// 临时文件目录
    tmp_path: PathBuf,
}

impl RegistryStorage {
    /// 创建新的存储实例
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        let base = base_path.as_ref().to_path_buf();
        let tmp = base.join("tmp");
        Self {
            base_path: base,
            tmp_path: tmp,
        }
    }

    /// 初始化存储目录
    pub async fn init(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.base_path).await?;
        fs::create_dir_all(&self.tmp_path).await?;
        // 创建 docker、npm 和 cargo 子目录
        fs::create_dir_all(self.base_path.join("docker")).await?;
        fs::create_dir_all(self.base_path.join("npm")).await?;
        fs::create_dir_all(self.base_path.join("cargo")).await?;
        info!("Registry storage initialized at {:?}", self.base_path);
        Ok(())
    }

    // ========================================================================
    // Docker 存储方法
    // ========================================================================

    /// 获取 Docker blob 存储路径
    /// 格式: docker/blobs/{digest[7:9]}/{digest[9:11]}/{digest}
    pub fn docker_blob_path(&self, digest: &str) -> PathBuf {
        // digest 格式: sha256:xxx...
        let hash = digest.trim_start_matches("sha256:");
        if hash.len() < 11 {
            return self.base_path.join("docker").join("blobs").join(hash);
        }
        self.base_path
            .join("docker")
            .join("blobs")
            .join(&hash[0..2])
            .join(&hash[2..4])
            .join(digest.replace(":", "_"))
    }

    /// 检查 Docker blob 是否存在
    pub async fn docker_blob_exists(&self, digest: &str) -> bool {
        let path = self.docker_blob_path(digest);
        path.exists()
    }

    /// 获取 Docker blob 大小
    pub async fn docker_blob_size(&self, digest: &str) -> Option<i64> {
        let path = self.docker_blob_path(digest);
        fs::metadata(&path).await.ok().map(|m| m.len() as i64)
    }

    /// 打开 Docker blob 文件
    pub async fn open_docker_blob(&self, digest: &str) -> std::io::Result<File> {
        let path = self.docker_blob_path(digest);
        File::open(&path).await
    }

    /// 存储 Docker blob
    pub async fn store_docker_blob(
        &self,
        digest: &str,
        data: &[u8],
    ) -> Result<PathBuf, StorageError> {
        // 验证 digest
        let expected_hash = digest.trim_start_matches("sha256:");
        let actual_hash = hex::encode(Sha256::digest(data));
        
        if expected_hash != actual_hash {
            return Err(StorageError::DigestMismatch {
                expected: digest.to_string(),
                actual: format!("sha256:{}", actual_hash),
            });
        }

        let path = self.docker_blob_path(digest);
        
        // 如果已存在，直接返回
        if path.exists() {
            debug!("Docker blob {} already exists", digest);
            return Ok(path);
        }

        // 创建目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::IoError(format!("Failed to create blob directory: {}", e))
            })?;
        }

        // 写入文件
        let mut file = File::create(&path).await.map_err(|e| {
            StorageError::IoError(format!("Failed to create blob file: {}", e))
        })?;
        file.write_all(data).await.map_err(|e| {
            StorageError::IoError(format!("Failed to write blob data: {}", e))
        })?;

        info!("Stored Docker blob: {}", digest);
        Ok(path)
    }

    /// 从临时文件完成 Docker blob 上传
    pub async fn finalize_docker_blob(
        &self,
        temp_path: &Path,
        expected_digest: &str,
    ) -> Result<PathBuf, StorageError> {
        // 计算实际 digest
        let actual_digest = self.calculate_sha256(temp_path).await?;
        let expected_hash = expected_digest.trim_start_matches("sha256:");
        
        if expected_hash != actual_digest {
            // 清理临时文件
            let _ = fs::remove_file(temp_path).await;
            return Err(StorageError::DigestMismatch {
                expected: expected_digest.to_string(),
                actual: format!("sha256:{}", actual_digest),
            });
        }

        let final_path = self.docker_blob_path(expected_digest);
        
        // 如果已存在，删除临时文件
        if final_path.exists() {
            let _ = fs::remove_file(temp_path).await;
            return Ok(final_path);
        }

        // 创建目标目录
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::IoError(format!("Failed to create blob directory: {}", e))
            })?;
        }

        // 移动文件
        fs::rename(temp_path, &final_path).await.map_err(|e| {
            StorageError::IoError(format!("Failed to move temp file: {}", e))
        })?;

        info!("Finalized Docker blob: {}", expected_digest);
        Ok(final_path)
    }

    /// 删除 Docker blob
    pub async fn delete_docker_blob(&self, digest: &str) -> Result<(), StorageError> {
        let path = self.docker_blob_path(digest);
        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                StorageError::IoError(format!("Failed to delete blob: {}", e))
            })?;
            info!("Deleted Docker blob: {}", digest);
        }
        Ok(())
    }

    // ========================================================================
    // npm 存储方法
    // ========================================================================

    /// 获取 npm tarball 存储路径
    /// 格式: npm/{scope}/{name}/{version}/{filename}
    pub fn npm_tarball_path(&self, scope: Option<&str>, name: &str, version: &str, filename: &str) -> PathBuf {
        let mut path = self.base_path.join("npm");
        if let Some(s) = scope {
            path = path.join(s.trim_start_matches('@'));
        }
        path.join(name).join(version).join(filename)
    }

    /// 检查 npm tarball 是否存在
    pub async fn npm_tarball_exists(&self, scope: Option<&str>, name: &str, version: &str, filename: &str) -> bool {
        let path = self.npm_tarball_path(scope, name, version, filename);
        path.exists()
    }

    /// 获取 npm tarball 大小
    pub async fn npm_tarball_size(&self, scope: Option<&str>, name: &str, version: &str, filename: &str) -> Option<i64> {
        let path = self.npm_tarball_path(scope, name, version, filename);
        fs::metadata(&path).await.ok().map(|m| m.len() as i64)
    }

    /// 打开 npm tarball 文件
    pub async fn open_npm_tarball(&self, scope: Option<&str>, name: &str, version: &str, filename: &str) -> std::io::Result<File> {
        let path = self.npm_tarball_path(scope, name, version, filename);
        File::open(&path).await
    }

    /// 存储 npm tarball
    pub async fn store_npm_tarball(
        &self,
        scope: Option<&str>,
        name: &str,
        version: &str,
        filename: &str,
        data: &[u8],
    ) -> Result<(PathBuf, String), StorageError> {
        let path = self.npm_tarball_path(scope, name, version, filename);

        // 创建目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::IoError(format!("Failed to create tarball directory: {}", e))
            })?;
        }

        // 计算 SHA-512 (npm 使用 sha512)
        use sha2::Sha512;
        let sha512 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            Sha512::digest(data),
        );
        let integrity = format!("sha512-{}", sha512);

        // 写入文件
        let mut file = File::create(&path).await.map_err(|e| {
            StorageError::IoError(format!("Failed to create tarball file: {}", e))
        })?;
        file.write_all(data).await.map_err(|e| {
            StorageError::IoError(format!("Failed to write tarball data: {}", e))
        })?;

        info!("Stored npm tarball: {}", path.display());
        Ok((path, integrity))
    }

    /// 删除 npm tarball
    pub async fn delete_npm_tarball(
        &self,
        scope: Option<&str>,
        name: &str,
        version: &str,
        filename: &str,
    ) -> Result<(), StorageError> {
        let path = self.npm_tarball_path(scope, name, version, filename);
        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                StorageError::IoError(format!("Failed to delete tarball: {}", e))
            })?;
            info!("Deleted npm tarball: {}", path.display());
        }
        Ok(())
    }

    // ========================================================================
    // Cargo 存储方法
    // ========================================================================

    /// 获取 Cargo crate 存储路径
    /// 格式: cargo/{namespace}/{crate_name}/{version}/{crate_name}-{version}.crate
    pub fn cargo_crate_path(&self, namespace: &str, crate_name: &str, version: &str) -> PathBuf {
        self.base_path
            .join("cargo")
            .join(namespace)
            .join(crate_name)
            .join(version)
            .join(format!("{}-{}.crate", crate_name, version))
    }

    /// 检查 Cargo crate 是否存在
    pub async fn cargo_crate_exists(&self, namespace: &str, crate_name: &str, version: &str) -> bool {
        let path = self.cargo_crate_path(namespace, crate_name, version);
        path.exists()
    }

    /// 获取 Cargo crate 大小
    pub async fn cargo_crate_size(&self, namespace: &str, crate_name: &str, version: &str) -> Option<i64> {
        let path = self.cargo_crate_path(namespace, crate_name, version);
        fs::metadata(&path).await.ok().map(|m| m.len() as i64)
    }

    /// 打开 Cargo crate 文件
    pub async fn open_cargo_crate(&self, namespace: &str, crate_name: &str, version: &str) -> std::io::Result<File> {
        let path = self.cargo_crate_path(namespace, crate_name, version);
        File::open(&path).await
    }

    /// 存储 Cargo crate
    /// 返回 (文件路径, sha256 校验和)
    pub async fn store_cargo_crate(
        &self,
        namespace: &str,
        crate_name: &str,
        version: &str,
        data: &[u8],
    ) -> Result<(PathBuf, String), StorageError> {
        let path = self.cargo_crate_path(namespace, crate_name, version);

        // 创建目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                StorageError::IoError(format!("Failed to create crate directory: {}", e))
            })?;
        }

        // 计算 SHA-256 校验和（Cargo 使用 sha256）
        let sha256 = hex::encode(Sha256::digest(data));

        // 写入文件
        let mut file = File::create(&path).await.map_err(|e| {
            StorageError::IoError(format!("Failed to create crate file: {}", e))
        })?;
        file.write_all(data).await.map_err(|e| {
            StorageError::IoError(format!("Failed to write crate data: {}", e))
        })?;

        info!("Stored Cargo crate: {}", path.display());
        Ok((path, sha256))
    }

    /// 验证 Cargo crate 校验和
    pub async fn verify_cargo_crate(
        &self,
        namespace: &str,
        crate_name: &str,
        version: &str,
        expected_sha256: &str,
    ) -> Result<bool, StorageError> {
        let path = self.cargo_crate_path(namespace, crate_name, version);
        if !path.exists() {
            return Err(StorageError::NotFound(format!(
                "Crate {}-{} not found",
                crate_name, version
            )));
        }

        let actual_sha256 = self.calculate_sha256(&path).await?;
        Ok(actual_sha256 == expected_sha256)
    }

    /// 删除 Cargo crate
    pub async fn delete_cargo_crate(
        &self,
        namespace: &str,
        crate_name: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        let path = self.cargo_crate_path(namespace, crate_name, version);
        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                StorageError::IoError(format!("Failed to delete crate: {}", e))
            })?;
            info!("Deleted Cargo crate: {}", path.display());

            // 尝试清理空目录
            if let Some(parent) = path.parent() {
                let _ = fs::remove_dir(parent).await; // 忽略错误（目录非空）
                if let Some(grandparent) = parent.parent() {
                    let _ = fs::remove_dir(grandparent).await;
                }
            }
        }
        Ok(())
    }

    /// 获取 Cargo crate 存储目录（用于列出版本）
    pub fn cargo_crate_dir(&self, namespace: &str, crate_name: &str) -> PathBuf {
        self.base_path
            .join("cargo")
            .join(namespace)
            .join(crate_name)
    }

    /// 获取 Cargo 存储根目录
    pub fn cargo_path(&self) -> PathBuf {
        self.base_path.join("cargo")
    }

    /// 列出 Cargo crate 的所有版本
    pub async fn list_cargo_versions(&self, namespace: &str, crate_name: &str) -> Result<Vec<String>, StorageError> {
        let dir = self.cargo_crate_dir(namespace, crate_name);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        let mut entries = fs::read_dir(&dir).await.map_err(|e| {
            StorageError::IoError(format!("Failed to read crate directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            StorageError::IoError(format!("Failed to read directory entry: {}", e))
        })? {
            if entry.path().is_dir() {
                if let Some(version) = entry.file_name().to_str() {
                    versions.push(version.to_string());
                }
            }
        }

        Ok(versions)
    }

    // ========================================================================
    // 临时文件操作
    // ========================================================================

    /// 创建临时文件
    pub async fn create_temp_file(&self) -> std::io::Result<(PathBuf, File)> {
        let temp_name = format!("upload-{}", uuid::Uuid::new_v4());
        let temp_path = self.tmp_path.join(&temp_name);
        let file = File::create(&temp_path).await?;
        Ok((temp_path, file))
    }

    /// 追加数据到临时文件
    pub async fn append_to_temp(&self, path: &Path, data: &[u8]) -> std::io::Result<i64> {
        use tokio::io::AsyncSeekExt;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .await?;
        file.write_all(data).await?;
        let metadata = file.metadata().await?;
        Ok(metadata.len() as i64)
    }

    /// 获取临时文件大小
    pub async fn temp_file_size(&self, path: &Path) -> std::io::Result<i64> {
        let metadata = fs::metadata(path).await?;
        Ok(metadata.len() as i64)
    }

    /// 删除临时文件
    pub async fn delete_temp_file(&self, path: &Path) -> std::io::Result<()> {
        if path.exists() {
            fs::remove_file(path).await?;
        }
        Ok(())
    }

    // ========================================================================
    // 辅助方法
    // ========================================================================

    /// 计算文件的 SHA-256
    async fn calculate_sha256(&self, path: &Path) -> Result<String, StorageError> {
        let mut file = File::open(path).await.map_err(|e| {
            StorageError::IoError(format!("Failed to open file for hashing: {}", e))
        })?;

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

        loop {
            let n = file.read(&mut buffer).await.map_err(|e| {
                StorageError::IoError(format!("Failed to read file: {}", e))
            })?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    /// 计算数据的 SHA-256
    pub fn calculate_sha256_bytes(data: &[u8]) -> String {
        hex::encode(Sha256::digest(data))
    }

    /// 获取临时目录路径
    pub fn tmp_path(&self) -> &Path {
        &self.tmp_path
    }
    
    /// 获取存储根目录
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

/// 存储错误
#[derive(Debug)]
pub enum StorageError {
    IoError(String),
    DigestMismatch { expected: String, actual: String },
    SizeMismatch { expected: i64, actual: i64 },
    NotFound(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::IoError(msg) => write!(f, "IO error: {}", msg),
            StorageError::DigestMismatch { expected, actual } => {
                write!(f, "Digest mismatch: expected {}, got {}", expected, actual)
            }
            StorageError::SizeMismatch { expected, actual } => {
                write!(f, "Size mismatch: expected {}, got {}", expected, actual)
            }
            StorageError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}
