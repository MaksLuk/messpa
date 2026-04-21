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
    tracing_subscriber::fmt::init();

    let config = config::Config::from_env().expect("Failed to load config from env");
    let state = Arc::new(state::AppState::new(&config).await);

    let app = Router::new()
        .route("/upload/image", post(handlers::upload::upload_image))
        .route("/upload/video", post(handlers::upload::upload_video))
        .route("/upload/file", post(handlers::upload::upload_file))
        .route("/files/:key", delete(handlers::delete::delete_file))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024 * 1024)) // глобальный лимит 10 ГБ
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    println!("File storage service running on http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(&addr).await.unwrap(), app).await.unwrap();
}
