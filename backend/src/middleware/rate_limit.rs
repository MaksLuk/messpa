use governor::middleware::NoOpMiddleware;
use tower_governor::{
    governor::{GovernorConfig, GovernorConfigBuilder},
    key_extractor::PeerIpKeyExtractor,
};

pub fn rate_limit_config() -> GovernorConfig<PeerIpKeyExtractor, NoOpMiddleware> {
    GovernorConfigBuilder::default()
        .per_second(5)      // 5 запросов в секунду
        .burst_size(10)
        .finish()
        .unwrap()
}

