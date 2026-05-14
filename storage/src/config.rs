use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub s3_endpoint: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket: String,
    /// внешний URL для формирования ссылок
    pub s3_public_url: String,

    pub clamav_host: String,
    pub clamav_port: u16,

    pub server_port: u16,

    /// Максимальный размер файла (Гб)
    pub max_file_size: u16,
    /// Максимальный размер тела документа (Гб)
    /// Размер файла + заголовки и т.д.
    pub max_body_size: u16,
    /// Максимальный размер изображения (Мб)
    /// Для аватарок, баннеров
    pub max_image_size: u16,
    /// Максимальный размер видео (Мб)
    /// Для сторис
    pub max_video_size: u16,

    pub test: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenvy::dotenv().ok();

        let cfg = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;

        cfg.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_success() {
        temp_env::with_vars(
            [
                ("S3_ENDPOINT", Some("https://s3.example.com")),
                ("S3_ACCESS_KEY", Some("AKIAIOSFODNN7EXAMPLE")),
                ("S3_SECRET_KEY", Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY")),
                ("S3_BUCKET", Some("my-bucket")),
                ("S3_PUBLIC_URL", Some("https://public.example.com")),
                ("CLAMAV_HOST", Some("localhost")),
                ("CLAMAV_PORT", Some("3310")),
                ("SERVER_PORT", Some("8080")),
                ("MAX_FILE_SIZE", Some("8")),
                ("MAX_BODY_SIZE", Some("10")),
                ("MAX_IMAGE_SIZE", Some("11")),
                ("MAX_VIDEO_SIZE", Some("100")),
                ("TEST", Some(true)),
            ],
            || {
                let config = Config::from_env().expect("Config должен успешно загрузиться из окружения");

                assert_eq!(config.s3_endpoint, "https://s3.example.com");
                assert_eq!(config.s3_access_key, "AKIAIOSFODNN7EXAMPLE");
                assert_eq!(config.s3_secret_key, "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
                assert_eq!(config.s3_bucket, "my-bucket");
                assert_eq!(config.s3_public_url, "https://public.example.com");
                assert_eq!(config.clamav_host, "localhost");
                assert_eq!(config.clamav_port, 3310);
                assert_eq!(config.server_port, 8080);
                assert_eq!(config.max_file_size, 8);
                assert_eq!(config.max_body_size, 10);
                assert_eq!(config.max_image_size, 11);
                assert_eq!(config.max_video_size, 100);
                assert_eq!(config.test, true);
            },
        );
    }

    #[test]
    fn from_env_missing_required_field() {
        temp_env::with_vars(
            [
                ("S3_ENDPOINT", Some("https://s3.example.com")),
                // остальные обязательные поля отсутствуют
            ],
            || {
                let err = Config::from_env()
                    .expect_err("Должен вернуть ошибку при отсутствии обязательных переменных");

                let err_str = err.to_string().to_lowercase();
                assert!(
                    err_str.contains("s3_access_key")
                        || err_str.contains("missing field")
                        || err_str.contains("not found"),
                    "Ожидалась ошибка о missing field, получено: {}",
                    err
                );
            },
        );
    }

    #[test]
    fn from_env_invalid_u16() {
        temp_env::with_vars(
            [
                ("S3_ENDPOINT", Some("https://s3.example.com")),
                ("S3_ACCESS_KEY", Some("key")),
                ("S3_SECRET_KEY", Some("secret")),
                ("S3_BUCKET", Some("bucket")),
                ("S3_PUBLIC_URL", Some("https://public.example.com")),
                ("CLAMAV_HOST", Some("localhost")),
                ("CLAMAV_PORT", Some("99999")), // значение не помещается в u16
                ("SERVER_PORT", Some("8080")),
                ("MAX_FILE_SIZE", Some("8")),
                ("MAX_BODY_SIZE", Some("10")),
                ("MAX_IMAGE_SIZE", Some("11")),
                ("MAX_VIDEO_SIZE", Some("100")),
                ("TEST", Some(true)),
            ],
            || {
                let err = Config::from_env()
                    .expect_err("Должен вернуть ошибку при некорректном значении u16");

                let err_str = err.to_string().to_lowercase();
                assert!(
                    err_str.contains("clamav_port")
                        || err_str.contains("invalid digit")
                        || err_str.contains("parse"),
                    "Ожидалась ошибка парсинга u16, получено: {}",
                    err
                );
            },
        );
    }
}
