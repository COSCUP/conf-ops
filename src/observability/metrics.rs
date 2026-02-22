use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

/// Application metrics collected in-memory.
#[derive(Debug)]
pub struct AppMetrics {
    pub http_requests_total: AtomicU64,
    pub http_errors_total: AtomicU64,
    pub ws_connections_active: AtomicU64,
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self {
            http_requests_total: AtomicU64::new(0),
            http_errors_total: AtomicU64::new(0),
            ws_connections_active: AtomicU64::new(0),
        }
    }
}

impl AppMetrics {
    /// Increment total HTTP requests.
    pub fn record_request(&self) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment total HTTP errors.
    pub fn record_error(&self) {
        self.http_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment active WebSocket connections.
    pub fn ws_connect(&self) {
        self.ws_connections_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active WebSocket connections.
    pub fn ws_disconnect(&self) {
        self.ws_connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Format metrics in Prometheus text exposition format.
    pub fn to_prometheus_text(&self, start_time: Instant) -> String {
        let uptime = start_time.elapsed().as_secs();
        let requests = self.http_requests_total.load(Ordering::Relaxed);
        let errors = self.http_errors_total.load(Ordering::Relaxed);
        let ws = self.ws_connections_active.load(Ordering::Relaxed);

        format!(
            "# HELP confops_http_requests_total Total HTTP requests\n\
             # TYPE confops_http_requests_total counter\n\
             confops_http_requests_total {requests}\n\
             # HELP confops_http_errors_total Total HTTP error responses\n\
             # TYPE confops_http_errors_total counter\n\
             confops_http_errors_total {errors}\n\
             # HELP confops_ws_connections_active Active WebSocket connections\n\
             # TYPE confops_ws_connections_active gauge\n\
             confops_ws_connections_active {ws}\n\
             # HELP confops_uptime_seconds Server uptime in seconds\n\
             # TYPE confops_uptime_seconds gauge\n\
             confops_uptime_seconds {uptime}\n"
        )
    }
}

/// Metrics endpoint state.
#[derive(Clone)]
pub struct MetricsState {
    pub metrics: Arc<AppMetrics>,
    pub start_time: Instant,
}

/// Handler for the /metrics endpoint.
///
/// Returns Prometheus text exposition format metrics.
#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "Prometheus metrics"),
    ),
    tag = "observability",
)]
pub async fn metrics_handler(State(state): State<MetricsState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
        state.metrics.to_prometheus_text(state.start_time),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_default_zero() {
        let m = AppMetrics::default();
        assert_eq!(m.http_requests_total.load(Ordering::Relaxed), 0);
        assert_eq!(m.http_errors_total.load(Ordering::Relaxed), 0);
        assert_eq!(m.ws_connections_active.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn record_request_increments() {
        let m = AppMetrics::default();
        m.record_request();
        m.record_request();
        assert_eq!(m.http_requests_total.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn ws_connect_disconnect() {
        let m = AppMetrics::default();
        m.ws_connect();
        m.ws_connect();
        assert_eq!(m.ws_connections_active.load(Ordering::Relaxed), 2);
        m.ws_disconnect();
        assert_eq!(m.ws_connections_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn prometheus_text_format() {
        let m = AppMetrics::default();
        m.record_request();
        m.record_error();
        let text = m.to_prometheus_text(Instant::now());
        assert!(text.contains("confops_http_requests_total 1"));
        assert!(text.contains("confops_http_errors_total 1"));
        assert!(text.contains("confops_ws_connections_active 0"));
        assert!(text.contains("confops_uptime_seconds"));
    }
}
