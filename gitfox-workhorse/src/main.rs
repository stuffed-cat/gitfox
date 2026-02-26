mod config;
mod proxy;
mod static_files;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
    middleware::{Compress, Logger},
    web, App, HttpServer,
};
use config::Config;
use reqwest::Client;
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

    // 加载配置
    let config = if let Ok(config_path) = std::env::var("WORKHORSE_CONFIG") {
        tracing::info!("Loading config from file: {}", config_path);
        Config::from_file(&config_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load config file: {}, using env vars", e);
            Config::from_env()
        })
    } else {
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
    tracing::info!("Backend URL: {}", config.backend_url);
    tracing::info!("Frontend dist: {:?}", config.frontend_dist_path);
    tracing::info!("WebIDE dist: {:?}", config.webide_dist_path);
    tracing::info!("Assets path: {:?}", config.assets_path);
    tracing::info!("Git repos path: {:?}", config.git_repos_path);

    // 创建 HTTP 客户端（用于代理请求）
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .expect("Failed to create HTTP client");

    let client_data = web::Data::new(client);
    let backend_url_data = web::Data::new(config.backend_url.clone());
    let frontend_dist_data = web::Data::new(config.frontend_dist_path.clone());
    let webide_dist_data = web::Data::new(config.webide_dist_path.clone());
    let assets_path_data = web::Data::new(config.assets_path.clone());

    let listen_addr = config.listen_addr.clone();
    let listen_port = config.listen_port;

    // 启动 HTTP 服务器
    HttpServer::new(move || {
        App::new()
            .app_data(client_data.clone())
            .app_data(backend_url_data.clone())
            .app_data(frontend_dist_data.clone())
            .app_data(webide_dist_data.clone())
            .app_data(assets_path_data.clone())
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
            
            // Assets 静态文件（用户上传的头像等）
            .route("/assets/{tail:.*}", web::get().to(static_files::serve_asset))
            
            // WebIDE 静态文件
            .service(
                Files::new("/-/ide", webide_dist_data.get_ref().clone())
                    .index_file("index.html")
                    .use_last_modified(true)
                    .use_etag(true)
                    .prefer_utf8(true)
                    .default_handler(web::to(static_files::webide_fallback))
            )
            
            // API 代理到后端
            .service(
                web::scope("/api")
                    .default_service(web::to(proxy::proxy_to_backend))
            )
            
            // OAuth 端点代理
            .service(
                web::scope("/oauth")
                    .default_service(web::to(proxy::proxy_to_backend))
            )
            
            // Git HTTP 协议代理
            // 匹配 /namespace/project.git/* 路径
            .route(
                r#"/{namespace}/{project:.*\.git.*}"#,
                web::route().to(proxy::proxy_git_http)
            )
            
            // 前端 SPA 静态文件（包括 fallback）
            // 必须放在最后，作为 catch-all
            .service(
                Files::new("/", frontend_dist_data.get_ref().clone())
                    .index_file("index.html")
                    .use_last_modified(true)
                    .use_etag(true)
                    .prefer_utf8(true)
                    .default_handler(web::to(static_files::spa_fallback))
            )
    })
    .bind((listen_addr.as_str(), listen_port))?
    .workers(num_cpus::get())
    .run()
    .await
}
