use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::net::SocketAddr;
use std::sync::Arc;

mod config;
mod state;
mod error;
mod routes;
mod models;
mod schema;
//mod domain;
//mod repositories;
//mod services;
mod handlers;
mod api_response;
mod middleware;
mod metrics;
mod openapi;
mod utils;
mod bot;

#[macro_use]
extern crate diesel_migrations;

pub(crate) const MIGRATIONS: diesel_migrations::EmbeddedMigrations =
    embed_migrations!("./migrations");

use diesel::{Connection, PgConnection};
use diesel_migrations::MigrationHarness;

#[tokio::main]
async fn main() {
    // Инициализация логирования
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = config::Config::from_env().expect("Failed to load config");

    // Создание таблиц БД
    let mut conn = PgConnection::establish(&cfg.database_url).unwrap();
    conn.run_pending_migrations(crate::MIGRATIONS).unwrap();

    // Пул соединений с PostgreSQL
    let db_pool = state::create_db_pool(&cfg.database_url);

    let app_state = Arc::new(state::AppState::new(db_pool, cfg.clone()).await);

    // Запуск телеграм-бота
    let bot_state = app_state.clone();
    tokio::spawn(async move {
        println!("Telegram бот запущен");
        bot::run_telegram_bot(bot_state).await;
    });
    
    let metrics_handle = metrics::setup_metrics();

    let app = Router::new()
        .merge(routes::api_routes(app_state.clone()))
        .route("/metrics", axum::routing::get(|| async move {
            axum::response::Html(metrics_handle.render())
        }))
        .layer(axum::middleware::from_fn(metrics::track_metrics))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = format!("0.0.0.0:{}", cfg.server_port);
    println!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener, app.into_make_service_with_connect_info::<SocketAddr>()
    ).await.unwrap();
}
