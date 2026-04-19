# Monitoring Stack

Prometheus + Grafana monitoring for anime-calendar.

## Services

| Service | Host port | Purpose |
|---------|-----------|---------|
| Prometheus | 9091 | Scrapes server metrics + redis_exporter |
| Grafana | 3001 | Dashboards (admin / admin) |
| redis_exporter | 9121 | Exposes Redis metrics to Prometheus |

## Prerequisites

- Docker + Docker Compose
- Server running on port 9090 (metrics endpoint at `/metrics`)

## Start

```bash
cd ops
docker compose -f docker-compose.monitoring.yml up -d
```

## Dashboards

Grafana auto-provisions four dashboards under **Anime Calendar**:

| Dashboard | UID | Contents |
|-----------|-----|---------|
| HTTP | ac-http | Request rate, error %, latency percentiles, slowest routes |
| Cache | ac-cache | Hit rate, Redis memory/clients, op latency |
| Database | ac-db | Query rate, error rate, P99 latency by op |
| Auth | ac-auth | Login outcomes, registrations/hour, OAuth, email verification |

## Alerts

Defined in `prometheus/rules.yml`:

| Alert | Condition | Severity |
|-------|-----------|----------|
| HighHttp5xxRate | 5xx > 5% for 5m | page |
| HighRequestLatencyP99 | P99 > 2s for 10m | ticket |
| LowCacheHitRate | hit rate < 50% for 15m | ticket |
| DbQueryErrorsSpiking | error rate > 2% for 5m | page |
| LoginFailureSpike | failures > 5/s for 5m | ticket |

## Metrics emitted by the server

| Metric | Type | Labels |
|--------|------|--------|
| `http_requests_total` | counter | method, path, status |
| `http_request_duration_seconds` | histogram | method, path, status |
| `cache_hits_total` | counter | — |
| `cache_misses_total` | counter | — |
| `cache_operation_duration_seconds` | histogram | op |
| `db_queries_total` | counter | op, outcome |
| `db_query_duration_seconds` | histogram | op |
| `auth_login_attempts_total` | counter | outcome |
| `auth_registrations_total` | counter | — |
| `auth_oauth_attempts_total` | counter | provider, outcome |
| `email_verifications_confirmed_total` | counter | outcome |
| `email_verifications_sent_total` | counter | — |
| `password_resets_requested_total` | counter | — |
| `password_resets_completed_total` | counter | outcome |
