use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

static RECORDER_HANDLE: std::sync::OnceLock<PrometheusHandle> = std::sync::OnceLock::new();

pub fn setup_metrics() -> PrometheusHandle {
    let handle = PrometheusBuilder::new()
        // Глобальные бакеты для всех гистограмм (включая http_request_duration_seconds)
        .set_buckets(&[0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0])
        .expect("Failed to set histogram buckets")
        .install_recorder()
        .expect("Failed to install Prometheus recorder");

    RECORDER_HANDLE.set(handle.clone()).ok();
    handle
}

pub fn get_metrics_handle() -> &'static PrometheusHandle {
    RECORDER_HANDLE.get().expect("Metrics not initialized")
}

/// Middleware для трекинга HTTP запросов
pub async fn track_metrics(
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    gauge!("http_active_requests").increment(1.0);

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    // Базовые метрики
    counter!(
        "http_requests_total",
        "method" => method.clone(),
        "path" => path.clone(),
        "status" => status.clone()
    )
    .increment(1);

    histogram!(
        "http_request_duration_seconds",
        "method" => method,
        "path" => path,
        "status" => status
    )
    .record(latency);

    gauge!("http_active_requests").decrement(1.0);

    response
}
