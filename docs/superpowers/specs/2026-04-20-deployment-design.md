# Deployment Design — anime-calendar

**Date:** 2026-04-20  
**Status:** Approved  
**Author:** Fernando Ferreira

---

## Summary

Two-environment deployment on GCP using Cloud Run (backend + frontend), Cloud SQL (production PostgreSQL), Neon serverless Postgres (staging), Upstash Redis (both environments), and Cloudflare as the CDN/WAF edge layer. CI remains GitHub Actions; CD extends it with automatic staging deploys on `main` merge and tag-gated production deploys. No VMs, no Kubernetes.

**Estimated cost:** ~$8–15/mo at A-scale (< 100 users). Upgrade path to B-scale (1,000 users) requires no architectural changes — only config/tier swaps.

---

## Architecture

### Request Flow

```
Browser
  └─► Cloudflare (DNS · WAF · CDN · DDoS)
        ├─► yourdomain.com        → Cloud Run: frontend  (nginx + Vue SPA)
        └─► api.yourdomain.com    → Cloud Run: backend   (Rust / Actix-Web)
                                        ├─► Cloud SQL (prod) / Neon (staging)
                                        ├─► Upstash Redis (TLS)
                                        └─► GCP Secret Manager (secrets injected at deploy)
```

### Services

| Service | Purpose | Staging | Production |
|---|---|---|---|
| Cloud Run `frontend` | Serves Vue SPA via nginx | Auto-deploy on `main` | Tag-gated deploy |
| Cloud Run `backend` | Rust API on port 8080 | Auto-deploy on `main` | Tag-gated deploy |
| Database | PostgreSQL 17 | Neon (free, auto-pauses) | Cloud SQL `db-f1-micro` |
| Cache | Redis | Upstash (free tier) | Upstash (pay-per-use) |
| Registry | Docker images | GCP Artifact Registry | Same registry, same SHA |
| Secrets | App credentials | GCP Secret Manager | GCP Secret Manager |
| Edge | CDN + WAF | Cloudflare free | Cloudflare free |

### Cloud Run Configuration

