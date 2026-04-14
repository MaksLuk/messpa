use diesel::r2d2::{self, ConnectionManager, Pool};
use diesel::PgConnection;
use crate::config::Config;
use redis::aio::ConnectionManager as RedisConnectionManager;

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub config: Config,
    pub redis_conn: RedisConnectionManager,
}

impl AppState {
    pub async fn new(db_pool: DbPool, config: Config) -> Self {
        let redis_client = redis::Client::open(config.redis_url.clone())
            .expect("Invalid Redis URL");
        let redis_conn = redis_client.get_connection_manager().await
            .expect("Failed to create Redis connection manager");

        Self { db_pool, config, redis_conn }
    }
}

pub fn create_db_pool(database_url: &str) -> DbPool {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create DB pool")
}
