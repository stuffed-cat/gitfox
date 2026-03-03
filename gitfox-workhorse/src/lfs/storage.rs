//! LFS 对象存储层
//!
//! 负责 LFS 对象的本地文件系统存储。
//! 使用内容寻址存储，按 OID 的前两个字符分目录存储。

use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use sha2::{Sha256, Digest};
use tracing::{debug, error, info};

/// LFS 对象存储
#[derive(Clone)]
pub struct LfsStorage {
    /// 存储根目录
    base_path: PathBuf,
    /// 临时文件目录
    tmp_path: PathBuf,
}

impl LfsStorage {
    /// 创建新的 LFS 存储实例
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
        info!("LFS storage initialized at {:?}", self.base_path);
        Ok(())
    }

    /// 获取对象的存储路径
    /// 格式: base/{oid[0:2]}/{oid[2:4]}/{oid}
    fn object_path(&self, oid: &str) -> PathBuf {
        if oid.len() < 4 {
            return self.base_path.join(oid);
        }
        self.base_path
            .join(&oid[0..2])
            .join(&oid[2..4])
            .join(oid)
    }

    /// 检查对象是否存在
    pub async fn exists(&self, oid: &str) -> bool {
        let path = self.object_path(oid);
        path.exists()
    }

    /// 获取对象大小（如果存在）
    pub async fn get_size(&self, oid: &str) -> Option<u64> {
        let path = self.object_path(oid);
        fs::metadata(&path).await.ok().map(|m| m.len())
    }

    /// 获取对象信息
    pub async fn get_object_info(&self, oid: &str) -> Option<LfsObjectInfo> {
        let path = self.object_path(oid);
        let metadata = fs::metadata(&path).await.ok()?;
        Some(LfsObjectInfo {
            oid: oid.to_string(),
            size: metadata.len() as i64,
            path,
        })
    }

    /// 打开对象文件用于读取
    pub async fn open_object(&self, oid: &str) -> std::io::Result<File> {
        let path = self.object_path(oid);
        File::open(&path).await
    }

    /// 创建临时文件用于上传
    pub async fn create_temp_file(&self) -> std::io::Result<(PathBuf, File)> {
        let temp_name = format!("upload-{}", uuid::Uuid::new_v4());
        let temp_path = self.tmp_path.join(&temp_name);
        let file = File::create(&temp_path).await?;
        Ok((temp_path, file))
    }

    /// 将临时文件移动到最终位置并验证
    pub async fn finalize_upload(
        &self,
        temp_path: &Path,
        expected_oid: &str,
        expected_size: i64,
    ) -> Result<(), LfsStorageError> {
        // 验证文件大小
        let metadata = fs::metadata(temp_path).await.map_err(|e| {
            LfsStorageError::IoError(format!("Failed to get temp file metadata: {}", e))
        })?;

        if metadata.len() != expected_size as u64 {
            // 清理临时文件
            let _ = fs::remove_file(temp_path).await;
            return Err(LfsStorageError::SizeMismatch {
                expected: expected_size,
                actual: metadata.len() as i64,
            });
        }

        // 计算并验证 OID
        let actual_oid = self.calculate_oid(temp_path).await?;
        if actual_oid != expected_oid {
            // 清理临时文件
            let _ = fs::remove_file(temp_path).await;
            return Err(LfsStorageError::OidMismatch {
                expected: expected_oid.to_string(),
                actual: actual_oid,
            });
        }

        // 创建目标目录
        let final_path = self.object_path(expected_oid);
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                LfsStorageError::IoError(format!("Failed to create object directory: {}", e))
            })?;
        }

        // 如果对象已存在，删除临时文件
        if final_path.exists() {
            debug!("Object {} already exists, removing temp file", expected_oid);
            let _ = fs::remove_file(temp_path).await;
            return Ok(());
        }

        // 移动文件到最终位置
        fs::rename(temp_path, &final_path).await.map_err(|e| {
            LfsStorageError::IoError(format!("Failed to move temp file: {}", e))
        })?;

        info!("LFS object {} stored successfully", expected_oid);
        Ok(())
    }

    /// 计算文件的 SHA-256 OID
    async fn calculate_oid(&self, path: &Path) -> Result<String, LfsStorageError> {
        let file = File::open(path).await.map_err(|e| {
            LfsStorageError::IoError(format!("Failed to open file for hashing: {}", e))
        })?;

        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

        loop {
            let bytes_read = reader.read(&mut buffer).await.map_err(|e| {
                LfsStorageError::IoError(format!("Failed to read file: {}", e))
            })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    /// 删除对象
    pub async fn delete_object(&self, oid: &str) -> std::io::Result<()> {
        let path = self.object_path(oid);
        if path.exists() {
            fs::remove_file(&path).await?;
            debug!("Deleted LFS object: {}", oid);
        }
        Ok(())
    }

    /// 清理过期的临时文件
    pub async fn cleanup_temp_files(&self, max_age_secs: u64) -> std::io::Result<usize> {
        let mut count = 0;
        let now = std::time::SystemTime::now();

        let mut entries = fs::read_dir(&self.tmp_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() > max_age_secs {
                            let _ = fs::remove_file(entry.path()).await;
                            count += 1;
                        }
                    }
                }
            }
        }

        if count > 0 {
            info!("Cleaned up {} expired temp files", count);
        }
        Ok(count)
    }

    /// 获取存储统计信息
    pub async fn get_stats(&self) -> LfsStorageStats {
        let mut total_objects = 0u64;
        let mut total_size = 0u64;

        // 遍历所有对象目录
        if let Ok(mut dirs) = fs::read_dir(&self.base_path).await {
            while let Ok(Some(dir1)) = dirs.next_entry().await {
                if dir1.path().is_dir() && dir1.file_name() != "tmp" {
                    if let Ok(mut dirs2) = fs::read_dir(dir1.path()).await {
                        while let Ok(Some(dir2)) = dirs2.next_entry().await {
                            if dir2.path().is_dir() {
                                if let Ok(mut files) = fs::read_dir(dir2.path()).await {
                                    while let Ok(Some(file)) = files.next_entry().await {
                                        if let Ok(meta) = file.metadata().await {
                                            if meta.is_file() {
                                                total_objects += 1;
                                                total_size += meta.len();
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        LfsStorageStats {
            total_objects,
            total_size,
        }
    }
}

/// LFS 对象信息
#[derive(Debug, Clone)]
pub struct LfsObjectInfo {
    pub oid: String,
    pub size: i64,
    pub path: PathBuf,
}

/// LFS 存储统计
#[derive(Debug, Clone)]
pub struct LfsStorageStats {
    pub total_objects: u64,
    pub total_size: u64,
}

/// LFS 存储错误
#[derive(Debug, thiserror::Error)]
pub enum LfsStorageError {
    #[error("IO error: {0}")]
    IoError(String),

    #[error("Size mismatch: expected {expected}, got {actual}")]
    SizeMismatch { expected: i64, actual: i64 },

    #[error("OID mismatch: expected {expected}, got {actual}")]
    OidMismatch { expected: String, actual: String },

    #[error("Object not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_object_path() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LfsStorage::new(temp_dir.path());

        let oid = "abc123def456";
        let path = storage.object_path(oid);
        assert!(path.ends_with("ab/c1/abc123def456"));
    }

    #[tokio::test]
    async fn test_init_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LfsStorage::new(temp_dir.path());
        storage.init().await.unwrap();

        assert!(temp_dir.path().join("tmp").exists());
    }
}
