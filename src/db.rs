use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn init_pg_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub fn init_redis_pool(redis_url: &str) -> Result<RedisPool, deadpool_redis::CreatePoolError> {
    let cfg = RedisConfig::from_url(redis_url);
    cfg.create_pool(Some(Runtime::Tokio1))
}
