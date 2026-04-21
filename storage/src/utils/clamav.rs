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
