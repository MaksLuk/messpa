mod config;
mod state;
mod handlers;
mod utils;

use axum::{
    routing::{post, delete},
    Router,
    extract::DefaultBodyLimit,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("file_storage_service=debug".parse().unwrap())
        )
        .init();

    let config = config::Config::from_env().expect("Failed to load config from env");
    let state = Arc::new(state::AppState::new(&config).await);

    let app = Router::new()
        .route("/upload/image", post(handlers::upload::upload_image))
        .route("/upload/video", post(handlers::upload::upload_video))
        .route("/upload/file", post(handlers::upload::upload_file))
        .route("/files/{*key}", delete(handlers::delete::delete_file))
        .layer(DefaultBodyLimit::max(
            config.max_body_size as usize * 1024 * 1024 * 1024) // глобальный лимит
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = if config.test {
        SocketAddr::from(([0, 0, 0, 0], config.server_port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], config.server_port))
    };
    tracing::info!("File storage service starting on port {}", config.server_port);
    axum::serve(tokio::net::TcpListener::bind(&addr).await.unwrap(), app).await.unwrap();
}
