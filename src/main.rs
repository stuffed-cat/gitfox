use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use actix_files as fs;
use dotenv::dotenv;
use std::sync::Arc;
use tokio::sync::RwLock;

mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod queue;
mod services;
mod ssh;

use config::AppConfig;
use db::{init_pg_pool, init_redis_pool};
use queue::RedisMessageQueue;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let config = AppConfig::from_env();
    let server_addr = format!("{}:{}", config.server_host, config.server_port);

    // Initialize database pools
    let pg_pool = init_pg_pool(&config.database_url)
        .await
        .expect("Failed to create PostgreSQL pool");

    let redis_pool = init_redis_pool(&config.redis_url)
        .expect("Failed to create Redis pool");

    // Initialize message queue
    let message_queue = RedisMessageQueue::new(redis_pool.clone());

    // Initialize CI/CD Runner Manager
    let runner_manager = Arc::new(RwLock::new(handlers::runner::RunnerManager::new()));

    // Run database migrations
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .expect("Failed to run migrations");

    // Seed initial admin user if configured and no admin exists
    if let Err(e) = services::UserService::seed_initial_admin(&pg_pool, &config).await {
        log::error!("Failed to seed initial admin: {}", e);
    }

    // Auto-configure WebIDE OAuth2 application if enabled
    if let Err(e) = services::OAuthService::auto_configure_webide_oauth(&pg_pool, &config).await {
        log::error!("Failed to auto-configure WebIDE OAuth2: {}", e);
    }

    // Start instance heartbeat in Redis (for multi-instance coordination)
    let heartbeat_redis = redis_pool.clone();
    let heartbeat_instance_id = config.instance_id.clone();
    tokio::spawn(async move {
        handlers::runner::start_instance_heartbeat(heartbeat_redis, heartbeat_instance_id).await;
    });
    log::info!("Instance heartbeat started (Redis-based)");

    // Start PostgreSQL LISTEN for job timeout notifications (failsafe from database trigger)
    let pg_listener_pool = pg_pool.clone();
    let pg_listener_instance_id = config.instance_id.clone();
    tokio::spawn(async move {
        handlers::runner::start_pg_timeout_listener(pg_listener_pool, pg_listener_instance_id).await;
    });
    log::info!("PostgreSQL job timeout listener started (failsafe)");

    // Start Redis-based job timeout listener in background (primary timeout mechanism)
    let timeout_pool = pg_pool.clone();
    let redis_url = config.redis_url.clone();
    let instance_id = config.instance_id.clone();
    tokio::spawn(async move {
        handlers::runner::start_redis_timeout_listener(timeout_pool, redis_url, instance_id).await;
    });
    log::info!("Redis job timeout listener started (primary)");

    // Start SSH server if enabled
    if config.ssh_enabled {
        let ssh_config = Arc::new(config.clone());
        let ssh_pool = Arc::new(pg_pool.clone());
        
        log::info!(
            "Starting GitFox SSH server on {}:{}",
            config.ssh_host,
            config.ssh_port
        );
        
        ssh::start_ssh_server(ssh_config, ssh_pool);
    }

    let runner_manager_clone = runner_manager.clone();
    
    // 提前提取 socket_path 以避免借用问题
    let socket_path = config.server_socket_path.clone();

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(web::Data::new(pg_pool.clone()))
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(message_queue.clone()))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(runner_manager_clone.clone()))
            .service(fs::Files::new("/assets", config.assets_path.clone()).show_files_listing())
            .configure(handlers::configure_routes)
    });

    // Unix Socket 优先于 TCP
    let server = if let Some(socket_path) = socket_path {
        log::info!("Starting GitFox HTTP server on Unix socket: {}", socket_path);
        
        // 删除旧的 socket 文件（如果存在）
        if std::path::Path::new(&socket_path).exists() {
            std::fs::remove_file(&socket_path)?;
        }
        
        server.bind_uds(socket_path)?
    } else {
        log::info!("Starting GitFox HTTP server at {}", server_addr);
        server.bind(&server_addr)?
    };

    server.run().await
}
