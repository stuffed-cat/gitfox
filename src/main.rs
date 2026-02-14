use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use std::sync::Arc;

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

    // Run database migrations
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .expect("Failed to run migrations");

    // Seed initial admin user if configured and no admin exists
    if let Err(e) = services::UserService::seed_initial_admin(&pg_pool, &config).await {
        log::error!("Failed to seed initial admin: {}", e);
    }

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

    log::info!("Starting GitFox HTTP server at {}", server_addr);

    HttpServer::new(move || {
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
            .configure(handlers::configure_routes)
    })
    .bind(&server_addr)?
    .run()
    .await
}
