use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use crate::state::AppState;

#[derive(Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, String)> {
    state.s3_client.delete_object()
        .bucket(&state.s3_bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DeleteResponse {
        success: true,
        message: format!("File {} deleted", key),
    }))
}
