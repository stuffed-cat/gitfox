//! 内置服务管理模块
//!
//! 管理内置的 PostgreSQL、Redis、Nginx 服务的启动、停止和健康检查。
//!
//! # 架构
//!
//! ```
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  gitfox start (服务编排)                      │
//! ├─────────────────────────────────────────────────────────────┤
//! │  1. 启动内置 PostgreSQL（如果启用）                           │
//! │     └── 等待端口可用                                         │
//! │  2. 启动内置 Redis（如果启用）                                │
//! │     └── 等待端口可用                                         │
//! │  3. 启动 GitFox 核心服务                                     │
//! │     ├── GitLayer                                            │
//! │     ├── Backend (devops)                                    │
//! │     ├── Shell (SSH)                                         │
//! │     └── Workhorse                                           │
//! │  4. 启动内置 Nginx（如果启用）                                │
//! │     └── 作为前端反向代理                                     │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use anyhow::{Context, Result};
use std::fs::{self, Permissions};
use std::io::{BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tracing::{error, info, warn};

#[cfg(feature = "bundled-deps")]
use rust_embed::RustEmbed;

// ============================================================================
// 内置依赖资源（仅在启用 bundled-deps feature 时可用）
// ============================================================================

#[cfg(feature = "bundled-deps")]
#[derive(RustEmbed)]
#[folder = "embedded/deps"]
#[prefix = ""]
struct BundledDepsAssets;

// ============================================================================
// 配置结构
// ============================================================================

/// 内置 PostgreSQL 配置
#[derive(Debug, Clone)]
pub struct BundledPostgresConfig {
    pub enabled: bool,
    pub port: u16,
    pub host: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
    pub shared_buffers_mb: u32,
    pub work_mem_mb: u32,
    pub data_dir: PathBuf,
}

impl Default for BundledPostgresConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 5432,
            host: "127.0.0.1".to_string(),
            database: "gitfox".to_string(),
            username: "gitfox".to_string(),
            password: String::new(),
            max_connections: 100,
            shared_buffers_mb: 256,
            work_mem_mb: 4,
            data_dir: PathBuf::from("/var/lib/gitfox/postgresql/data"),
        }
    }
}

/// 内置 Redis 配置
#[derive(Debug, Clone)]
pub struct BundledRedisConfig {
    pub enabled: bool,
    pub port: u16,
    pub host: String,
    pub maxmemory_mb: u32,
    pub maxmemory_policy: String,
    pub persistence: bool,
    pub persistence_mode: String,
    pub data_dir: PathBuf,
}

impl Default for BundledRedisConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 6379,
            host: "127.0.0.1".to_string(),
            maxmemory_mb: 256,
            maxmemory_policy: "volatile-lru".to_string(),
            persistence: true,
            persistence_mode: "rdb".to_string(),
            data_dir: PathBuf::from("/var/lib/gitfox/redis/data"),
        }
    }
}

/// 内置 Nginx 配置
#[derive(Debug, Clone)]
pub struct BundledNginxConfig {
    pub enabled: bool,
    pub http_port: u16,
    pub https_port: u16,
    pub host: String,
    pub ssl_enabled: bool,
    pub ssl_certificate: String,
    pub ssl_certificate_key: String,
    /// 上游服务器列表（支持 workhorse 集群负载均衡）
    pub upstream_servers: Vec<String>,
    pub client_max_body_size: String,
    pub worker_processes: u32,
    pub worker_connections: u32,
    pub conf_dir: PathBuf,
    pub log_dir: PathBuf,
    // Package Registry 配置
    pub registry_enabled: bool,
    pub registry_domain: String,
}

impl Default for BundledNginxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            http_port: 80,
            https_port: 443,
            host: "0.0.0.0".to_string(),
            ssl_enabled: false,
            ssl_certificate: String::new(),
            ssl_certificate_key: String::new(),
            upstream_servers: vec!["127.0.0.1:8080".to_string()],
            client_max_body_size: "1g".to_string(),
            worker_processes: 0, // auto
            worker_connections: 1024,
            conf_dir: PathBuf::from("/var/lib/gitfox/nginx/conf"),
            log_dir: PathBuf::from("/var/lib/gitfox/nginx/logs"),
            registry_enabled: false,
            registry_domain: String::new(),
        }
    }
}

// ============================================================================
// 服务管理器
// ============================================================================

/// 内置服务管理器
pub struct BundledServices {
    /// 数据目录
    data_dir: PathBuf,
    /// PostgreSQL 进程
    postgres: Option<Child>,
    /// Redis 进程
    redis: Option<Child>,
    /// Nginx 进程
    nginx: Option<Child>,
}

