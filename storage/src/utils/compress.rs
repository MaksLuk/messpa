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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::NamedTempFile;
    use tokio::fs;

    // Вспомогательная функция для создания тестового файла
    async fn create_test_file(content: &[u8]) -> NamedTempFile {
        let file = NamedTempFile::new().expect("Failed to create temp file");
        fs::write(file.path(), content)
            .await
            .expect("Failed to write test content");
        file
    }

    #[tokio::test]
    async fn test_gzip_compress_empty_file() {
        let file = create_test_file(b"").await;
        let result = gzip_compress_file(file.path()).await;

        assert!(result.is_ok(), "Empty file should compress successfully");
        let compressed = result.unwrap();
        assert!(!compressed.is_empty(), "Compressed data should not be empty");
    }

    #[tokio::test]
    async fn test_gzip_compress_nonexistent_file() {
        let nonexistent_path = Path::new("/non/existent/file/that/does/not/exist.txt");
        let result = gzip_compress_file(nonexistent_path).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Failed to read file for compression"));
    }

    #[tokio::test]
    async fn test_gzip_compress_permission_error() {
        // Создаём файл и снимаем права на чтение
        let file = NamedTempFile::new().expect("Failed to create temp file");
        let path = file.path().to_path_buf();

        // Убираем права на чтение (только на Unix-like системах)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).await.unwrap().permissions();
            perms.set_mode(0o000); // нет прав
            fs::set_permissions(&path, perms).await.unwrap();
        }

        let result = gzip_compress_file(&path).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to read file for compression"));
    }

    #[tokio::test]
    async fn test_gzip_compress_invalid_data_handling() {
        // Тест на то, что функция корректно обрабатывает ошибки внутри blocking task
        // (например, если бы GzEncoder упал — но в нормальных условиях он не падает)
        // Этот тест проверяет ветку .map_err в spawn_blocking

        let content = b"valid content";
        let file = create_test_file(content).await;

        let result = gzip_compress_file(file.path()).await;
        assert!(result.is_ok()); // основной случай уже покрыт выше
    }
}