**Backend:**
- Memory: 512 MB · CPU: 1 (request-allocated)
- Min instances: 0 · Max instances: 5 (prod), 2 (staging)
- Max DB connections per instance: 3 (stays under Cloud SQL f1-micro's 25-connection limit at 5 max instances = 15 connections)
- Cloud SQL Auth Proxy integration: native (no VPC connector needed)
- Secrets: injected as env vars from Secret Manager

**Frontend:**
- Memory: 256 MB
- Min instances: 0 · Max instances: 5
- `VITE_API_BASE_URL` baked at image build time (staging vs prod values differ)

### Domain & Cookies

- `yourdomain.com` → Cloud Run frontend (production)
- `api.yourdomain.com` → Cloud Run backend (production)
- `staging.yourdomain.com` → Cloud Run frontend (staging)
- `staging-api.yourdomain.com` → Cloud Run backend (staging)
- Both served via Cloudflare (HTTPS enforced at Cloudflare, Cloud Run serves HTTP internally)
- Cookies: `Domain=.yourdomain.com; SameSite=Lax; Secure; HttpOnly` — works across subdomains (same eTLD+1)
- CORS: backend allows `https://yourdomain.com` origin (config value, not hardcoded)

### Cloudflare Origin Lock

A Cloudflare Transform Rule adds `X-CF-Origin-Secret: <value>` to every proxied request. The Rust backend rejects requests missing or mismatching this header with 403. The secret is stored in GCP Secret Manager and injected into Cloud Run at deploy time — it never appears in source code or GitHub.

This prevents anyone from bypassing Cloudflare by calling the Cloud Run URL directly.

---

## CI/CD Pipeline

### File Structure

Extend the existing `ci.yml` with deploy jobs, or create a separate `deploy.yml` — both approaches work. The spec assumes `deploy.yml` for separation of concerns.

### Triggers

| Trigger | Jobs run |
|---|---|
| PR to `main` | CI only (backend · frontend · openapi) — no deploy |
| Merge / push to `main` | CI + staging deploy (if CI passes) |
| Push tag `v*` | CI + production deploy gated by GitHub Environment approval |

### Staging Deploy (automatic)

1. Authenticate to GCP via **Workload Identity Federation** (`github-staging@…` SA)
2. Build `backend:{sha}` image → push to Artifact Registry
3. Build `frontend-staging:{sha}` (`VITE_API_BASE_URL=https://staging-api.yourdomain.com`) → push to Artifact Registry
4. Build `frontend-prod:{sha}` (`VITE_API_BASE_URL=https://api.yourdomain.com`) → push to Artifact Registry
5. Run sqlx migrations against Neon DB (direct TLS connection, no proxy needed)
6. Deploy Cloud Run backend (staging) revision
7. Deploy Cloud Run frontend (staging) revision using `frontend-staging:{sha}`
8. Smoke test: `GET https://staging.yourdomain.com/api/health`

> Both frontend variants are built here so `frontend-prod:{sha}` is already in Artifact Registry when the production deploy runs.

### Production Deploy (tag-gated)

1. GitHub Environment `production` gate — required reviewer(s) must approve
2. Authenticate via **Workload Identity Federation** (`github-prod@…` SA)
3. Pull `backend:{sha}` + `frontend-prod:{sha}` from Artifact Registry — **no rebuild**
4. Run sqlx migrations against Cloud SQL (via Cloud SQL Auth Proxy + IAM auth from `github-prod@…`)
5. Deploy Cloud Run backend (prod) revision
6. Deploy Cloud Run frontend (prod) revision using `frontend-prod:{sha}`
7. Smoke test: `GET https://yourdomain.com/api/health`
8. On success: tag Cloud Run revision as `stable`
9. On failure: `gcloud run services update-traffic backend --to-revisions=PREV=100` (~10s rollback)

**Key principle:** The backend image that ran in staging is the exact binary deployed to production — no recompilation. Both frontend images are built from the same git SHA; they differ only in the baked-in API URL.

### Workload Identity Federation

GitHub Actions authenticates to GCP via OIDC token exchange — no service account key JSON in GitHub Secrets. The WIF pool is bound to `repo:yourname/anime-calendar` so forks cannot impersonate it.

GitHub Secrets required (non-sensitive):
- `WORKLOAD_IDENTITY_PROVIDER` — WIF provider resource name
- `GCP_STAGING_SA` — staging service account email
- `GCP_PROD_SA` — production service account email

---

## Secrets & IAM

### GCP Secret Manager Layout

| Secret name | Value |
|---|---|
| `staging/database-url` | Neon connection string |
| `prod/database-url` | Cloud SQL connection string |
| `staging/redis-url` | Upstash TLS URL |
| `prod/redis-url` | Upstash TLS URL |
| `staging/jwt-secret` | Random 256-bit hex |
| `prod/jwt-secret` | Random 256-bit hex |
| `shared/google-client-id` | OAuth client ID |
| `shared/google-client-secret` | OAuth client secret |
| `shared/cf-origin-secret` | Cloudflare shared header secret |

Rotation: bump the secret version in Secret Manager + redeploy Cloud Run (no image rebuild needed).

### Service Accounts

| SA | Used by | Permissions |
|---|---|---|
| `cr-runtime@…` | Cloud Run containers (runtime identity) | `secretmanager.secretAccessor` · `cloudsql.client` |
| `github-staging@…` | GitHub Actions staging job | Artifact Registry Writer · Cloud Run Developer · `cloudsql.client` · `secretmanager.secretAccessor` |
| `github-prod@…` | GitHub Actions production job | Cloud Run Developer · `cloudsql.client` · `secretmanager.secretAccessor` — **no Artifact Registry write** |

---

## Database Migrations

### Execution

Migrations run as an explicit GitHub Actions step **before** the Cloud Run revision is deployed. The server binary does not auto-migrate on startup.

- **Staging:** `sqlx migrate run` via direct Neon TLS connection string (read from Secret Manager)
- **Production:** `sqlx migrate run` via Cloud SQL Auth Proxy binary (downloaded in CI) + IAM auth from `github-prod@…` SA

### Rules

All migrations must be **backwards-compatible**:
- Safe: add columns with `DEFAULT`, add tables, add indexes
- Unsafe (defer to next deploy): drop columns, rename columns, change column types
- Rollback via Cloud Run traffic split is instant; DB rollback requires manual `sqlx migrate revert` + redeployment

### `.sqlx/` Cache

The committed `.sqlx/` offline query cache must be regenerated and committed after any sqlx query macro change:
```bash
DATABASE_URL=... cargo sqlx prepare --workspace
```

---

## Monitoring

### Phase 1 — Built-in (day one, zero cost)

| Tool | What it covers |
|---|---|
| Cloud Run Metrics | Request count · latency (p50/p95/p99) · error rate · active instances |
| Cloud Logging | All `log::` output from Rust backend, structured JSON, queryable |
| GCP Uptime Check | `GET /api/health` every 60s from 3 regions · email alert on 2-min failure |

### Phase 2 — Prometheus + Grafana Cloud (when B/C scale warrants it)

The backend already exposes `GET /metrics` (Prometheus format) via `metrics-exporter-prometheus`. Upgrading to full observability requires:

**Step 1 — Protect `/metrics`**

Add Actix middleware (~20 lines, same pattern as the CF origin secret middleware) that requires `Authorization: Bearer <scrape-token>` on `/metrics`. Store the token in Secret Manager.

**Step 2 — Grafana Cloud account**

Create a free Grafana Cloud account. Free tier covers:
- 10,000 active metrics series
- 50 GB logs
- 14-day retention

Obtain the **Prometheus remote write endpoint** and a write token from the Grafana Cloud portal.

**Step 3 — Deploy Grafana Alloy**

Deploy [Grafana Alloy](https://grafana.com/docs/alloy/) as a minimal Cloud Run **service** (256 MB, min-instances=1, ~$3–5/mo). Alloy configuration:

```hcl
prometheus.scrape "backend" {
  targets = [{ __address__ = "https://api.yourdomain.com" }]
  bearer_token = env("METRICS_SCRAPE_TOKEN")
  scrape_interval = "15s"
  forward_to = [prometheus.remote_write.grafana_cloud.receiver]
}

prometheus.remote_write "grafana_cloud" {
  endpoint {
    url = env("GRAFANA_REMOTE_WRITE_URL")
    basic_auth {
      username = env("GRAFANA_USER_ID")
      password = env("GRAFANA_API_KEY")
    }
  }
}
```

**Step 4 — Import dashboards and alerts**

Import the existing dashboards from `ops/grafana/dashboards/` into Grafana Cloud. Migrate alert rules from `ops/prometheus/rules.yml` to Grafana Cloud alerting.

**Result:** Full Prometheus metrics visible in Grafana Cloud, alerts on error rate / latency spikes, no VMs or self-hosted Prometheus required.

---

## Upgrade Path to B-Scale

All upgrades are configuration changes — no architecture changes, no code changes.

| Component | Current (A-scale) | Upgrade (B-scale) | Action |
|---|---|---|---|
| Database | Cloud SQL `db-f1-micro` | Cloud SQL `db-g1-small` | GCP Console → Edit instance |
| Cache | Upstash pay-per-use | GCP Memorystore Basic 1GB | Create Memorystore, update `prod/redis-url` in Secret Manager, redeploy |
| Cloud Run max instances | 5 | 20 | Update Cloud Run service config |
| DB max connections per instance | 3 | 5 (g1-small supports ~100) | Update Cloud Run env var |

---

## Cost Estimate

| Item | Monthly cost |
|---|---|
| Cloud Run (backend + frontend, ~2M req free) | $0–3 |
| Cloud SQL `db-f1-micro` (prod) | ~$7 |
| Neon (staging) | $0 |
| Upstash Redis (both envs, A-scale) | $0–3 |
| Artifact Registry (~10 image versions) | ~$0.50 |
| Cloudflare (free tier) | $0 |
| GCP Secret Manager (~10 secrets, low access) | ~$0.10 |
| GCP Uptime Check | $0 |
| **Total** | **~$8–15/mo** |

B-scale upgrade adds ~$45–50/mo (Cloud SQL g1-small + Memorystore).

---

## Open Items (pre-implementation)

- [ ] Purchase custom domain (recommended: Cloudflare Registrar)
- [ ] Create GCP project + enable billing
- [ ] Decide GCP region (recommend `us-central1` for cost, or `europe-west1` if audience is EU-based)
- [ ] Create Neon account + project
- [ ] Create Upstash account + two Redis databases (staging + prod)
- [ ] Create Grafana Cloud account (optional, for Phase 2 monitoring)