impl BundledServices {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            postgres: None,
            redis: None,
            nginx: None,
        }
    }

    /// 检查是否有内置依赖可用
    #[cfg(feature = "bundled-deps")]
    pub fn has_bundled_deps() -> bool {
        // 检查是否有嵌入的依赖
        BundledDepsAssets::iter().next().is_some()
    }

    #[cfg(not(feature = "bundled-deps"))]
    pub fn has_bundled_deps() -> bool {
        false
    }

    /// 解压内置依赖到数据目录
    #[cfg(feature = "bundled-deps")]
    pub fn extract_deps(&self) -> Result<()> {
        let deps_dir = self.data_dir.join("deps");
        if deps_dir.exists() {
            info!("Bundled dependencies already extracted");
            return Ok(());
        }

        info!("Extracting bundled dependencies...");
        fs::create_dir_all(&deps_dir)?;

        for file in BundledDepsAssets::iter() {
            let content = BundledDepsAssets::get(&file)
                .context(format!("Failed to read embedded file: {}", file))?;
            
            let dest = deps_dir.join(file.as_ref());
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&dest, content.data.as_ref())?;

            // 设置可执行权限（对于 bin 目录下的文件）
            if file.contains("/bin/") || file.contains("/sbin/") {
                fs::set_permissions(&dest, Permissions::from_mode(0o755))?;
            }
        }

        info!("Bundled dependencies extracted to: {}", deps_dir.display());
        Ok(())
    }

    #[cfg(not(feature = "bundled-deps"))]
    pub fn extract_deps(&self) -> Result<()> {
        Ok(())
    }

    /// 启动内置 PostgreSQL
    pub fn start_postgresql(&mut self, config: &BundledPostgresConfig) -> Result<String> {
        if !config.enabled {
            return Ok(String::new());
        }

        #[cfg(not(feature = "bundled-deps"))]
        {
            return Err(anyhow::anyhow!(
                "Bundled PostgreSQL is not available. Build with --bundled-deps to include it."
            ));
        }

        #[cfg(feature = "bundled-deps")]
        {
            info!("Starting bundled PostgreSQL on {}:{}...", config.host, config.port);

            let deps_dir = self.data_dir.join("deps/postgresql");
            let postgres_bin = deps_dir.join("bin/postgres");
            let initdb_bin = deps_dir.join("bin/initdb");
            let lib_dir = self.data_dir.join("deps/shared/lib"); // 使用统一的共享库目录

            if !postgres_bin.exists() {
                return Err(anyhow::anyhow!(
                    "PostgreSQL binary not found: {}",
                    postgres_bin.display()
                ));
            }

            // 查找 musl 动态链接器 (ld-musl-x86_64.so.1)
            // PostgreSQL 使用 musl libc 动态链接编译，需要通过 ld-musl 启动
            // 这避免了系统需要安装 musl，也避免了 patchelf 修改 ELF interpreter
            let ld_musl = self.find_musl_loader(&lib_dir)?;
            info!("Using musl loader: {}", ld_musl.display());

            // 设置 LD_LIBRARY_PATH 让动态链接器找到所有 .so
            let ld_library_path = lib_dir.to_string_lossy().to_string();

            // 创建数据目录
            fs::create_dir_all(&config.data_dir)?;

            // 初始化数据库（如果是新安装）
            let pg_version_file = config.data_dir.join("PG_VERSION");
            if !pg_version_file.exists() {
                info!("Initializing PostgreSQL database...");
                // 通过 ld-musl 执行 initdb
                let status = Command::new(&ld_musl)
                    .arg(&initdb_bin)
                    .args([
                        "-D", &config.data_dir.to_string_lossy(),
                        "-U", &config.username,
                        "--encoding=UTF8",
                        "--locale=C",
                    ])
                    .env("LD_LIBRARY_PATH", &ld_library_path)
                    .status()?;

                if !status.success() {
                    return Err(anyhow::anyhow!("Failed to initialize PostgreSQL database"));
                }

                // 创建自定义配置
                self.write_postgresql_config(config)?;
            }

            // 通过 ld-musl 启动 PostgreSQL
            // 命令格式: /path/to/ld-musl-x86_64.so.1 /path/to/postgres -D ... -p ... -h ...
            let child = Command::new(&ld_musl)
                .arg(&postgres_bin)
                .args([
                    "-D", &config.data_dir.to_string_lossy(),
                    "-p", &config.port.to_string(),
                    "-h", &config.host,
                ])
                .env("LD_LIBRARY_PATH", &ld_library_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("Failed to start PostgreSQL")?;

            self.postgres = Some(child);

            // 等待 PostgreSQL 启动
            self.wait_for_port(&config.host, config.port, Duration::from_secs(30))?;

            // 创建数据库和用户
            self.setup_postgresql_database(config)?;

            // 返回连接字符串
            let password_part = if config.password.is_empty() {
                String::new()
            } else {
                format!(":{}", config.password)
            };
            
            Ok(format!(
                "postgres://{}{}@{}:{}/{}",
                config.username, password_part, config.host, config.port, config.database
            ))
        }
    }

    #[cfg(feature = "bundled-deps")]
    fn write_postgresql_config(&self, config: &BundledPostgresConfig) -> Result<()> {
        let conf_file = config.data_dir.join("postgresql.conf");
        
        let conf_content = format!(r#"
# GitFox Bundled PostgreSQL Configuration
listen_addresses = '{}'
port = {}
max_connections = {}
shared_buffers = {}MB
work_mem = {}MB
maintenance_work_mem = 64MB
effective_cache_size = 512MB
wal_level = replica
max_wal_size = 1GB
min_wal_size = 80MB
logging_collector = on
log_directory = 'log'
log_filename = 'postgresql-%Y-%m-%d.log'
log_rotation_age = 1d
"#,
            config.host,
            config.port,
            config.max_connections,
            config.shared_buffers_mb,
            config.work_mem_mb,
        );

        fs::write(&conf_file, conf_content)?;

        // pg_hba.conf
        let hba_file = config.data_dir.join("pg_hba.conf");
        let hba_content = r#"
# GitFox Bundled PostgreSQL HBA Configuration
local   all             all                                     trust
host    all             all             127.0.0.1/32            md5
host    all             all             ::1/128                 md5
"#;
        fs::write(&hba_file, hba_content)?;

        Ok(())
    }

    /// 查找 musl 动态链接器 (ld-musl-x86_64.so.1)
    /// PostgreSQL 使用 musl 动态链接编译，需要通过此加载器启动
    #[cfg(feature = "bundled-deps")]
    fn find_musl_loader(&self, lib_dir: &Path) -> Result<PathBuf> {
        // 查找 ld-musl-*.so.1 文件
        for entry in fs::read_dir(lib_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("ld-musl") && name.ends_with(".so.1") {
                return Ok(entry.path());
            }
        }
        
        // 备选：查找符号链接可能指向的位置
        let ld_musl = lib_dir.join("ld-musl-x86_64.so.1");
        if ld_musl.exists() {
            return Ok(ld_musl);
        }
        
        Err(anyhow::anyhow!(
            "musl dynamic loader not found in {}. Expected ld-musl-x86_64.so.1",
            lib_dir.display()
        ))
    }

    #[cfg(feature = "bundled-deps")]
    fn setup_postgresql_database(&self, config: &BundledPostgresConfig) -> Result<()> {
        let deps_dir = self.data_dir.join("deps/postgresql");
        let lib_dir = self.data_dir.join("deps/shared/lib"); // 使用统一的共享库目录
        let psql_bin = deps_dir.join("bin/psql");
        
        // 使用 musl loader 执行 psql
        let ld_musl = self.find_musl_loader(&lib_dir)?;
        let ld_library_path = lib_dir.to_string_lossy().to_string();

        // 检查数据库是否存在
        let output = Command::new(&ld_musl)
            .arg(&psql_bin)
            .args([
                "-h", &config.host,
                "-p", &config.port.to_string(),
                "-U", &config.username,
                "-tAc", &format!("SELECT 1 FROM pg_database WHERE datname = '{}'", config.database),
            ])
            .env("LD_LIBRARY_PATH", &ld_library_path)
            .output()?;

        if output.stdout.is_empty() || String::from_utf8_lossy(&output.stdout).trim() != "1" {
            info!("Creating database: {}", config.database);
            Command::new(&ld_musl)
                .arg(&psql_bin)
                .args([
                    "-h", &config.host,
                    "-p", &config.port.to_string(),
                    "-U", &config.username,
                    "-c", &format!("CREATE DATABASE {}", config.database),
                ])
                .env("LD_LIBRARY_PATH", &ld_library_path)
                .status()?;
        }

        // 如果设置了密码，更新用户密码
        if !config.password.is_empty() {
            Command::new(&ld_musl)
                .arg(&psql_bin)
                .args([
                    "-h", &config.host,
                    "-p", &config.port.to_string(),
                    "-U", &config.username,
                    "-c", &format!("ALTER USER {} WITH PASSWORD '{}'", config.username, config.password),
                ])
                .env("LD_LIBRARY_PATH", &ld_library_path)
                .status()?;
        }

        Ok(())
    }

    /// 启动内置 Redis
    pub fn start_redis(&mut self, config: &BundledRedisConfig) -> Result<String> {
        if !config.enabled {
            return Ok(String::new());
        }

        #[cfg(not(feature = "bundled-deps"))]
        {
            return Err(anyhow::anyhow!(
                "Bundled Redis is not available. Build with --bundled-deps to include it."
            ));
        }

        #[cfg(feature = "bundled-deps")]
        {
            info!("Starting bundled Redis on {}:{}...", config.host, config.port);

            let deps_dir = self.data_dir.join("deps/redis");
            let redis_bin = deps_dir.join("bin/redis-server");
            let lib_dir = self.data_dir.join("deps/shared/lib"); // 使用统一的共享库目录

            if !redis_bin.exists() {
                return Err(anyhow::anyhow!(
                    "Redis binary not found: {}",
                    redis_bin.display()
                ));
            }

            // 查找 musl 动态链接器（Redis 也是用 musl 动态链接编译的）
            let ld_musl = self.find_musl_loader(&lib_dir)?;
            info!("Using musl loader for Redis: {}", ld_musl.display());

            // 设置 LD_LIBRARY_PATH
            let ld_library_path = lib_dir.to_string_lossy().to_string();

            // 创建数据目录
            fs::create_dir_all(&config.data_dir)?;

            // 生成配置文件
            let conf_file = config.data_dir.join("redis.conf");
            self.write_redis_config(config, &conf_file)?;

            // 通过 ld-musl 启动 Redis
            // 命令格式: /path/to/ld-musl-x86_64.so.1 /path/to/redis-server config.conf
            let child = Command::new(&ld_musl)
                .arg(&redis_bin)
                .arg(&conf_file)
                .env("LD_LIBRARY_PATH", &ld_library_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("Failed to start Redis")?;

            self.redis = Some(child);

            // 等待 Redis 启动
            self.wait_for_port(&config.host, config.port, Duration::from_secs(10))?;

            // 返回连接字符串
            Ok(format!("redis://{}:{}", config.host, config.port))
        }
    }

    #[cfg(feature = "bundled-deps")]
    fn write_redis_config(&self, config: &BundledRedisConfig, conf_file: &Path) -> Result<()> {
        let mut conf_content = format!(r#"
# GitFox Bundled Redis Configuration
bind {}
port {}
daemonize no
dir {}
maxmemory {}mb
maxmemory-policy {}
"#,
            config.host,
            config.port,
            config.data_dir.display(),
            config.maxmemory_mb,
            config.maxmemory_policy,
        );

        // 持久化配置
        if config.persistence {
            match config.persistence_mode.as_str() {
                "aof" => {
                    conf_content.push_str("appendonly yes\n");
                    conf_content.push_str("appendfsync everysec\n");
                }
                "rdb+aof" => {
                    conf_content.push_str("save 900 1\n");
                    conf_content.push_str("save 300 10\n");
                    conf_content.push_str("save 60 10000\n");
                    conf_content.push_str("appendonly yes\n");
                    conf_content.push_str("appendfsync everysec\n");
                }
                _ => {
                    // rdb
                    conf_content.push_str("save 900 1\n");
                    conf_content.push_str("save 300 10\n");
                    conf_content.push_str("save 60 10000\n");
                }
            }
        } else {
            conf_content.push_str("save \"\"\n");
        }

        fs::write(conf_file, conf_content)?;
        Ok(())
    }

    /// 启动内置 Nginx
    pub fn start_nginx(&mut self, config: &BundledNginxConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        #[cfg(not(feature = "bundled-deps"))]
        {
            return Err(anyhow::anyhow!(
                "Bundled Nginx is not available. Build with --bundled-deps to include it."
            ));
        }

        #[cfg(feature = "bundled-deps")]
        {
            info!("Starting bundled Nginx on {}:{}...", config.host, config.http_port);

            let deps_dir = self.data_dir.join("deps/nginx");
            let nginx_bin = deps_dir.join("sbin/nginx");
            let lib_dir = self.data_dir.join("deps/shared/lib"); // 使用统一的共享库目录

            if !nginx_bin.exists() {
                return Err(anyhow::anyhow!(
                    "Nginx binary not found: {}",
                    nginx_bin.display()
                ));
            }

            // 查找 musl 动态链接器（Nginx 也是用 musl 动态链接编译的）
            let ld_musl = self.find_musl_loader(&lib_dir)?;
            info!("Using musl loader for Nginx: {}", ld_musl.display());

            // 设置 LD_LIBRARY_PATH
            let ld_library_path = lib_dir.to_string_lossy().to_string();

            // 创建目录
            fs::create_dir_all(&config.conf_dir)?;
            fs::create_dir_all(&config.log_dir)?;

            // 生成配置文件
            self.write_nginx_config(config)?;

            // 通过 ld-musl 启动 Nginx
            // 命令格式: /path/to/ld-musl-x86_64.so.1 /path/to/nginx -c config -p prefix
            let child = Command::new(&ld_musl)
                .arg(&nginx_bin)
                .args([
                    "-c", &config.conf_dir.join("nginx.conf").to_string_lossy(),
                    "-p", &self.data_dir.join("nginx").to_string_lossy(),
                ])
                .env("LD_LIBRARY_PATH", &ld_library_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("Failed to start Nginx")?;

            self.nginx = Some(child);

            // 等待 Nginx 启动
            self.wait_for_port(&config.host, config.http_port, Duration::from_secs(10))?;

            info!("Nginx started successfully");
            Ok(())
        }
    }

    #[cfg(feature = "bundled-deps")]
    fn write_nginx_config(&self, config: &BundledNginxConfig) -> Result<()> {
        let worker_processes = if config.worker_processes == 0 {
            "auto".to_string()
        } else {
            config.worker_processes.to_string()
        };

        // 构建 upstream servers 列表（支持集群负载均衡）
        let upstream_servers = config.upstream_servers
            .iter()
            .map(|s| format!("        server {};", s))
            .collect::<Vec<_>>()
            .join("\n");

        let mut server_block = format!(r#"
    server {{
        listen {} default_server;
        listen [::]:{}  default_server;
        server_name _;

        client_max_body_size {};

        location / {{
            proxy_pass http://gitfox_backend;
            proxy_http_version 1.1;
            proxy_set_header Host $http_host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
        }}

        location ~ \.git {{
            proxy_pass http://gitfox_backend;
            proxy_http_version 1.1;
            proxy_read_timeout 300s;
            proxy_connect_timeout 75s;
        }}
    }}
"#,
            config.http_port,
            config.http_port,
            config.client_max_body_size,
        );

        // SSL 配置
        if config.ssl_enabled && !config.ssl_certificate.is_empty() {
            server_block.push_str(&format!(r#"
    server {{
        listen {} ssl http2 default_server;
        listen [::]:{}  ssl http2 default_server;
        server_name _;

        ssl_certificate {};
        ssl_certificate_key {};
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_prefer_server_ciphers on;

        client_max_body_size {};

        location / {{
            proxy_pass http://gitfox_backend;
            proxy_http_version 1.1;
            proxy_set_header Host $http_host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
        }}
    }}
"#,
                config.https_port,
                config.https_port,
                config.ssl_certificate,
                config.ssl_certificate_key,
                config.client_max_body_size,
            ));
        }

        // Package Registry 独立域名配置
        if config.registry_enabled && !config.registry_domain.is_empty() {
            // HTTP server for registry domain
            server_block.push_str(&format!(r#"
    # Package Registry (Docker/npm)
    server {{
        listen {};
        listen [::]:{}; 
        server_name {};

        client_max_body_size 500m;

        # Docker Registry V2 API
        location /v2/ {{
            proxy_pass http://gitfox_backend/v2/;
            proxy_http_version 1.1;
            proxy_set_header Host $http_host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 600s;
            proxy_send_timeout 600s;
            proxy_buffering off;
            chunked_transfer_encoding on;
        }}

        # npm Registry API
        location / {{
            proxy_pass http://gitfox_backend/npm/;
            proxy_http_version 1.1;
            proxy_set_header Host $http_host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 300s;
        }}
    }}
"#,
                config.http_port,
                config.http_port,
                config.registry_domain,
            ));

            // HTTPS server for registry domain (if SSL enabled)
            if config.ssl_enabled && !config.ssl_certificate.is_empty() {
                server_block.push_str(&format!(r#"
    # Package Registry (Docker/npm) - HTTPS
    server {{
        listen {} ssl http2;
        listen [::]:{}  ssl http2;
        server_name {};

        ssl_certificate {};
        ssl_certificate_key {};
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_prefer_server_ciphers on;

        client_max_body_size 500m;

        # Docker Registry V2 API
        location /v2/ {{
            proxy_pass http://gitfox_backend/v2/;
            proxy_http_version 1.1;
            proxy_set_header Host $http_host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 600s;
            proxy_send_timeout 600s;
            proxy_buffering off;
            chunked_transfer_encoding on;
        }}

        # npm Registry API
        location / {{
            proxy_pass http://gitfox_backend/npm/;
            proxy_http_version 1.1;
            proxy_set_header Host $http_host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_read_timeout 300s;
        }}
    }}
"#,
                    config.https_port,
                    config.https_port,
                    config.registry_domain,
                    config.ssl_certificate,
                    config.ssl_certificate_key,
                ));
            }
        }

        let conf_content = format!(r#"
worker_processes {};
error_log {} error;
pid {}/nginx.pid;

events {{
    worker_connections {};
    use epoll;
    multi_accept on;
}}

http {{
    include mime.types;
    default_type application/octet-stream;

    log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                    '$status $body_bytes_sent "$http_referer" '
                    '"$http_user_agent" "$http_x_forwarded_for"';

    access_log {}/access.log main;

    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;

    proxy_buffering on;
    proxy_buffer_size 4k;
    proxy_buffers 8 32k;
    proxy_busy_buffers_size 64k;

    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;

    # 上游服务器组（Workhorse 集群）
    upstream gitfox_backend {{
{}
        keepalive 32;
    }}

{}
}}
"#,
            worker_processes,
            config.log_dir.join("error.log").display(),
            self.data_dir.join("nginx").display(),
            config.worker_connections,
            config.log_dir.display(),
            upstream_servers,
            server_block,
        );

        fs::write(config.conf_dir.join("nginx.conf"), conf_content)?;

        // 复制 mime.types（如果不存在）
        let mime_types_dest = config.conf_dir.join("mime.types");
        if !mime_types_dest.exists() {
            let deps_dir = self.data_dir.join("deps/nginx");
            let mime_types_src = deps_dir.join("conf/mime.types");
            if mime_types_src.exists() {
                fs::copy(&mime_types_src, &mime_types_dest)?;
            } else {
                return Err(anyhow::anyhow!(
                    "mime.types not found at {}. Bundled nginx is corrupted.",
                    mime_types_src.display()
                ));
            }
        }

        Ok(())
    }

    /// 等待端口可用
    fn wait_for_port(&self, host: &str, port: u16, timeout: Duration) -> Result<()> {
        use std::net::TcpStream;
        use std::time::Instant;

        let start = Instant::now();
        let addr = format!("{}:{}", host, port);

        while start.elapsed() < timeout {
            if TcpStream::connect(&addr).is_ok() {
                info!("Port {} is ready", port);
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        Err(anyhow::anyhow!(
            "Timeout waiting for port {} to become available",
            port
        ))
    }

    /// 停止所有内置服务
    pub fn stop_all(&mut self) {
        if let Some(ref mut nginx) = self.nginx {
            info!("Stopping Nginx...");
            let _ = nginx.kill();
            let _ = nginx.wait();
        }

        if let Some(ref mut redis) = self.redis {
            info!("Stopping Redis...");
            let _ = redis.kill();
            let _ = redis.wait();
        }

        if let Some(ref mut postgres) = self.postgres {
            info!("Stopping PostgreSQL...");
            // PostgreSQL 需要优雅关闭
            let _ = Command::new("kill")
                .args(["-SIGTERM", &postgres.id().to_string()])
                .status();
            let _ = postgres.wait();
        }
    }
}

impl Drop for BundledServices {
    fn drop(&mut self) {
        self.stop_all();
    }
}

// 内置的基本 mime.types（当无法找到时使用）
#[cfg(feature = "bundled-deps")]
const _MIME_TYPES_DEFAULT: &str = r#"
types {
    text/html                             html htm shtml;
    text/css                              css;
    text/xml                              xml;
    image/gif                             gif;
    image/jpeg                            jpeg jpg;
    application/javascript                js;
    application/json                      json;
    text/plain                            txt;
    image/png                             png;
    image/svg+xml                         svg svgz;
    image/webp                            webp;
    image/x-icon                          ico;
    font/woff                             woff;
    font/woff2                            woff2;
    application/octet-stream              bin exe dll;
    application/zip                       zip;
    application/gzip                      gz;
}
"#;
