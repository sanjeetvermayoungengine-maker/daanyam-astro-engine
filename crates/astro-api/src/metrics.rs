use std::sync::OnceLock;

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

use crate::ApiState;

static PROMETHEUS: OnceLock<PrometheusHandle> = OnceLock::new();

pub fn init(state: &ApiState) {
    PROMETHEUS.get_or_init(|| {
        let handle = PrometheusBuilder::new()
            .install_recorder()
            .expect("failed to install Prometheus metrics recorder");
        metrics::gauge!("astro_kernel_load_seconds").set(state.kernel_load_seconds());
        handle
    });
}

pub fn record_request(path: &str, status: u16, latency_ms: u128) {
    let status_label = status.to_string();
    metrics::counter!(
        "astro_requests_total",
        "path" => path.to_string(),
        "status" => status_label
    )
    .increment(1);
    metrics::histogram!("astro_request_latency_ms", "path" => path.to_string())
        .record(latency_ms as f64);
}

pub fn render() -> Option<String> {
    PROMETHEUS.get().map(PrometheusHandle::render)
}
