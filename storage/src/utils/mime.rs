/// Разрешённые MIME-типы для изображений
pub fn is_allowed_image_mime(content_type: &str) -> bool {
    let allowed = [
        "image/jpeg",
        "image/jpg",
        "image/png",
        "image/webp",
        "image/gif",
        "image/svg+xml",
        "image/avif",
    ];
    allowed.iter().any(|&m| m == content_type)
}

/// Разрешённые MIME-типы для видео
pub fn is_allowed_video_mime(content_type: &str) -> bool {
    let allowed = [
        "video/mp4",
        "video/webm",
        "video/ogg",
        "video/quicktime", // .mov
    ];
    allowed.iter().any(|&m| m == content_type)
}
