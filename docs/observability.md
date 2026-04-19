# Observability

## Architecture

```
server (:9090/metrics) ←── Prometheus (:9091) ←── Grafana (:3001)
redis (:6379) ←── redis_exporter (:9121) ↗
```

The server exposes a Prometheus-format `/metrics` endpoint. Prometheus scrapes it every 15s alongside the redis_exporter. Grafana reads from Prometheus and auto-provisions four dashboards.

## Metrics

### HTTP (`server/src/metrics/http.rs`)

| Metric | Type | Labels |
|--------|------|--------|
| `http_requests_total` | counter | method, path, status |
| `http_request_duration_seconds` | histogram | method, path, status |

`path` is the route template (e.g. `/calendars/{id}`), not the raw URI. This keeps cardinality bounded at O(routes).

### Cache (`server/src/cache.rs`)

| Metric | Type | Labels |
|--------|------|--------|
| `cache_hits_total` | counter | — |
| `cache_misses_total` | counter | — |
| `cache_operation_duration_seconds` | histogram | op (get/set/delete/get_keys/flush) |

### Database (`server/src/metrics/db.rs`)

| Metric | Type | Labels |
|--------|------|--------|
| `db_queries_total` | counter | op, outcome (ok/err) |
| `db_query_duration_seconds` | histogram | op |

`op` is a `&'static str` literal (e.g. `"user.get_by_email"`) — only string literals compile, keeping cardinality fixed.

### Auth (`server/src/controllers/auth.rs`, `oauth.rs`, `email_verification.rs`, `password_reset.rs`)

| Metric | Type | Labels |
|--------|------|--------|
| `auth_login_attempts_total` | counter | outcome (ok/failed) |
| `auth_registrations_total` | counter | — |
| `auth_oauth_attempts_total` | counter | provider, outcome |
| `email_verifications_confirmed_total` | counter | outcome |
| `email_verifications_sent_total` | counter | — |
| `password_resets_requested_total` | counter | — |
| `password_resets_completed_total` | counter | outcome |

Auth metrics use only `outcome={ok,failed}` — they never leak whether a user exists.

## Recording Rules

Defined in `ops/prometheus/rules.yml`, evaluated every 30s:

| Rule | Expression |
|------|-----------|
| `job:http_requests:rate5m` | HTTP request rate by job/path/method/status |
| `job:http_request_duration_seconds:p99_5m` | P99 latency by path/method |
| `job:cache_hit_rate:5m` | `hits / (hits + misses)` |
| `job:db_query_error_rate:5m` | error queries / total queries by op |

## Alerts

| Alert | Condition | Severity |
|-------|-----------|----------|
| `HighHttp5xxRate` | 5xx > 5% for 5m | page |
| `HighRequestLatencyP99` | P99 > 2s for 10m | ticket |
| `LowCacheHitRate` | hit rate < 50% for 15m | ticket |
| `DbQueryErrorsSpiking` | error rate > 2% for 5m | page |
| `LoginFailureSpike` | failures > 5/s for 5m | ticket |

## Server Config

Metrics bind port is configured in `config.toml`:

```toml
[metrics]
port = 9090
```

The exporter listens on `0.0.0.0:{port}` and serves only the `/metrics` path.

## Running the Stack

```bash
cd ops
docker compose -f docker-compose.monitoring.yml up -d
```

Grafana: http://localhost:3001 (admin / admin)
Prometheus: http://localhost:9091

## Testing Metrics in Rust

All metric tests share a single `DebuggingRecorder` installed once via `OnceLock` in `server/src/test_helpers.rs`:

```rust
let snapshotter = crate::test_helpers::shared_snapshotter();
// ... trigger instrumented code ...
let snapshot = snapshotter.snapshot();
// assert metric names present in snapshot
```

Never install a second recorder in a test module — `metrics` has a single global recorder per process.
