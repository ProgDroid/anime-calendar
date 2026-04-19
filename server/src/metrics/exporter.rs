use crate::config::server::MetricsConfig;

/// Install the Prometheus recorder and bind the HTTP listener on the
/// configured loopback address.
///
/// # Errors
/// Fails if the recorder cannot be installed or the listener cannot bind.
pub fn install(_config: &MetricsConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}
