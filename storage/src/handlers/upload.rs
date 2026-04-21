use axum::{
    extract::{Multipart, Query, State},
    http::StatusCode,
    Json,
};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::state::AppState;
use crate::utils::{
    mime::{is_allowed_image_mime, is_allowed_video_mime},
    clamav::scan_with_clamav,
};

#[derive(Deserialize)]
pub struct UploadQuery {
    /// Файл хранится бесконечно или 180 дней
    #[serde(default)]
    pub permanent: Option<bool>,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub link: String,
    pub key: String,
}

pub async fn upload_image(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UploadQuery>,
    multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    upload_handler(state, multipart, 10 * 1024 * 1024, true, query.permanent.unwrap_or(false), "images/").await
}

pub async fn upload_video(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    upload_handler(state, multipart, 100 * 1024 * 1024, true, false, "videos/").await
}

pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UploadQuery>, // compress игнорируется — всегда 180 дней
    multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    // Для общего файла всегда temporary (180 дней), сжатие на выбор
    let compress = query.permanent.unwrap_or(true); // переиспользуем поле как "compress" для удобства
    upload_handler(state, multipart, 8 * 1024 * 1024 * 1024, compress, false, "files/").await
}

// Общий обработчик
async fn upload_handler(
    state: Arc<AppState>,
    mut multipart: Multipart,
    max_size: u64,
    always_compress: bool,
    permanent: bool,
    prefix: &str,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    let field = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
        .ok_or((StatusCode::BAD_REQUEST, "No file field".to_string()))?;

    let original_name = field.file_name().unwrap_or("file").to_string();
    let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();

    if prefix == "images/" && !is_allowed_image_mime(&content_type) {
        return Err((StatusCode::BAD_REQUEST, "Unsupported MIME type".to_string()));
    }
    else if prefix == "videos/" && !is_allowed_video_mime(&content_type) {
        return Err((StatusCode::BAD_REQUEST, "Unsupported MIME type".to_string()));
    }

    let temp_dir = tempfile::tempdir().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let temp_path = temp_dir.path().join(&original_name);

    let mut file = tokio::fs::File::create(&temp_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut bytes_read: u64 = 0;
    let mut stream = field.into_stream();

    while let Some(chunk) = stream.try_next().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))? {
        let len = chunk.len() as u64;
        if bytes_read + len > max_size {
            return Err((StatusCode::PAYLOAD_TOO_LARGE, format!("File too large. Max: {} bytes", max_size)));
        }
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        bytes_read += len;
    }

    // Антивирус
    if !scan_with_clamav(&temp_path, &state.clamav_host, state.clamav_port).await? {
        return Err((StatusCode::BAD_REQUEST, "File contains virus".to_string()));
    }

    // Сжатие (для image и video — всегда, для file — по выбору)
    let should_compress = if prefix == "files/" { always_compress } else { true };
    let (upload_path, content_encoding, final_mime) = if should_compress {
        // gzip сжатие
        let compressed = crate::utils::compress::gzip_compress_file(&temp_path)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        let gz_path = temp_dir.path().join(format!("{}.gz", original_name));
        tokio::fs::write(&gz_path, compressed).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        (gz_path, Some("gzip".to_string()), "application/gzip".to_string())
    } else {
        (temp_path.clone(), None, content_type.clone())
    };

    // Генерация ключа
    let ext = std::path::Path::new(&original_name)
        .extension().and_then(|e| e.to_str()).unwrap_or("");
    let uuid = uuid::Uuid::new_v4();
    let key = if permanent {
        format!("{prefix}permanent/{uuid}.{ext}")
    } else {
        format!("{prefix}temporary/{uuid}.{ext}")
    };
    let key = if should_compress { format!("{}.gz", key) } else { key };

    // Загрузка в S3
    let body = aws_sdk_s3::primitives::ByteStream::from_path(&upload_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    state.s3_client.put_object()
        .bucket(&state.s3_bucket)
        .key(&key)
        .body(body)
        .content_type(final_mime)
        .set_content_encoding(content_encoding)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let link = format!("{}/{}/{}", state.s3_public_url.trim_end_matches('/'), state.s3_bucket, key);

    Ok(Json(UploadResponse {
        success: true,
        link,
        key,
    }))
}
