use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),

    #[error("Connection pool error: {0}")]
    Pool(#[from] r2d2::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Not found")]
    NotFound,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal server error")]
    Internal,

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Refresh token reuse detected - all sessions revoked")]
    RefreshReuseDetected,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Auth(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::RateLimit => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(json!({
            "ok": false,
            "data": null,
            "meta": { "request_id": "TODO" },
            "error": {
                "code": format!("{:?}", self),
                "message": message
            }
        }));

        (status, body).into_response()
    }
}
