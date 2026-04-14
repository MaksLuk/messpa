use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_access_ttl: i64,   // минуты
    pub jwt_refresh_ttl: i64,  // дни
    pub refresh_pepper: String,
    pub server_port: u16,
    /// Адрес почтового сервиса (контейнер postfix)
    pub smtp_host: String,
    /// Порт почтового сервиса
    pub smtp_port: u16,
    /// Адрес почты для отправки сообщений
    pub email_addr: String,
    pub telegram_bot_token: String,
    pub telegram_bot_username: String,
    /// Тестируется ли приложение (true) или запущено на проде (false)
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
