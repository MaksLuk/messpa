use axum::{
    extract::{Multipart, Query, State},
    http::StatusCode,
    Json,
};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::state::AppState;
use crate::utils::{
    compress::gzip_compress_file,
    mime::{is_allowed_image_mime, is_allowed_video_mime},
    clamav::scan_with_clamav,
};

#[derive(Deserialize, Default)]
pub struct UploadQuery {
    #[serde(default)]
    pub permanent: Option<bool>,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub success: bool,
    pub link: String,
    pub key: String,
}

// ===================== ХЕНДЛЕРЫ =====================

pub async fn upload_image(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UploadQuery>,
    multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    let max_size = (state.max_image_size as u64) * 1024 * 1024;
    upload_handler(
        state,
        multipart,
        max_size,
        true,                                 // always compress
        query.permanent.unwrap_or(false),
        "images/",
    ).await
}

pub async fn upload_video(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    let max_size = (state.max_video_size as u64) * 1024 * 1024;
    upload_handler(
        state,
        multipart,
        max_size,
        true,
        false,
        "videos/",
    ).await
}

pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UploadQuery>,
    multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    let compress = query.permanent.unwrap_or(true); // true = compress
    let max_size = (state.max_file_size as u64) * 1024 * 1024 * 1024;
    upload_handler(
        state,
        multipart,
        max_size,
        compress,
        false,
        "files/",
    ).await
}

// ===================== ОСНОВНОЙ ОБРАБОТЧИК =====================

async fn upload_handler(
    state: Arc<AppState>,
    mut multipart: Multipart,
    max_size: u64,
    always_compress: bool,
    permanent: bool,
    prefix: &str,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    info!("Upload started | prefix: {} | max_size: {} bytes | compress: {}", 
          prefix, max_size, always_compress);

    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => return Err((StatusCode::BAD_REQUEST, "No file field found".into())),
        Err(e) => {
            error!("Multipart parsing error: {}", e);
            return Err((StatusCode::BAD_REQUEST, e.to_string()));
        }
    };

    let original_name = field.file_name().unwrap_or("file").to_string();
    let content_type = field.content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    info!("File: {} | MIME: {}", original_name, content_type);

    // Проверка MIME
    let mime_ok = match prefix {
        "images/" => is_allowed_image_mime(&content_type),
        "videos/" => is_allowed_video_mime(&content_type),
        _ => true, // для файлов — всё разрешаем
    };

    if !mime_ok {
        warn!("Unsupported MIME: {}", content_type);
        return Err((StatusCode::BAD_REQUEST, format!("Unsupported MIME type: {}", content_type)));
    }

    // Сохраняем во временный файл
    let temp_dir = tempfile::tempdir()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("tempdir: {}", e)))?;

    let temp_path = temp_dir.path().join(&original_name);

    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("create file: {}", e)))?;

    let mut bytes_read: u64 = 0;
    let mut stream = field.into_stream();

    while let Some(chunk) = stream.try_next().await.map_err(|e| {
        error!("Chunk read error: {}", e);
        (StatusCode::BAD_REQUEST, e.to_string())
    })? {
        let len = chunk.len() as u64;
        if bytes_read + len > max_size {
            return Err((StatusCode::PAYLOAD_TOO_LARGE, format!("Max size exceeded: {} bytes", max_size)));
        }
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("write error: {}", e)))?;
        bytes_read += len;
    }

    info!("File saved temporarily: {} bytes", bytes_read);

    // === Антивирус ===
    match scan_with_clamav(&temp_path, &state.clamav_host, state.clamav_port).await {
        Ok(true) => info!("ClamAV: clean"),
        Ok(false) => return Err((StatusCode::BAD_REQUEST, "File contains virus".into())),
        Err((_, e)) => {
            error!("ClamAV scan failed: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Antivirus error: {}", e)));
        }
    }

    // === Сжатие ===
    let should_compress = if prefix == "files/" { always_compress } else { true };
    let (upload_path, content_encoding, final_mime) = if should_compress {
        let compressed = gzip_compress_file(&temp_path)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Compression failed: {}", e)))?;

        let gz_path = temp_dir.path().join(format!("{}.gz", original_name));
        tokio::fs::write(&gz_path, &compressed)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("write gz: {}", e)))?;

        (gz_path, Some("gzip".to_string()), "application/gzip".to_string())
    } else {
        (temp_path.clone(), None, content_type.clone())
    };

    // === Ключ в S3 ===
    let ext = std::path::Path::new(&original_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let uuid = uuid::Uuid::new_v4();
    let base_key = if permanent {
        format!("{prefix}permanent/{uuid}.{ext}")
    } else {
        format!("{prefix}temporary/{uuid}.{ext}")
    };
    let key = if should_compress {
        format!("{}.gz", base_key)
    } else {
        base_key
    };

    // === Загрузка в MinIO ===
    let body = aws_sdk_s3::primitives::ByteStream::from_path(&upload_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("ByteStream: {}", e)))?;

    state.s3_client
        .put_object()
        .bucket(&state.s3_bucket)
        .key(&key)
        .body(body)
        .content_type(final_mime)
        .set_content_encoding(content_encoding)
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("S3 upload failed: {}", e)))?;

    let link = format!(
        "{}/{}/{}",
        state.s3_public_url.trim_end_matches('/'),
        state.s3_bucket,
        key
    );

    info!("Upload successful! Key: {}", key);

    Ok(Json(UploadResponse {
        success: true,
        link,
        key,
    }))
}
