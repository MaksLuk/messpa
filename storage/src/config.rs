use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub s3_endpoint: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_bucket: String,
    pub s3_public_url: String,        // внешний URL для формирования ссылок

    pub clamav_host: String,
    pub clamav_port: u16,

    pub server_port: u16,
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

