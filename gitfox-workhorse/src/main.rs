mod auth_client;
mod config;
mod git_http;
mod gitlayer_client;
mod http_client;
mod lfs;
mod proxy;
mod registry;
mod registry_client;
mod static_files;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
    guard,
    middleware::{Compress, Logger},
    web, App, HttpRequest, HttpServer,
};
use config::Config;
use http_client::BackendClient;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gitfox_workhorse=info,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置（按优先级）
    let config = if let Ok(config_path) = std::env::var("WORKHORSE_CONFIG") {
        // 1. 优先使用 WORKHORSE_CONFIG 环境变量指定的路径
        tracing::info!("Loading config from WORKHORSE_CONFIG: {}", config_path);
        Config::from_file(&config_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load config file: {}, using env vars", e);
            Config::from_env()
        })
    } else if std::path::Path::new("config.toml").exists() {
        // 2. 尝试当前目录的 config.toml
        tracing::info!("Loading config from ./config.toml");
        Config::from_file("config.toml").unwrap_or_else(|e| {
            tracing::warn!("Failed to load ./config.toml: {}, using env vars", e);
            Config::from_env()
        })
    } else {
        // 3. 从环境变量加载
        tracing::info!("Loading config from environment variables");
        Config::from_env()
    };

    // 验证配置
    if let Err(e) = config.validate() {
        tracing::error!("Configuration validation failed: {}", e);
        std::process::exit(1);
    }

    tracing::info!("GitFox Workhorse starting...");
    tracing::info!("Listen address: {}:{}", config.listen_addr, config.listen_port);
    
    // 根据配置创建后端客户端
    let backend_client = if let Some(socket_path) = &config.backend_socket {
        tracing::info!("Backend mode: Unix Socket ({})", socket_path);
        BackendClient::unix(socket_path)
    } else {
        tracing::info!("Backend mode: HTTP ({})", config.backend_url);
        BackendClient::tcp(std::time::Duration::from_secs(300))
            .expect("Failed to create HTTP client")
    };
    
    tracing::info!("Frontend dist: {:?}", config.frontend_dist_path);
    tracing::info!("WebIDE dist: {:?}", config.webide_dist_path);
    tracing::info!("Assets path: {:?}", config.assets_path);

    // 初始化 Auth gRPC 客户端（必需，用于 Git HTTP/LFS/Registry 权限验证）
    // gRPC auth 强制启用，shell/workhorse 依赖此服务进行认证
    let auth_client = {
        let addr = config.auth_grpc_address.as_ref()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Configuration error: auth_grpc_address is required. \
                Git HTTP requires authentication service to function properly."
            ))?;
        
        // 重试连接 Auth gRPC，最多重试 10 次，每次间隔递增
        let mut retry_count = 0;
        let max_retries = 10;
        let mut last_error = None;
        
        loop {
            match auth_client::AuthClient::connect(addr, config.shell_secret.clone()).await {
                Ok(client) => {
                    tracing::info!("Auth gRPC client connected to {}", addr);
                    break client;
                }
                Err(e) => {
                    retry_count += 1;
                    last_error = Some(e.to_string());
                    
                    if retry_count >= max_retries {
                        // 重试次数用尽，退出程序
                        // 不应该在没有认证服务的情况下继续运行
                        tracing::error!(
                            "Failed to connect to Auth gRPC at {} after {} retries: {}",
                            addr, max_retries, last_error.as_ref().unwrap()
                        );
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::ConnectionRefused,
                            format!(
                                "Cannot start workhorse: Auth gRPC service at {} is not available. \
                                Make sure the main gitfox service is running.",
                                addr
                            )
                        ));
                    }
                    
                    let delay = std::time::Duration::from_secs(retry_count as u64);
                    tracing::warn!(
                        "Failed to connect to Auth gRPC at {} (attempt {}/{}): {}. Retrying in {:?}...",
                        addr, retry_count, max_retries, e, delay
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    };

    let client_data = web::Data::new(backend_client);
    let config_data = web::Data::new(config.clone());
    let backend_url_data = web::Data::new(config.backend_url.clone());
    let backend_socket_data = web::Data::new(config.backend_socket.clone());
    let frontend_dist_data = web::Data::new(config.frontend_dist_path.clone());
    let webide_dist_data = web::Data::new(config.webide_dist_path.clone());
    let assets_path_data = web::Data::new(config.assets_path.clone());

    // 初始化 LFS 状态
    let lfs_state = if config.lfs_enabled {
        tracing::info!("LFS enabled, storage path: {:?}", config.lfs_storage_path);
        let state = lfs::LfsState::new(std::sync::Arc::new(config.clone()), auth_client.clone());
        if let Err(e) = state.init().await {
            tracing::error!("Failed to initialize LFS storage: {}", e);
            std::process::exit(1);
        }
        Some(web::Data::new(state))
    } else {
        tracing::info!("LFS disabled");
        None
    };

    // 初始化 Registry 状态
    let registry_config = std::sync::Arc::new(registry::RegistryConfig {
        enabled: config.registry_enabled,
        domain: config.registry_domain.clone(),
        docker_enabled: config.registry_docker_enabled,
        npm_enabled: config.registry_npm_enabled,
        cargo_enabled: config.registry_cargo_enabled,
        storage_path: config.registry_storage_path.clone(),
        max_package_size: config.registry_max_size,
        token_expires: 3600,
    });
    
    let (docker_registry_state, npm_registry_state, cargo_registry_state) = if config.registry_enabled {
        tracing::info!("Package Registry enabled");
        if let Some(ref domain) = config.registry_domain {
            tracing::info!("Registry domain: {}", domain);
        }
        tracing::info!("Registry storage path: {:?}", config.registry_storage_path);
        
        let docker_state = if config.registry_docker_enabled {
            tracing::info!("Docker Registry enabled");
            let state = registry::DockerRegistryState::new(
                registry_config.clone(),
                config.shell_secret.clone(),
                config.backend_url.clone(),
            );
            if let Err(e) = state.init().await {
                tracing::error!("Failed to initialize Docker Registry storage: {}", e);
                std::process::exit(1);
            }
            Some(web::Data::new(state))
        } else {
            None
        };
        
        let npm_state = if config.registry_npm_enabled {
            tracing::info!("npm Registry enabled");
            // registry_domain 在 config.validate() 中已验证为必需
            let base_url = format!("https://{}", config.registry_domain.as_ref().unwrap());
            let state = registry::NpmRegistryState::new(
                registry_config.clone(),
                config.shell_secret.clone(),
                base_url,
                config.backend_url.clone(),
            );
            if let Err(e) = state.init().await {
                tracing::error!("Failed to initialize npm Registry storage: {}", e);
                std::process::exit(1);
            }
            Some(web::Data::new(state))
        } else {
            None
        };

        let cargo_state = if config.registry_cargo_enabled {
            tracing::info!("Cargo Registry enabled");
            // registry_domain 在 config.validate() 中已验证为必需
            let base_url = format!("https://{}", config.registry_domain.as_ref().unwrap());
            let state = registry::cargo::CargoRegistryState::new(
                registry_config.clone(),
                config.shell_secret.clone(),
                base_url,
                config.backend_url.clone(),
            );
            if let Err(e) = state.init().await {
                tracing::error!("Failed to initialize Cargo Registry storage: {}", e);
                std::process::exit(1);
            }
            Some(web::Data::new(state))
        } else {
            None
        };
        
        (docker_state, npm_state, cargo_state)
    } else {
        tracing::info!("Package Registry disabled");
        (None, None, None)
    };

    let listen_addr = config.listen_addr.clone();
    let listen_port = config.listen_port;
    let max_upload_size = config.max_upload_size;
    let lfs_max_upload_size = config.lfs_max_object_size as usize;
    let registry_max_upload_size = config.registry_max_size as usize;

    tracing::info!("Max upload size: {} bytes ({:.2} MB)", max_upload_size, max_upload_size as f64 / 1024.0 / 1024.0);

    // 启动 HTTP 服务器
    HttpServer::new(move || {
        let mut app = App::new()
            .app_data(client_data.clone())
            .app_data(backend_url_data.clone())
            .app_data(backend_socket_data.clone())
            .app_data(frontend_dist_data.clone())
            .app_data(webide_dist_data.clone())
            .app_data(assets_path_data.clone())
            // 配置请求体大小限制
            .app_data(web::PayloadConfig::new(max_upload_size))
            // CORS 配置
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
                    .max_age(3600),
            )
            // 日志中间件
            .wrap(Logger::default())
            // 压缩中间件
            .wrap(Compress::default())
            // 健康检查端点
            .route("/-/health", web::get().to(proxy::health_check))
            .route("/-/workhorse/health", web::get().to(proxy::health_check))
            
            // 用户上传文件 (/assets/upload/*)
            // 必须在 /assets/* 之前注册以获得更高优先级
            .route("/assets/upload/{tail:.*}", web::get().to(static_files::serve_upload))
            
            // 主前端静态资源 (/assets/* -> assets_path/main/*)
            .route("/assets/{tail:.*}", web::get().to(static_files::serve_main_assets))
            
            // WebIDE 静态资源 - 路径重写
            // /-/ide/assets/* (web) -> assets_path/webide/main/* (fs)
            .route("/-/ide/assets/{tail:.*}", web::get().to(static_files::serve_webide_main))
            // /-/ide/extensions/* (web) -> assets_path/webide/extensions/* (fs)
            // 必须在 /-/ide/vscode/* 之前注册，避免被 vscode 路由捕获
            .route("/-/ide/extensions/{tail:.*}", web::get().to(static_files::serve_webide_extensions))
            // /-/ide/vscode/* (web) -> assets_path/webide/vscode/* (fs)
            .route("/-/ide/vscode/{tail:.*}", web::get().to(static_files::serve_webide_vscode))
            
            // WebIDE SPA 入口 (/-/ide/* 返回 webide 的 index.html)
            .service(
                web::scope("/-/ide")
                    .default_service({
                        let webide_dist = webide_dist_data.clone();
                        web::to(move |req: HttpRequest| {
                            let dist = webide_dist.clone();
                            async move { static_files::serve_webide_index(req, dist).await }
                        })
                    })
            )
            
            // API 代理到后端
            .service(
                web::scope("/api")
                    // API 上传也需要更大的限制
                    .app_data(web::PayloadConfig::new(max_upload_size))
                    .default_service(web::to(proxy::proxy_to_backend))
            )
            
            // OAuth API 端点代理（只代理实际的API端点，不包括 /oauth/authorize）
            // /oauth/authorize 由前端 Vue Router 处理（OAuthAuthorizeView.vue）
            // 使用 web::route() 匹配所有 HTTP 方法，与 vite proxy 行为一致
            .route("/oauth/token", web::route().to(proxy::proxy_to_backend))
            .route("/oauth/revoke", web::route().to(proxy::proxy_to_backend))
            .route("/oauth/userinfo", web::route().to(proxy::proxy_to_backend));
            
        // 添加 LFS 路由（如果启用）
        // LFS 路由必须在 Git HTTP 代理之前，因为它们匹配更具体的路径
        if let Some(ref lfs_data) = lfs_state {
            app = app
                .app_data(lfs_data.clone())
                // LFS Batch API
                .route(
                    "/{namespace}/{project}.git/info/lfs/objects/batch",
                    web::post().to(lfs::handle_batch)
                )
                // LFS 对象验证
                .route(
                    "/{namespace}/{project}.git/info/lfs/objects/verify",
                    web::post().to(lfs::handle_verify)
                )
                // LFS 对象下载
                .route(
                    "/{namespace}/{project}.git/info/lfs/objects/{oid}",
                    web::get().to(lfs::handle_download)
                )
                // LFS 对象上传
                .service(
                    web::resource("/{namespace}/{project}.git/info/lfs/objects/{oid}")
                        .app_data(web::PayloadConfig::new(lfs_max_upload_size))
                        .route(web::put().to(lfs::handle_upload))
                )
                // LFS 锁 API
                .route(
                    "/{namespace}/{project}.git/info/lfs/locks",
                    web::post().to(lfs::handle_create_lock)
                )
                .route(
                    "/{namespace}/{project}.git/info/lfs/locks",
                    web::get().to(lfs::handle_list_locks)
                )
                .route(
                    "/{namespace}/{project}.git/info/lfs/locks/verify",
                    web::post().to(lfs::handle_verify_locks)
                )
                .route(
                    "/{namespace}/{project}.git/info/lfs/locks/{id}/unlock",
                    web::post().to(lfs::handle_delete_lock)
                );
        }
        
        // 添加 Docker Registry 路由（如果启用）
        // 使用 Host guard 确保只响应 registry_domain 的请求
        if let Some(ref docker_data) = docker_registry_state {
            let registry_domain = config_data.registry_domain.clone().unwrap();
            app = app
                .app_data(docker_data.clone())
                .service(
                    web::scope("/v2")
                        .guard(guard::Host(registry_domain))
                        // Docker Registry V2 API
                        .route("/", web::get().to(registry::handle_v2_check))
                        .route("", web::get().to(registry::handle_v2_check))
                        .route("/auth", web::get().to(registry::handle_token_auth))
                        .route("/_catalog", web::get().to(registry::handle_catalog))
                        // Blob 操作
                        .route("/{name:.*}/blobs/{digest}", web::head().to(registry::handle_blob_head))
                        .route("/{name:.*}/blobs/{digest}", web::get().to(registry::handle_blob_get))
                        .route("/{name:.*}/blobs/{digest}", web::delete().to(registry::handle_blob_delete))
                        // Blob 上传
                        .service(
                            web::resource("/{name:.*}/blobs/uploads/")
                                .app_data(web::PayloadConfig::new(registry_max_upload_size))
                                .route(web::post().to(registry::handle_blob_upload_start))
                        )
                        .service(
                            web::resource("/{name:.*}/blobs/uploads/{uuid}")
                                .app_data(web::PayloadConfig::new(registry_max_upload_size))
                                .route(web::patch().to(registry::handle_blob_upload_patch))
                                .route(web::put().to(registry::handle_blob_upload_put))
                                .route(web::delete().to(registry::handle_blob_upload_delete))
                        )
                        // Manifest 操作
                        .route("/{name:.*}/manifests/{reference}", web::head().to(registry::handle_manifest_head))
                        .route("/{name:.*}/manifests/{reference}", web::get().to(registry::handle_manifest_get))
                        .service(
                            web::resource("/{name:.*}/manifests/{reference}")
                                .app_data(web::PayloadConfig::new(registry_max_upload_size))
                                .route(web::put().to(registry::handle_manifest_put))
                                .route(web::delete().to(registry::handle_manifest_delete))
                        )
                        // Tags 列表
                        .route("/{name:.*}/tags/list", web::get().to(registry::handle_tags_list))
                );
        }
        
        // 添加 npm Registry 路由（如果启用）
        // 使用 Host guard 确保只响应 registry_domain 的请求
        if let Some(ref npm_data) = npm_registry_state {
            let registry_domain = config_data.registry_domain.clone().unwrap();
            app = app
                .app_data(npm_data.clone())
                .service(
                    web::scope("/npm")
                        .guard(guard::Host(registry_domain))
                        // npm Registry API
                        .route("/-/ping", web::get().to(registry::handle_ping))
                        .route("/-/whoami", web::get().to(registry::handle_whoami))
                        .route("/-/user/org.couchdb.user:{username}", web::put().to(registry::handle_user_login))
                        .route("/-/v1/search", web::get().to(registry::handle_search))
                        // dist-tags
                        .route("/-/package/@{scope}/{name}/dist-tags", web::get().to(registry::handle_dist_tags_get))
                        .route("/-/package/@{scope}/{name}/dist-tags/{tag}", web::get().to(registry::handle_dist_tags_get))
                        .route("/-/package/@{scope}/{name}/dist-tags/{tag}", web::put().to(registry::handle_dist_tag_put))
                        .route("/-/package/@{scope}/{name}/dist-tags/{tag}", web::delete().to(registry::handle_dist_tag_delete))
                        // Scoped 包
                        .route("/@{scope}/{name}", web::get().to(registry::handle_package_get_scoped))
                        .service(
                            web::resource("/@{scope}/{name}")
                                .app_data(web::PayloadConfig::new(registry_max_upload_size))
                                .route(web::put().to(registry::handle_package_publish_scoped))
                        )
                        .route("/@{scope}/{name}/-/{tarball}", web::get().to(registry::handle_tarball_get_scoped))
                        .route("/@{scope}/{name}/-/{tarball}/-rev/{rev}", web::delete().to(registry::handle_tarball_delete_scoped))
                        // 非 scoped 包（返回错误）
                        .route("/{name}", web::get().to(registry::handle_package_get))
                );
        }

        // 添加 Cargo Registry 路由（如果启用）
        // 使用 Host guard 确保只响应 registry_domain 的请求
        if let Some(ref cargo_data) = cargo_registry_state {
            let registry_domain = config_data.registry_domain.clone().unwrap();
            app = app
                .app_data(cargo_data.clone())
                .service(
                    web::scope("/cargo")
                        .guard(guard::Host(registry_domain))
                        // Cargo Sparse Index API
                        .route("/{namespace}/index/config.json", web::get().to(registry::cargo::handle_config))
                        // Index 条目路径遵循 crate 名称长度规则
                        // 1 字符: /1/{name}
                        // 2 字符: /2/{name}
                        // 3 字符: /3/{first_char}/{name}
                        // 4+ 字符: /{first_two}/{next_two}/{name}
                        .route("/{namespace}/index/1/{name}", web::get().to(registry::cargo::handle_index_entry))
                        .route("/{namespace}/index/2/{name}", web::get().to(registry::cargo::handle_index_entry))
                        .route("/{namespace}/index/3/{prefix}/{name}", web::get().to(registry::cargo::handle_index_entry))
                        .route("/{namespace}/index/{prefix1}/{prefix2}/{name}", web::get().to(registry::cargo::handle_index_entry))
                        // Cargo Web API
                        .service(
                            web::resource("/{namespace}/api/v1/crates/new")
                                .app_data(web::PayloadConfig::new(registry_max_upload_size))
                                .route(web::put().to(registry::cargo::handle_publish))
                        )
                        .route("/{namespace}/api/v1/crates/{name}/{version}/yank", web::delete().to(registry::cargo::handle_yank))
                        .route("/{namespace}/api/v1/crates/{name}/{version}/unyank", web::put().to(registry::cargo::handle_unyank))
                        .route("/{namespace}/api/v1/crates/{name}/owners", web::get().to(registry::cargo::handle_owners_list))
                        .route("/{namespace}/api/v1/crates/{name}/owners", web::put().to(registry::cargo::handle_owners_add))
                        .route("/{namespace}/api/v1/crates/{name}/owners", web::delete().to(registry::cargo::handle_owners_remove))
                        .route("/{namespace}/api/v1/crates/{name}/{version}/download", web::get().to(registry::cargo::handle_download))
                        .route("/{namespace}/api/v1/crates/{name}", web::get().to(registry::cargo::handle_crate_info))
                        .route("/{namespace}/api/v1/crates", web::get().to(registry::cargo::handle_search))
                        // Cargo 登录
                        .route("/{namespace}/me", web::get().to(registry::cargo::handle_login))
                );
        }

        // Registry domain catch-all 404
        // 对于 registry_domain 的任何未匹配路径返回 404，防止 fallback 到主应用路由
        if config_data.registry_enabled {
            let registry_domain = config_data.registry_domain.clone().unwrap();
            app = app.service(
                web::scope("")
                    .guard(guard::Host(registry_domain))
                    .default_service(web::to(|| async {
                        actix_web::HttpResponse::NotFound()
                            .content_type("application/json")
                            .body(r#"{"error": "Not Found"}"#)
                    }))
            );
        }
            
        // Git HTTP 协议 - 直接通过 GitLayer gRPC 处理（流式传输）
        // 支持大仓库 push/clone，不再代理到 Main App
        if config_data.gitlayer_address.is_some() {
            let git_state = web::Data::new(git_http::GitHttpState::new(
                std::sync::Arc::new(config_data.get_ref().clone()),
                auth_client.clone(),
            ));
            app = app
                .app_data(git_state.clone())
                // PayloadConfig for large push
                .app_data(web::PayloadConfig::new(max_upload_size))
                // Git HTTP 路由（使用原始字符串避免转义问题）
                .route(
                    r"/{tail:.*\.git/info/refs}",
                    web::get().to(git_http::handle_info_refs),
                )
                .route(
                    r"/{tail:.*\.git/git-upload-pack}",
                    web::post().to(git_http::handle_upload_pack),
                )
                .route(
                    r"/{tail:.*\.git/git-receive-pack}",
                    web::post().to(git_http::handle_receive_pack),
                );
            tracing::info!("Git HTTP: direct GitLayer mode (streaming)");
        } else {
            // GitLayer 未配置，Git HTTP 请求将失败
            tracing::warn!("GITLAYER_ADDRESS not configured, Git HTTP will be unavailable");
            app = app.service(
                web::resource(r#"/{namespace}/{project:.*\.git.*}"#)
                    .route(web::route().to(|| async {
                        actix_web::HttpResponse::ServiceUnavailable()
                            .content_type("text/plain")
                            .body("GitLayer not configured. Git operations unavailable.")
                    }))
            );
        }
            
        // 前端 SPA 文件服务
        // 必须放在最后作为 catch-all 路由，匹配所有未被上面路由处理的请求
        // default_handler 用于处理前端路由（如 /projects）- 返回 index.html 让前端路由接管
        app.service(
            Files::new("/", frontend_dist_data.get_ref().clone())
                .index_file("index.html")
                .use_last_modified(true)
                .use_etag(true)
                .prefer_utf8(true)
                .default_handler({
                    let frontend_dist = frontend_dist_data.clone();
                    web::to(move |req: HttpRequest| {
                        let dist = frontend_dist.clone();
                        async move { static_files::serve_spa_index(req, dist).await }
                    })
                })
        )
    })
    .bind((listen_addr.as_str(), listen_port))?
    .workers(num_cpus::get())
    .run()
    .await
}
