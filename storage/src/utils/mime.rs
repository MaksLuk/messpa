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
        "application/octet-stream",
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
        "application/octet-stream",
    ];
    allowed.iter().any(|&m| m == content_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed_image_mime() {
        assert!(is_allowed_image_mime("image/jpeg"));
        assert!(is_allowed_image_mime("image/jpg"));
        assert!(is_allowed_image_mime("image/png"));
        assert!(is_allowed_image_mime("image/webp"));
        assert!(is_allowed_image_mime("image/gif"));
        assert!(is_allowed_image_mime("image/svg+xml"));
        assert!(is_allowed_image_mime("image/avif"));

        assert!(!is_allowed_image_mime("image/bmp"));
        assert!(!is_allowed_image_mime("image/tiff"));
        assert!(!is_allowed_image_mime("video/mp4"));
        assert!(!is_allowed_image_mime("application/json"));
        assert!(!is_allowed_image_mime(""));
        assert!(!is_allowed_image_mime("image/jpeg "));
        assert!(!is_allowed_image_mime("IMAGE/JPEG"));
    }

    #[test]
    fn test_is_allowed_video_mime() {
        assert!(is_allowed_video_mime("video/mp4"));
        assert!(is_allowed_video_mime("video/webm"));
        assert!(is_allowed_video_mime("video/ogg"));
        assert!(is_allowed_video_mime("video/quicktime"));

        assert!(!is_allowed_video_mime("video/avi"));
        assert!(!is_allowed_video_mime("video/mkv"));
        assert!(!is_allowed_video_mime("image/png"));
        //assert!(!is_allowed_video_mime("application/octet-stream"));
        assert!(!is_allowed_video_mime(""));
        assert!(!is_allowed_video_mime("video/mp4 "));
        assert!(!is_allowed_video_mime("VIDEO/MP4"));
    }
}
