//! SSH Host Key Management Service
//!
//! 管理 SSH host key 的生成、存储和同步：
//! - 启动时检查并同步 host key（文件 <-> 数据库）
//! - 支持集群部署：所有节点使用同一个 host key
//! - 支持迁移：已有文件自动上传到数据库

use sqlx::PgPool;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;
use tokio::fs;
use log::{debug, error, info, warn};

/// SSH Host Key 信息
#[derive(Debug, Clone)]
pub struct SshHostKey {
    pub private_key_pem: String,
    pub public_key: String,
    pub fingerprint: String,
}

/// SSH Host Key 服务
pub struct SshHostKeyService;

impl SshHostKeyService {
    /// 确保 SSH host key 存在并同步
    ///
    /// 逻辑：
    /// 1. 如果本地文件存在 → 读取并检查数据库
    ///    - 数据库为空 → 上传到数据库
    ///    - 数据库有不同的 key → 使用数据库的（集群场景）
    /// 2. 如果本地文件不存在 → 检查数据库
    ///    - 数据库有 key → 下载到本地
    ///    - 数据库为空 → 生成新 key，保存到数据库和本地
    pub async fn ensure_host_key(pool: &PgPool, key_path: &Path) -> Result<SshHostKey, String> {
        // 检查本地文件
        let local_key = if key_path.exists() {
            match Self::load_from_file(key_path).await {
                Ok(key) => {
                    info!("Loaded SSH host key from file: {}", key_path.display());
                    Some(key)
                }
                Err(e) => {
                    warn!("Failed to load local SSH host key: {}", e);
                    None
                }
            }
        } else {
            debug!("Local SSH host key file not found: {}", key_path.display());
            None
        };

        // 检查数据库
        let db_key = match Self::load_from_db(pool).await {
            Ok(Some(key)) => {
                info!("Found SSH host key in database with fingerprint: {}", key.fingerprint);
                Some(key)
            }
            Ok(None) => {
                debug!("No SSH host key in database");
                None
            }
            Err(e) => {
                warn!("Failed to load SSH host key from database: {}", e);
                None
            }
        };

        match (local_key, db_key) {
            // 两者都存在
            (Some(local), Some(db)) => {
                if local.fingerprint == db.fingerprint {
                    info!("SSH host key synchronized (fingerprint: {})", local.fingerprint);
                    Ok(local)
                } else {
                    // 数据库优先（集群场景）
                    warn!(
                        "Local SSH host key differs from database, using database key. \
                         Local: {}, DB: {}",
                        local.fingerprint, db.fingerprint
                    );
                    // 更新本地文件
                    if let Err(e) = Self::save_to_file(&db, key_path).await {
                        error!("Failed to update local SSH host key file: {}", e);
                    }
                    Ok(db)
                }
            }
            // 只有本地文件
            (Some(local), None) => {
                info!("Uploading local SSH host key to database");
                if let Err(e) = Self::save_to_db(pool, &local).await {
                    error!("Failed to save SSH host key to database: {}", e);
                }
                Ok(local)
            }
            // 只有数据库
            (None, Some(db)) => {
                info!("Downloading SSH host key from database to local file");
                if let Err(e) = Self::save_to_file(&db, key_path).await {
                    error!("Failed to save SSH host key to file: {}", e);
                }
                Ok(db)
            }
            // 都没有，生成新的
            (None, None) => {
                info!("Generating new SSH host key");
                let key = Self::generate_key()?;
                
                // 保存到数据库
                if let Err(e) = Self::save_to_db(pool, &key).await {
                    error!("Failed to save generated SSH host key to database: {}", e);
                }
                
                // 保存到本地文件
                if let Err(e) = Self::save_to_file(&key, key_path).await {
                    error!("Failed to save generated SSH host key to file: {}", e);
                }
                
                info!("Generated new SSH host key with fingerprint: {}", key.fingerprint);
                Ok(key)
            }
        }
    }

    /// 从文件加载 SSH host key
    async fn load_from_file(path: &Path) -> Result<SshHostKey, String> {
        let private_key_pem = fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read key file: {}", e))?;

        // 解析私钥并派生公钥和指纹
        Self::parse_private_key(&private_key_pem)
    }

