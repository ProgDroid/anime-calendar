pub mod db;
pub mod exporter;
pub mod http;
pub mod names;

/// Initialise the metrics subsystem: install the Prometheus recorder and
/// start the HTTP exporter on the configured loopback address. Safe to
/// call once at process startup.
///
/// # Errors
/// Fails if the Prometheus recorder cannot be installed or the listener
/// cannot bind.
pub fn init(
    config: &crate::config::server::MetricsConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !config.enabled {
        return Ok(());
    }
    exporter::install(config)?;
    describe();
    Ok(())
}

fn describe() {
    use metrics::{Unit, describe_counter, describe_histogram};
    use names::{
        AUTH_LOGIN_ATTEMPTS_TOTAL, AUTH_OAUTH_ATTEMPTS_TOTAL, AUTH_REGISTRATIONS_TOTAL,
        CACHE_HITS_TOTAL, CACHE_MISSES_TOTAL, CACHE_OPERATION_DURATION_SECONDS, DB_QUERIES_TOTAL,
        DB_QUERY_DURATION_SECONDS, EMAIL_VERIFICATIONS_CONFIRMED_TOTAL,
        EMAIL_VERIFICATIONS_SENT_TOTAL, HTTP_REQUEST_DURATION_SECONDS, HTTP_REQUESTS_TOTAL,
        PASSWORD_RESETS_COMPLETED_TOTAL, PASSWORD_RESETS_REQUESTED_TOTAL,
        STRIPE_WEBHOOK_PAYMENT_ACTION_REQUIRED_TOTAL,
    };

    describe_counter!(
        CACHE_HITS_TOTAL,
        Unit::Count,
        "Cache reads that returned a value"
    );
    describe_counter!(
        CACHE_MISSES_TOTAL,
        Unit::Count,
        "Cache reads that returned no value"
    );
    describe_histogram!(
        CACHE_OPERATION_DURATION_SECONDS,
        Unit::Seconds,
        "Latency of cache operations"
    );
    describe_counter!(
        HTTP_REQUESTS_TOTAL,
        Unit::Count,
        "HTTP requests handled by the server"
    );
    describe_histogram!(
        HTTP_REQUEST_DURATION_SECONDS,
        Unit::Seconds,
        "HTTP request latency"
    );
    describe_counter!(DB_QUERIES_TOTAL, Unit::Count, "Database queries executed");
    describe_histogram!(
        DB_QUERY_DURATION_SECONDS,
        Unit::Seconds,
        "Database query latency"
    );
    describe_counter!(
        AUTH_LOGIN_ATTEMPTS_TOTAL,
        Unit::Count,
        "User login attempts"
    );
    describe_counter!(AUTH_REGISTRATIONS_TOTAL, Unit::Count, "User registrations");
    describe_counter!(
        AUTH_OAUTH_ATTEMPTS_TOTAL,
        Unit::Count,
        "OAuth sign-in attempts"
    );
    describe_counter!(
        EMAIL_VERIFICATIONS_SENT_TOTAL,
        Unit::Count,
        "Email verification messages sent"
    );
    describe_counter!(
        EMAIL_VERIFICATIONS_CONFIRMED_TOTAL,
        Unit::Count,
        "Email addresses confirmed"
    );
    describe_counter!(
        PASSWORD_RESETS_REQUESTED_TOTAL,
        Unit::Count,
        "Password reset requests"
    );
    describe_counter!(
        PASSWORD_RESETS_COMPLETED_TOTAL,
        Unit::Count,
        "Password resets completed"
    );
    describe_counter!(
        STRIPE_WEBHOOK_PAYMENT_ACTION_REQUIRED_TOTAL,
        Unit::Count,
        "Stripe invoice.payment_action_required webhooks (SCA / 3DS prompts)"
    );
}
