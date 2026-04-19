//! End-to-end test: install the Prometheus recorder on a random port,
//! emit a metric, scrape /metrics, assert the output is well-formed.

use std::net::{IpAddr, Ipv4Addr};

#[tokio::test]
async fn prometheus_endpoint_serves_emitted_metrics() {
    // Bind to port 0 to let the OS pick a free port — avoids conflicts
    // when tests run in parallel.
    let listener =
        std::net::TcpListener::bind((IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).expect("bind 0");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let config = server::config::server::MetricsConfig {
        enabled: true,
        host: "127.0.0.1".to_owned(),
        port,
    };

    server::metrics::init(&config).expect("metrics init");

    // Emit a metric so scraping returns a value.
    metrics::counter!("cache_hits_total").increment(3);

    // Give the exporter a tick to start.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let url = format!("http://127.0.0.1:{port}/metrics");
    let body = reqwest::get(&url)
        .await
        .expect("scrape")
        .text()
        .await
        .unwrap();

    assert!(
        body.contains("cache_hits_total"),
        "body missing cache_hits_total:\n{body}"
    );
    assert!(
        body.contains("cache_hits_total 3") || body.contains("cache_hits_total{"),
        "counter value not reflected:\n{body}"
    );
    assert!(
        body.contains("# TYPE cache_hits_total counter"),
        "missing TYPE metadata:\n{body}"
    );
}
