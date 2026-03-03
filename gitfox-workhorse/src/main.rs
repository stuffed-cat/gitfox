mod auth_client;
mod config;
mod gitlayer_client;
mod http_client;
mod lfs;
mod proxy;
mod static_files;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
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

    let client_data = web::Data::new(backend_client);
    let backend_url_data = web::Data::new(config.backend_url.clone());
    let backend_socket_data = web::Data::new(config.backend_socket.clone());
    let frontend_dist_data = web::Data::new(config.frontend_dist_path.clone());
    let webide_dist_data = web::Data::new(config.webide_dist_path.clone());
    let assets_path_data = web::Data::new(config.assets_path.clone());

    // 初始化 LFS 状态
    let lfs_state = if config.lfs_enabled {
        tracing::info!("LFS enabled, storage path: {:?}", config.lfs_storage_path);
        let state = lfs::LfsState::new(std::sync::Arc::new(config.clone()));
        if let Err(e) = state.init().await {
            tracing::error!("Failed to initialize LFS storage: {}", e);
            std::process::exit(1);
        }
        Some(web::Data::new(state))
    } else {
        tracing::info!("LFS disabled");
        None
    };

    let listen_addr = config.listen_addr.clone();
    let listen_port = config.listen_port;
    let max_upload_size = config.max_upload_size;
    let lfs_max_upload_size = config.lfs_max_object_size as usize;

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
            
        // Git HTTP 协议代理
        // 匹配 /namespace/project.git/* 路径
        app = app.service(
            web::resource(r#"/{namespace}/{project:.*\.git.*}"#)
                // Git push 操作需要更大的上传限制
                .app_data(web::PayloadConfig::new(max_upload_size))
                .route(web::route().to(proxy::proxy_git_http))
        );
            
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
