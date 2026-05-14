use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client as S3Client, config::Region};

#[derive(Clone)]
pub struct AppState {
    pub s3_client: S3Client,
    pub s3_bucket: String,
    pub s3_public_url: String,
    pub clamav_host: String,
    pub clamav_port: u16,
    pub max_file_size: u16,
    pub max_image_size: u16,
    pub max_video_size: u16,
}

impl AppState {
    pub async fn new(config: &crate::config::Config) -> Self {
        let s3_config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&config.s3_endpoint)
            .region(Region::new("us-east-1"))
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                &config.s3_access_key,
                &config.s3_secret_key,
                None,
                None,
                "static",
            ))
            .load()
            .await;

        let s3_client = S3Client::from_conf(
            aws_sdk_s3::config::Builder::from(&s3_config)
                .force_path_style(true)
                .build()
        );

        Self {
            s3_client,
            s3_bucket: config.s3_bucket.clone(),
            s3_public_url: config.s3_public_url.clone(),
            clamav_host: config.clamav_host.clone(),
            clamav_port: config.clamav_port,
            max_file_size: config.max_file_size,
            max_image_size: config.max_image_size,
            max_video_size: config.max_video_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_state_new_success() {
        let config = crate::config::Config {
            s3_endpoint: "http://127.0.0.1:9000".into(),
            s3_access_key: "minioadmin".into(),
            s3_secret_key: "minioadmin".into(),
            s3_bucket: "testbucket".into(),
            s3_public_url: "http://localhost:9000".into(),
            clamav_host: "127.0.0.1".into(),
            clamav_port: 3310,
            server_port: 3003,
            max_file_size: 8,
            max_body_size: 10,
            max_image_size: 10,
            max_video_size: 100,
            test: false,
        };

        let state = AppState::new(&config).await;

        assert_eq!(state.s3_bucket, "testbucket");
        assert_eq!(state.s3_public_url, "http://localhost:9000");
        assert_eq!(state.clamav_host, "127.0.0.1");
        assert_eq!(state.clamav_port, 3310);
        assert_eq!(state.max_file_size, 8);
        assert_eq!(state.max_image_size, 10);
        assert_eq!(state.max_video_size, 100);
    }
}