    /// 从数据库加载 SSH host key
    async fn load_from_db(pool: &PgPool) -> Result<Option<SshHostKey>, String> {
        let private_key: Option<String> = sqlx::query_scalar(
            r#"SELECT value::text FROM system_configs WHERE key = 'ssh_host_key_private'"#
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .and_then(|v: String| serde_json::from_str::<String>(&v).ok());

        let public_key: Option<String> = sqlx::query_scalar(
            r#"SELECT value::text FROM system_configs WHERE key = 'ssh_host_key_public'"#
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .and_then(|v: String| serde_json::from_str::<String>(&v).ok());

        let fingerprint: Option<String> = sqlx::query_scalar(
            r#"SELECT value::text FROM system_configs WHERE key = 'ssh_host_key_fingerprint'"#
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .and_then(|v: String| serde_json::from_str::<String>(&v).ok());

        match (private_key, public_key, fingerprint) {
            (Some(priv_key), Some(pub_key), Some(fp)) if !priv_key.is_empty() => {
                Ok(Some(SshHostKey {
                    private_key_pem: priv_key,
                    public_key: pub_key,
                    fingerprint: fp,
                }))
            }
            _ => Ok(None),
        }
    }

    /// 保存 SSH host key 到数据库
    async fn save_to_db(pool: &PgPool, key: &SshHostKey) -> Result<(), String> {
        // 使用 JSON 格式存储
        let priv_json = serde_json::to_string(&key.private_key_pem)
            .map_err(|e| format!("JSON encode error: {}", e))?;
        let pub_json = serde_json::to_string(&key.public_key)
            .map_err(|e| format!("JSON encode error: {}", e))?;
        let fp_json = serde_json::to_string(&key.fingerprint)
            .map_err(|e| format!("JSON encode error: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO system_configs (key, value, updated_at) 
            VALUES ('ssh_host_key_private', $1::jsonb, NOW())
            ON CONFLICT (key) DO UPDATE SET value = $1::jsonb, updated_at = NOW()
            "#
        )
        .bind(&priv_json)
        .execute(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO system_configs (key, value, updated_at) 
            VALUES ('ssh_host_key_public', $1::jsonb, NOW())
            ON CONFLICT (key) DO UPDATE SET value = $1::jsonb, updated_at = NOW()
            "#
        )
        .bind(&pub_json)
        .execute(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        sqlx::query(
            r#"
            INSERT INTO system_configs (key, value, updated_at) 
            VALUES ('ssh_host_key_fingerprint', $1::jsonb, NOW())
            ON CONFLICT (key) DO UPDATE SET value = $1::jsonb, updated_at = NOW()
            "#
        )
        .bind(&fp_json)
        .execute(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        Ok(())
    }

    /// 保存 SSH host key 到文件
    async fn save_to_file(key: &SshHostKey, path: &Path) -> Result<(), String> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // 写入私钥文件
        fs::write(path, &key.private_key_pem)
            .await
            .map_err(|e| format!("Failed to write key file: {}", e))?;

        // 设置文件权限为 0600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms)
                .map_err(|e| format!("Failed to set file permissions: {}", e))?;
        }

        Ok(())
    }

    /// 生成新的 Ed25519 SSH host key (使用 ssh-keygen)
    fn generate_key() -> Result<SshHostKey, String> {
        // 创建临时目录
        let temp_dir = tempfile::tempdir()
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let key_path = temp_dir.path().join("host_key");
        let key_path_str = key_path.to_string_lossy();
        
        // 使用 ssh-keygen 生成密钥
        let output = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-f", &key_path_str, "-N", "", "-C", "gitfox@localhost"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to run ssh-keygen: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ssh-keygen failed: {}", stderr));
        }
        
        // 读取生成的私钥
        let private_key_pem = std::fs::read_to_string(&key_path)
            .map_err(|e| format!("Failed to read generated private key: {}", e))?;
        
        // 读取公钥
        let pub_key_path = format!("{}.pub", key_path_str);
        let public_key = std::fs::read_to_string(&pub_key_path)
            .map_err(|e| format!("Failed to read generated public key: {}", e))?
            .trim()
            .to_string();
        
        // 获取指纹
        let fingerprint = Self::get_fingerprint_from_pubkey(&public_key)?;
        
        Ok(SshHostKey {
            private_key_pem,
            public_key,
            fingerprint,
        })
    }

    /// 解析私钥 PEM，派生公钥和指纹
    fn parse_private_key(pem: &str) -> Result<SshHostKey, String> {
        // 创建临时文件写入私钥
        let temp_dir = tempfile::tempdir()
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let key_path = temp_dir.path().join("host_key");
        
        std::fs::write(&key_path, pem)
            .map_err(|e| format!("Failed to write temp key: {}", e))?;
        
        // 设置权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&key_path, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }
        
        // 使用 ssh-keygen 提取公钥
        let output = Command::new("ssh-keygen")
            .args(["-y", "-f", &key_path.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to run ssh-keygen: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to extract public key: {}", stderr));
        }
        
        let public_key = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        // 获取指纹
        let fingerprint = Self::get_fingerprint_from_pubkey(&public_key)?;
        
        Ok(SshHostKey {
            private_key_pem: pem.to_string(),
            public_key,
            fingerprint,
        })
    }

    /// 从公钥字符串获取指纹
    fn get_fingerprint_from_pubkey(public_key: &str) -> Result<String, String> {
        // 使用 ssh-keygen 计算指纹
        let mut child = Command::new("ssh-keygen")
            .args(["-l", "-f", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to run ssh-keygen: {}", e))?;
        
        {
            let stdin = child.stdin.as_mut()
                .ok_or_else(|| "Failed to get stdin".to_string())?;
            stdin.write_all(public_key.as_bytes())
                .map_err(|e| format!("Failed to write to ssh-keygen: {}", e))?;
        }
        
        let output = child.wait_with_output()
            .map_err(|e| format!("Failed to wait for ssh-keygen: {}", e))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to get fingerprint: {}", stderr));
        }
        
        // 解析输出: "256 SHA256:xxxx gitfox@localhost (ED25519)"
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .split_whitespace()
            .nth(1)
            .map(|s| s.to_string())
            .ok_or_else(|| "Failed to parse fingerprint".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let key = SshHostKeyService::generate_key().unwrap();
        
        assert!(key.private_key_pem.contains("BEGIN OPENSSH PRIVATE KEY") 
            || key.private_key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(key.public_key.starts_with("ssh-ed25519 "));
        assert!(key.fingerprint.starts_with("SHA256:"));
        
        // 验证可以重新解析
        let parsed = SshHostKeyService::parse_private_key(&key.private_key_pem).unwrap();
        assert_eq!(parsed.fingerprint, key.fingerprint);
        assert_eq!(parsed.public_key, key.public_key);
    }
}
