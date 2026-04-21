use flate2::{write::GzEncoder, Compression};
use std::io::Write;
use std::path::Path;
use tokio::fs;

/// Сжимает файл в gzip и возвращает Vec<u8> со сжатыми данными
pub async fn gzip_compress_file(path: &Path) -> Result<Vec<u8>, String> {
    let data = fs::read(path)
        .await
        .map_err(|e| format!("Failed to read file for compression: {}", e))?;

    tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default()); // default = level 6
        encoder.write_all(&data)
            .map_err(|e| format!("Gzip compression failed: {}", e))?;
        encoder.finish()
            .map_err(|e| format!("Failed to finish gzip: {}", e))
    })
    .await
    .map_err(|e| format!("Blocking task failed: {}", e))?
}
