use std::net::{IpAddr, SocketAddr};
use std::str::FromStr as _;
use std::sync::OnceLock;

use metrics_exporter_prometheus::PrometheusBuilder;

use crate::config::server::MetricsConfig;

static INSTALLED: OnceLock<()> = OnceLock::new();

/// Install the global Prometheus recorder and start an HTTP listener that
/// serves `GET /metrics` in Prometheus text format.
///
/// Safe to call more than once — subsequent calls are no-ops that reuse
/// the already-installed recorder (important for test isolation).
///
/// # Errors
/// Fails if the host cannot be parsed, the recorder cannot be installed,
/// or the listener cannot bind.
pub fn install(config: &MetricsConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if INSTALLED.get().is_some() {
        return Ok(());
    }

    let ip = IpAddr::from_str(&config.host)
        .map_err(|e| format!("invalid metrics host '{}': {e}", config.host))?;
    let addr = SocketAddr::new(ip, config.port);

    PrometheusBuilder::new()
        .with_http_listener(addr)
        // Latency-oriented default buckets (seconds). Cover 1ms .. 10s.
        .set_buckets(&[
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])?
        .install()?;

    INSTALLED.set(()).ok();
    log::info!("Prometheus exporter listening on http://{addr}/metrics");
    Ok(())
}
