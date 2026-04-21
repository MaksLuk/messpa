use aws_sdk_s3::Client as S3Client;

#[derive(Clone)]
pub struct AppState {
    pub s3_client: S3Client,
    pub s3_bucket: String,
    pub s3_public_url: String,
    pub clamav_host: String,
    pub clamav_port: u16,
}

impl AppState {
    pub async fn new(config: &crate::config::Config) -> Self {
        let sdk_config = aws_config::from_env()
            .endpoint_url(&config.s3_endpoint)
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                &config.s3_access_key,
                &config.s3_secret_key,
                None,
                None,
                "static",
            ))
            .load()
            .await;

        let s3_client = S3Client::new(&sdk_config);

        Self {
            s3_client,
            s3_bucket: config.s3_bucket.clone(),
            s3_public_url: config.s3_public_url.clone(),
            clamav_host: config.clamav_host.clone(),
            clamav_port: config.clamav_port,
        }
    }
}
