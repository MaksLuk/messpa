use diesel::r2d2::{self, ConnectionManager, Pool};
use diesel::PgConnection;
use redis::aio::ConnectionManager as RedisConnectionManager;

use std::time::Duration;

use crate::config::Config;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub config: Config,
    pub redis_conn: RedisConnectionManager,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub async fn new(db_pool: DbPool, config: Config) -> Self {
        let redis_client = redis::Client::open(config.redis_url.clone())
            .expect("Invalid Redis URL");
        let redis_conn = redis_client.get_connection_manager().await
            .expect("Failed to create Redis connection manager");
        let http_client = reqwest::Client::builder()
                .timeout(Duration::from_secs(90))
                .connect_timeout(Duration::from_secs(15))
                .build()
                .expect("Failed to create HTTP client");

        Self { db_pool, config, redis_conn, http_client }
    }
}

pub fn create_db_pool(database_url: &str) -> DbPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create DB pool")
}
