use axum::http::StatusCode;
use tokio::{
    net::TcpStream,
    io::{AsyncWriteExt, AsyncReadExt},
};

use std::path::Path;

pub async fn scan_with_clamav(path: &Path, host: &str, port: u16) -> Result<bool, (StatusCode, String)> {
    let mut stream = TcpStream::connect(format!("{}:{}", host, port))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    stream.write_all(b"zINSTREAM\0").await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if n == 0 { break; }

        let size_bytes = (n as u32).to_be_bytes();
        stream.write_all(&size_bytes).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        stream.write_all(&buffer[0..n]).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    stream.write_all(&0u32.to_be_bytes()).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut response = String::new();
    let mut buf = [0u8; 1024];
    loop {
        let n = stream.read(&mut buf).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if n == 0 { break; }
        response.push_str(&String::from_utf8_lossy(&buf[0..n]));
        if response.contains("FOUND") || response.contains("OK") {
            break;
        }
    }

    Ok(!response.contains("FOUND"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        net::TcpListener,
        io::{AsyncReadExt, AsyncWriteExt},
    };
    use tempfile::NamedTempFile;
    use std::io::Write;
    use std::path::PathBuf;

    // Сервер принимает уже созданный listener (из теста) и отвечает заданной строкой.
    async fn run_mock_clamd(listener: TcpListener, response: &'static str) {
        if let Ok((mut socket, _)) = listener.accept().await {
            // прочитать точно 9 байт команды "zINSTREAM\0"
            let mut hello = [0u8; 10];
            if socket.read_exact(&mut hello).await.is_err() {
                return;
            }
            if &hello != b"zINSTREAM\0" {
                // неверная команда — просто закрыть
                return;
            }

            // читать пакеты: 4 байта размера big-endian, потом payload
            loop {
                let mut size_buf = [0u8; 4];
                if socket.read_exact(&mut size_buf).await.is_err() {
                    return;
                }
                let size = u32::from_be_bytes(size_buf);
                if size == 0 {
                    break;
                }
                let mut chunk = vec![0u8; size as usize];
                if socket.read_exact(&mut chunk).await.is_err() {
                    return;
                }
                // игнорируем содержимое
            }

            let _ = socket.write_all(response.as_bytes()).await;
            // дать клиенту время прочитать, затем закрыть (drop)
        }
    }

    async fn write_temp_file(contents: &[u8]) -> (NamedTempFile, PathBuf) {
        let mut tmp = NamedTempFile::new().expect("failed to create temp file");
        tmp.write_all(contents).expect("failed to write to temp file");
        tmp.flush().expect("flush failed");
        tmp.as_file_mut().sync_all().expect("sync_all failed");

        let path = tmp.path().to_path_buf();
        (tmp, path)  // NamedTempFile остаётся владельцем файла
    }

    #[tokio::test]
    async fn test_scan_infected() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        let response = "stream: Eicar-Test-Signature FOUND\n";
        let srv = tokio::spawn(async move {
            run_mock_clamd(listener, response).await;
        });

        let (tmp_file, path) = write_temp_file(b"EICAR-TEST-VECTOR").await;

        let res = scan_with_clamav(&path, "127.0.0.1", addr.port()).await;

        let _ = srv.await;
        drop(tmp_file); // явно дропаем после сканирования (можно и не писать — дропнется автоматически)

        dbg!(&res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), false, "файл должен считаться заражённым");
    }

    #[tokio::test]
    async fn test_scan_clean() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        let response = "stream: OK\n";
        let srv = tokio::spawn(async move {
            run_mock_clamd(listener, response).await;
        });

        let (tmp_file, path) = write_temp_file(b"HELLO WORLD").await;

        let res = scan_with_clamav(&path, "127.0.0.1", addr.port()).await;

        let _ = srv.await;
        drop(tmp_file);

        dbg!(&res);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), true, "файл должен считаться чистым");
    }
}

