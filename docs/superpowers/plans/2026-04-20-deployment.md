# Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make anime-calendar deployable to GCP Cloud Run with full CI/CD automation, Cloudflare protection, and two isolated environments (staging/production).

**Architecture:** Rust backend + Vue frontend as separate Cloud Run services behind Cloudflare, in `europe-west1`. Secrets injected at runtime from GCP Secret Manager. No config files in production — env vars only.

**Tech Stack:** GCP Cloud Run, Cloudflare (free), GCP Secret Manager, GCP Artifact Registry, GitHub Actions, Workload Identity Federation, sqlx migrations, nginx envsubst, Docker multi-stage builds. Postgres and Redis vendors: see *Decisions of record* below.

---

## Decisions of record (2026-09-22)

**This plan was written 2026-04-20 and its original platform choices are superseded.**
The authority is `docs/deployment-readiness.md` (2026-09-10), which was written after a
source-level assessment and was **not** available to the sessions that implemented Tasks 1–8.
Where the two disagree, the readiness doc wins. Superseded choices, recorded so nobody
reinstates them from an older draft:

| This plan originally said | Superseded by | Why |
|---|---|---|
| `us-central1` | **`europe-west1`** | GDPR (EU user data stays in the EU, which shortens the legal-basis paragraph the unticked legal checklist requires); ~10–20 ms vs ~120 ms from Europe, which matters for SSE presence. Cloud Run pricing is near-flat across regions and scale-to-zero removes the idle-cost argument. |
| Staging on Neon + Upstash, production on Cloud SQL + Upstash | **Same mechanism in both environments** | Owner's stated hard constraint. Two different backing stores means staging stops being a rehearsal. |
| Upstash Redis | **Not Upstash** | Unresolved whether `SUBSCRIBE` works over their native TCP endpoint; their own troubleshooting docs steer you to the REST client "to avoid persistent connections". This app holds a `SUBSCRIBE` open for a user's whole browser session, which is the opposite shape. |

**Still open (does not block Tasks 1–8, 12, or 13):** the Postgres vendor and the Redis
vendor. Only **Tasks 9, 10 and 11** — infrastructure provisioning, external account setup and
secret population — depend on the answer. Everything else is vendor-neutral, including the
code already written: `database.url` and `redis.url` take a full connection string verbatim,
and the vendor names that remain in doc comments and test fixtures are illustrative only.

**Evidence gathered 2026-09-22 that bears on the open choices:**

- **Redis Cloud Essentials scales the connection ceiling steeply and upgrades in place.**
  30 MB free = 30 connections / 100 ops·s⁻¹ / 5 GB·mo⁻¹; 250 MB (~$5) = **256** connections /
  1,000 ops·s⁻¹ / 100 GB·mo⁻¹. Redis documents that changing plan leaves "data and endpoints
  ... not disrupted" with no availability impact — so outgrowing the free tier costs a console
  click, not a migration or a redeploy.
- **The connection budget in readiness §2 is an implementation artifact, not a floor.**
  `server/src/redis_pubsub.rs:235-237` opens a fresh `Client::open` + `get_async_pubsub()` per
  channel. Redis pub/sub permits one connection to `SUBSCRIBE` to many channels; multiplexing
  collapses `3 fixed + 2/calendar + 1/viewer` to roughly **4 flat**. See Task 16.
- **Postgres pooling is unbudgeted and is the sharper constraint.** See Task 15.
- **Redis Cloud pub/sub is not *documented as restricted* on Essentials, which is not the same
  as documented as supported.** Same evidential gap that left Upstash unresolved — settle it
  with a live `SUBSCRIBE` against a real free database before committing. Essentials free is
  single-shard, so the Redis Enterprise clustered-pub/sub caveat does not apply.

### Sequencing

Tasks 14–20 were appended in discovery order, **not** execution order. The numbering is an id,
not a sequence. Real order:

| When | Tasks | Why | Status |
|---|---|---|---|
| **Before provisioning anything** | 15 (pool budget), 19 (cargo-deny) | 15 sets the Postgres sizing arithmetic that Task 9 provisions against; 19 is a green-CI precondition for trusting any deploy. | ✅ **both done 2026-09-22** |
| | 20 (migration strategy) | Decides what Task 12's workflow does. | ✅ **decided 2026-09-22** — CI applies, startup verifies. Work not yet done. |
| **Blocked on buying the domain** | 9, 10, 11 | Vendor provisioning, external accounts, secret values. | Blocked on the domain only — the environment-shape question is settled |
| **Before the first public deploy** | 17 (Access), 14 (reconcile) | 17 or the smoke test reports failure on a healthy service; 14 or Stripe state silently stops reconciling. | Open |
| **Before letting anyone in free** | 18 (comp path) | Blocked the owner's own use and the friends allow-list. | ✅ **done 2026-09-22** |
| **Any time — removes a constraint, fixes no fault** | 16 (Redis multiplexing) | Worth doing before sizing Redis, since it changes the answer. | ✅ **done 2026-09-22** (Steps 1–3; Step 4, the throughput measurement, still open) |

**Sizing is no longer a blocker on either vendor decision.** Task 15 made the Postgres budget one
configurable number that fits the smallest Cloud SQL tier, and the Redis evidence above shows the
free tier's ceiling lifts to 256 connections for ~$5 with an in-place upgrade. Both decisions can
now be made on cost and preference rather than on whether the app fits.

### Decisions taken 2026-09-22 (evening) — these close the two open questions

**1. ONE environment. There is no staging.** Production stands alone. Asked whether this should
be a bounded pre-launch campaign, permanent staging alongside production, or a single
environment, the owner chose the single environment.

What that changes, concretely:

- **Tasks 9–12 build one environment, not two.** Every step in them that is written twice —
  two Cloud Run services, two databases, two secret sets, two deploy jobs — collapses to one.
  Task 12's workflow does not need an environment matrix or a `staging`/`production` input.
- **The "same mechanism in both environments" constraint is moot**, because there is only one.
  It was the hard constraint that ruled out a Redis sidecar container. **That does not
  reinstate the sidecar** — the reason to keep managed Redis is now simply that production
  should not depend on a container that dies with the instance.
- **Migrations are rehearsed locally, not on a staging box.** That path is proven: applying all
  21 migrations in order to a fresh `postgres:16` container yields 11 tables, 0 failures
  (re-verified 2026-09-22). This is now the *only* rehearsal, so it has to actually be run
  before each migration ships, not assumed.
- **It sharpens, rather than settles, the Postgres choice.** The argument for Neon was
  copy-on-write branching to rehearse migrations against prod-shaped data. With no staging,
  branching becomes the *only* way to rehearse against real data — so it gets **more**
  valuable here, not less. Weigh that against every query being cross-cloud to Frankfurt.
  This is a live trade-off, not a closed one.
- **The "production cliff" in readiness §5 arrives on day one**, since day one *is*
  production. Task 16 has since taken the connection half of that cliff away (4 per replica,
  20 at `--max-instances=5`, against the free tier's 30). The throughput half is unmeasured.

**2. Task 20 — CI applies, startup verifies.** `sqlx migrate run` from the deploy workflow, so
it runs exactly once per deploy instead of racing across cold-starting instances. Plus a startup
check that reads `sqlx::migrate!`'s applied-versions list **without applying anything** and
refuses to serve on a mismatch. That keeps CI's single-run property while closing what the
CI-only approach gives up: a container started outside the workflow now fails loudly instead of
serving traffic against a schema it cannot satisfy. Recorded in `docs/deployment-readiness.md`
B-1, which previously said "migrate at startup" and contradicted this plan.

### Resume here (2026-09-22, evening)

**CI on `main` is green** — all five jobs, first time since 2026-06-12, so B-2 is closed.
**Task 16 is done** (Steps 1–3; Step 4's throughput measurement is still open).

**One thing now gates everything else, and it is not code: buy the domain.** Porkbun,
nameservers → Cloudflare immediately. It gates TLS (B-3), Resend's DKIM/SPF, and Google OAuth,
which rejects bare IPs as authorized origins. B-3 and B-4 both wait on it, as do Tasks 9–11.
As of this session the domain has **not** been bought and the owner wants to decide *which*
domain first — so that conversation is the next thing, not a purchase.

Unblocked work, independent of the domain, in the recommended order:

| Order | Task | Why this one |
|---|---|---|
| 1 | **14** — reconcile → Cloud Scheduler | Fails *silently* in production otherwise: the hourly timer never fires under CPU throttling and Stripe state stops reconciling. With one environment, production is the only environment, so there is no staging run to catch it. |
| 2 | **20** — implement the migration decision | The decision is made (above); the work is not. Also drops the stale `schema.sql` mount, which is a hard blocker for anyone setting the project up fresh. |
| 3 | **17** — Access + a smoke test that survives it | Reports a healthy service as failed on the very first deploy. Step 1 needs the domain; the smoke-test fix does not. |

**Task 20 also needs a decision rather than work:** the readiness doc's B-1 says migrate at
startup, this plan's Task 12 migrates from CI, and **they currently contradict each other** — the
same condition that produced the `schema.sql` drift in the first place.

Tasks 1–8 are **done in code** on the cloud branch, verified type-clean, and the checkbox state
in this document was never updated to reflect that — do not re-run them from the unticked boxes.

**Spec note:** The design doc `docs/superpowers/specs/2026-04-20-deployment-design.md` states two frontend images (frontend-staging, frontend-prod). That is incorrect — the frontend uses `baseURL: '/api'` (relative URL via nginx proxy), so `VITE_API_BASE_URL` is never consulted at runtime. One image with runtime `BACKEND_URL` envsubst is correct.

**Runtime config update (2026-05-03):** `VITE_GOOGLE_CLIENT_ID` has also been retired. The Google OAuth client ID is now served by the backend via `GET /api/public-config` and fetched by the SPA at bootstrap (see `server/src/controllers/public_config.rs`, `frontend/src/services/publicConfig.ts`). The frontend image is now fully environment-agnostic — no Vite build args of any kind. All references to `VITE_GOOGLE_CLIENT_ID` below have been struck through and updated.

---

## File Map

**Created:**
- `server/src/controllers/health.rs` — `GET /health` handler (nginx strips the `/api` prefix)
- `server/src/middleware/cloudflare.rs` — CF origin secret middleware

**Modified:**
- `server/src/controllers.rs` — add `pub mod health;`
- `server/src/middleware.rs` — add `pub mod cloudflare;`
- `server/src/config/server.rs` — add env var override, `cf_origin_secret`, `cookie_domain` to `CookieSettings`, `RedisConfig.url` override
- `server/src/config/database.rs` — add env var override + `url` field for Neon/Cloud SQL connection strings
- `server/src/cache.rs` — add `Cache::from_url()` constructor
- `server/src/main.rs` — switch to `Cache::from_url()` when `redis.url` is set
- `server/src/server.rs` — wire health endpoint + CF middleware + `cookie_domain`
- `server/src/controllers/auth.rs` — `SameSite::Strict` → `SameSite::Lax`, add optional domain to both cookie builders
- `frontend/nginx.conf` → `frontend/nginx.conf.template` — replace `http://server:8080` with `${BACKEND_URL}`
- `frontend/Dockerfile` — envsubst at container startup instead of baked URL; no build args (Google client ID now served at runtime by backend `/api/public-config`)
- `config.toml.dist` — add `cf_origin_secret`, `cookie_domain` fields
- `.github/workflows/ci.yml` → **rename** to keep CI, add staging + production deploy jobs (or create `.github/workflows/deploy.yml`)

---

## Task 1: Health Endpoint

**Files:**
- Create: `server/src/controllers/health.rs`
- Modify: `server/src/controllers.rs`
- Modify: `server/src/server.rs`

**Route prefix (corrected 2026-09-14):** register the handler at `/health`, **not** `/api/health`.
`frontend/nginx.conf` proxies `location /api/` to `http://server:8080/` — the trailing slash strips
the `/api` prefix, so a browser request to `/api/health` reaches actix as `/health`. Every other
backend route follows the same convention (`#[get("/public-config")]`, `/calendars/...`).
The public-URL smoke tests in Tasks 12 and 13 correctly keep `/api/health` — they go through nginx.

- [x] **Step 1: Write the health controller**

Create `server/src/controllers/health.rs`:

```rust
use actix_web::{HttpResponse, get};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[get("/health")]
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse { status: "ok" })
}
```

- [x] **Step 2: Register the module**

Edit `server/src/controllers.rs`, add:

```rust
pub mod health;
```

- [x] **Step 3: Register the route in server.rs**

In `server/src/server.rs`, add to the `use crate::controllers` import:

```rust
use crate::controllers::{
    auth, calendar, email_verification, health, item, items, oauth, password_reset, refresh, user,
};
```

Add `.service(health::health)` to the service list (before `.service(item::get)`).

- [x] **Step 4: Verify it compiles**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors.

- [x] **Step 5: Write a test for the health endpoint**

Add to `server/src/controllers/health.rs`:

```rust
#[cfg(test)]
mod tests {
    use actix_web::{App, test};
    use super::*;

    #[actix_web::test]
    async fn health_returns_200_ok() {
        let app = test::init_service(App::new().service(health)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }

    #[actix_web::test]
    async fn health_returns_status_ok_json() {
        let app = test::init_service(App::new().service(health)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
        assert_eq!(body["status"], "ok");
    }
}
```

- [x] **Step 6: Run tests**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server controllers::health
```

Expected: 2 tests pass.

- [x] **Step 7: Commit**

```bash
git add server/src/controllers/health.rs server/src/controllers.rs server/src/server.rs
git commit -m "feat: add GET /health endpoint"
```

---

## Task 2: Environment Variable Config Override

The `config` crate supports layered sources — file first, then env vars override. Cloud Run has no config files, so env vars must work standalone.

**Corrections applied 2026-09-14** (all three verified against `config` 0.15.14 and covered by tests):

1. **`Database::new()` must use `Environment::with_prefix("DATABASE")`, not `Environment::default()`.**
   Task 12 injects `DATABASE__URL`. Unprefixed, that splits on `__` into `database` -> `url` — a map
   landing on the `database: String` field — and the load *fails* rather than populating `url`.
   Prefixing also stops ambient `USER` / `HOST` / `PORT` / `PASS` (present in most shells and in CI
   runners) from silently overriding `database.toml`, since the crate skips any key not matching the
   prefix. `Server::new()` stays unprefixed on purpose: Cloud Run injects `PORT` itself.
2. **The `Server` struct block in Step 3 is stale.** It predates `trust_proxy_header`, `app`,
   `stripe`, `reconcile`, `cache`, `limits` and `sharing`. Add the two new fields; do not paste the
   block over the current struct or you will delete seven.
3. **Step 4 breaks the build on its own.** Adding `domain` to `CookieSettings` requires updating all
   five construction sites (`server/src/server.rs` plus the test helpers in `controllers/auth.rs`,
   `controllers/refresh.rs`, `controllers/email_verification.rs`), or Step 6 cannot compile.

**Correction applied 2026-09-22 — this one was a latent startup crash.** `allowed_origins` is a
`Vec<String>`, and an environment variable is always a scalar. Task 12 injects
`ALLOWED_ORIGINS=https://staging.${DOMAIN}`; measured against `config` 0.15.14, that load
**fails outright**:

```
invalid type: string "https://staging.example.com", expected a sequence for key `allowed_origins`
```

The container would not have started on the first deploy, and the error names a config key
rather than the deploy command, so it would not have been obvious where it came from. Fixed by
adding `.list_separator(",").with_list_parse_key("allowed_origins")` to `Server::env_source()`.

The scoping matters: a bare `list_separator` turns **every** string field into a list
(`config/src/env.rs:47` warns about exactly this), which would silently truncate any secret
containing a comma to its first segment. Three tests lock it in — single value, comma-separated
list, and a comma inside `JWT_SECRET` staying intact.

Why nothing caught it: the test module's `required()` fixture lists the fields the struct needs
to *deserialize*, and `allowed_origins` has a serde default — so it was invisible to that
question while being load-bearing for the deploy. **A config fixture should mirror the deploy
command's env block, not the struct's mandatory fields.**

**Files:**
- Modify: `server/src/config/server.rs`
- Modify: `server/src/config/database.rs`

- [x] **Step 1: Update Server::new() to support env vars**

In `server/src/config/server.rs`, replace the `Server::new()` implementation:

```rust
impl Server {
    /// # Errors
    /// Returns `ConfigError` if config is invalid
    pub fn new() -> std::result::Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name(CONFIG_FILE).required(false))
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()
    }
}
```

Add `config` to the imports at the top — it's already in scope via `use config::{Config, ConfigError, File};`. Add `config::Environment` reference inline or bring it with:

```rust
use config::{Config, ConfigError, File};
```

(No change needed — `config::Environment` is accessed with the full path.)

- [x] **Step 2: Add optional redis.url field to RedisConfig**

In `server/src/config/server.rs`, update `RedisConfig`:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub db: u8,
    /// Full Redis URL (e.g. `rediss://...` for Upstash TLS). When set,
    /// individual host/port/password fields are ignored.
    #[serde(default)]
    pub url: Option<String>,
}
```

Update `Default for RedisConfig` to include the new field:

```rust
impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6379,
            password: String::new(),
            db: 0,
            url: None,
        }
    }
}
```

- [x] **Step 3: Add cf_origin_secret and cookie_domain to Server**

In `server/src/config/server.rs`, add two fields to `Server`:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub google_client_id: String,
    pub redis: RedisConfig,
    pub jwt_secret: SecretString,
    pub compress: bool,
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: Vec<String>,
    #[serde(default)]
    pub cookie_secure: bool,
    #[serde(default = "default_app_base_url")]
    pub app_base_url: String,
    #[serde(default)]
    pub smtp: SmtpConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub enable_docs: bool,
    /// Shared secret injected by Cloudflare via X-CF-Origin-Secret header.
    /// When set, requests missing this header are rejected with 403.
    #[serde(default)]
    pub cf_origin_secret: Option<String>,
    /// Cookie domain (e.g. `.yourdomain.com`). When set, auth cookies carry
    /// `Domain=<value>` so they work across subdomains.
    #[serde(default)]
    pub cookie_domain: Option<String>,
}
```

Update `Default for Server` to include the new fields:

```rust
impl Default for Server {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            log_level: "debug".to_string(),
            google_client_id: String::new(),
            redis: RedisConfig::default(),
            jwt_secret: SecretString::from(""),
            compress: true,
            allowed_origins: default_allowed_origins(),
            cookie_secure: false,
            app_base_url: default_app_base_url(),
            smtp: SmtpConfig::default(),
            metrics: MetricsConfig::default(),
            enable_docs: false,
            cf_origin_secret: None,
            cookie_domain: None,
        }
    }
}
```

- [x] **Step 4: Add cookie_domain to CookieSettings**

Update the `CookieSettings` struct at the bottom of `server/src/config/server.rs`:

```rust
#[derive(Clone)]
pub struct CookieSettings {
    pub secure: bool,
    pub domain: Option<String>,
}
```

- [x] **Step 5: Update Database::new() for env vars and add url field**

In `server/src/config/database.rs`, replace the entire file:

```rust
use config::{Config, ConfigError, File};
use secrecy::SecretString;
use serde::Deserialize;

const DATABASE_FILE: &str = "database.toml";

#[allow(clippy::struct_field_names)]
#[derive(Deserialize, Clone)]
pub struct Database {
    pub user: String,
    pub pass: SecretString,
    pub host: String,
    pub port: String,
    pub database: String,
    /// Full database URL (e.g. Neon or Cloud SQL connection string).
    /// When set, individual fields are ignored.
    #[serde(default)]
    pub url: Option<SecretString>,
}

impl Database {
    /// # Errors
    /// Returns `ConfigError` if config is invalid
    pub fn new() -> std::result::Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name(DATABASE_FILE).required(false))
            .add_source(
                config::Environment::with_prefix("DATABASE")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()
    }
}
```

- [x] **Step 6: Verify compile**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors (some `unused field` warnings for `url` are acceptable — they disappear in the next task).

- [x] **Step 7: Commit**

```bash
git add server/src/config/server.rs server/src/config/database.rs
git commit -m "feat: support env var config override for Cloud Run (no config files required)"
```

---

## Task 3: Redis TLS + Full-URL Support

*(Retitled 2026-09-22 — was "Upstash Redis TLS Support". The work is vendor-neutral and stands
whatever vendor is chosen; every managed Redis hands out a `rediss://` string.)*

Managed Redis requires `rediss://` (TLS). `Cache::new()` builds only `redis://`. Add
`Cache::from_url()` and wire it in `main.rs`.

**Corrections applied 2026-09-14:**

1. **The `redis` crate had no TLS feature at all.** It was declared
   `features = ["aio", "tokio-comp"]`, so `rediss://` failed with
   `InvalidClientConfig: "can't connect with TLS, the feature is not enabled"`
   *before any network I/O* — the whole point of this task would have panicked at
   startup on first deploy. Fixed by adding `tokio-native-tls-comp` (native-tls to
   match sqlx and lettre, rather than pulling rustls in as a second TLS stack).
   Locked in by `cache::tls_support::rediss_scheme_is_supported_by_the_enabled_feature_set`.
2. **There are three Redis consumers, not one.** `main.rs` also builds
   `RedisPubSub` (co-editor SSE fan-out) and `PresenceService`, both of which
   assembled their own `redis://` URL from host/port/password. Honouring `redis.url`
   in `Cache` alone would have left those two dialling `127.0.0.1:6379` and
   panicking at startup, taking co-editor sharing down. All three now take a URL;
   `RedisConfig::connection_url()` resolves it once in `main.rs`.
   `RedisPubSub` additionally stored host/port/password to rebuild the URL for each
   subscriber reconnect — it now stores the resolved URL as a `SecretString`.

**Files:**
- Modify: `server/src/cache.rs`
- Modify: `server/src/main.rs`

- [x] **Step 1: Add Cache::from_url() constructor**

In `server/src/cache.rs`, add a new constructor after `Cache::new()`:

```rust
/// Connect using a full Redis URL (e.g. `rediss://...` for Upstash TLS).
///
/// # Errors
/// Fails if the URL is invalid or the connection fails.
pub async fn from_url(url: &str) -> RedisResult<Self> {
    let client = Client::open(url)?;
    let connection = client.get_multiplexed_async_connection().await?;
    Ok(Self { connection })
}
```

- [x] **Step 2: Update main.rs to use from_url when redis.url is set**

Read the current `Cache::new` call in `server/src/main.rs` (look for the line starting `let cache = Cache::new`). Replace it with:

```rust
let cache = if let Some(url) = &settings.redis.url {
    Cache::from_url(url)
        .await
        .expect("Failed to connect to Redis via URL")
} else {
    Cache::new(
        &settings.redis.host,
        settings.redis.port,
        &settings.redis.password,
        settings.redis.db,
    )
    .await
    .expect("Failed to initialize Redis cache")
};
```

- [x] **Step 3: Verify compile**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors.

- [x] **Step 4: Write a test for from_url**

In `server/src/cache.rs` inside the `#[cfg(test)] mod tests` block, add:

```rust
#[tokio::test]
async fn from_url_connects_successfully() {
    let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "aegyptvault.local".to_owned());
    let port = std::env::var("REDIS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(2435_u16);
    let url = format!("redis://{host}:{port}");
    let cache = Cache::from_url(&url).await.expect("from_url must connect");
    // Sanity-check: set + get round-trips via this connection
    let key = "test:from_url:v";
    cache.set(key, &1_u8, 60).await.unwrap();
    let val: Option<u8> = cache.get(key).await.unwrap();
    cache.delete(key).await.unwrap();
    assert_eq!(val, Some(1));
}
```

- [x] **Step 5: Run tests**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server cache
```

Expected: all cache tests pass (including new `from_url` test if Redis test server is available).

- [x] **Step 6: Commit**

```bash
git add server/src/cache.rs server/src/main.rs
git commit -m "feat: add Cache::from_url() for Upstash TLS (rediss://) support"
```

---

## Task 4: Cloudflare Origin Secret Middleware

Requests not routed through Cloudflare are rejected with 403. The secret is injected at deploy time from GCP Secret Manager.

**Files:**
- Create: `server/src/middleware/cloudflare.rs`
- Modify: `server/src/middleware.rs`
- Modify: `server/src/server.rs`

- [x] **Step 1: Write the middleware**

Create `server/src/middleware/cloudflare.rs`:

```rust
use actix_web::{
    Error,
    body::BoxBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::{LocalBoxFuture, Ready, err, ok};
use std::rc::Rc;

pub struct CloudflareOrigin {
    secret: String,
}

impl CloudflareOrigin {
    #[must_use]
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

impl<S> Transform<S, ServiceRequest> for CloudflareOrigin
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = CloudflareOriginMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(CloudflareOriginMiddleware {
            service: Rc::new(service),
            secret: self.secret.clone(),
        })
    }
}

pub struct CloudflareOriginMiddleware<S> {
    service: Rc<S>,
    secret: String,
}

impl<S> Service<ServiceRequest> for CloudflareOriginMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let header_val = req
            .headers()
            .get("X-CF-Origin-Secret")
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned);

        let secret = self.secret.clone();
        let svc = Rc::clone(&self.service);

        Box::pin(async move {
            if header_val.as_deref() != Some(secret.as_str()) {
                let response = actix_web::HttpResponse::Forbidden()
                    .json(serde_json::json!({"error": "forbidden"}));
                return Ok(req.into_response(response.map_into_boxed_body()));
            }
            svc.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{App, HttpResponse, get, test, web};
    use super::*;

    #[get("/ping")]
    async fn ping() -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    #[actix_web::test]
    async fn missing_header_returns_403() {
        let app = test::init_service(
            App::new()
                .wrap(CloudflareOrigin::new("secret123".to_string()))
                .service(ping),
        )
        .await;
        let req = test::TestRequest::get().uri("/ping").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    #[actix_web::test]
    async fn wrong_secret_returns_403() {
        let app = test::init_service(
            App::new()
                .wrap(CloudflareOrigin::new("secret123".to_string()))
                .service(ping),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/ping")
            .insert_header(("X-CF-Origin-Secret", "wrong"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    #[actix_web::test]
    async fn correct_secret_passes_through() {
        let app = test::init_service(
            App::new()
                .wrap(CloudflareOrigin::new("secret123".to_string()))
                .service(ping),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/ping")
            .insert_header(("X-CF-Origin-Secret", "secret123"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
    }
}
```

- [x] **Step 2: Register the module**

Edit `server/src/middleware.rs`:

```rust
pub mod auth;
pub mod cloudflare;
```

- [x] **Step 3: Add futures-util to Cargo.toml**

Check if `futures-util` is already a dependency:

```bash
grep "futures" server/Cargo.toml
```

If not present, add to `server/Cargo.toml` under `[dependencies]`:

```toml
futures-util = "0.3"
```

- [x] **Step 4: Wire the middleware in server.rs conditionally**

In `server/src/server.rs`, add to the imports:

```rust
use crate::middleware::cloudflare::CloudflareOrigin;
```

In the `start()` function, after `let cookie_settings = CookieSettings { ... }`, add:

```rust
let cf_origin_secret = config.cf_origin_secret.clone();
```

In the `HttpServer::new(move || { ... })` closure, add the CF middleware wrap conditionally after the `cors` wrap. Actix middleware wraps outermost-first (last `.wrap()` call executes first):

```rust
.wrap(Condition::new(
    cf_origin_secret.is_some(),
    cf_origin_secret
        .clone()
        .map(CloudflareOrigin::new)
        .unwrap_or_else(|| CloudflareOrigin::new(String::new())),
))
```

> **Note:** `actix_web::middleware::Condition` requires the inner type to impl `Transform` regardless of the condition flag. A simpler pattern — use an `Option` wrap via a helper — or just always construct the middleware but with an empty secret that behaves as "disabled". Add this logic to `CloudflareOrigin`:

Update `CloudflareOriginMiddleware::call` to skip the check when the secret is empty:

```rust
fn call(&self, req: ServiceRequest) -> Self::Future {
    // When secret is empty the middleware is disabled (local dev without CF)
    if self.secret.is_empty() {
        let svc = Rc::clone(&self.service);
        return Box::pin(async move { svc.call(req).await });
    }

    let header_val = req
        .headers()
        .get("X-CF-Origin-Secret")
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);

    let secret = self.secret.clone();
    let svc = Rc::clone(&self.service);

    Box::pin(async move {
        if header_val.as_deref() != Some(secret.as_str()) {
            let response = actix_web::HttpResponse::Forbidden()
                .json(serde_json::json!({"error": "forbidden"}));
            return Ok(req.into_response(response.map_into_boxed_body()));
        }
        svc.call(req).await
    })
}
```

Then in `server.rs` always wrap:

```rust
let cf_secret = config.cf_origin_secret.clone().unwrap_or_default();
```

And in the closure:

```rust
.wrap(CloudflareOrigin::new(cf_secret.clone()))
```

Place this `.wrap()` just after the `.wrap(cors)` line so it executes before CORS.

- [x] **Step 5: Update CookieSettings construction in server.rs**

Replace the `CookieSettings` construction:

```rust
let cookie_settings = CookieSettings {
    secure: config.cookie_secure,
    domain: config.cookie_domain.clone(),
};
```

- [x] **Step 6: Verify compile**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors.

- [x] **Step 7: Run middleware tests**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server middleware::cloudflare
```

Expected: 3 tests pass.

- [x] **Step 8: Commit**

```bash
git add server/src/middleware/cloudflare.rs server/src/middleware.rs server/src/server.rs
git commit -m "feat: add Cloudflare origin secret middleware (empty secret = disabled)"
```

---

## Task 5: Cookie Domain + SameSite::Lax

Cookies need `Domain=.yourdomain.com` and `SameSite=Lax` to work across subdomains behind Cloudflare.

**Files:**
- Modify: `server/src/controllers/auth.rs`

**Corrections applied 2026-09-14:**

1. **The plan's stated reason for `SameSite=Lax` is wrong.** Lax is *not* needed for
   subdomain auth — `app.example.com` and `api.example.com` are same-site under any
   SameSite value; sharing cookies across them is what `Domain` does. The real reason
   to leave `Strict` is cross-site *top-level navigation*: returning from Stripe
   Checkout, and opening a co-editor invitation link from an email client. Under
   `Strict` the user lands logged out in both. Lax still withholds cookies on
   cross-site POST/PUT/DELETE, so CSRF protection for mutations is unchanged — worth
   stating explicitly because `.claude/memory/feedback_httponly_cookie_migration.md`
   records `Strict` as a deliberate anti-CSRF choice.
2. **There are four cookie sites, not two.** Besides `build_auth_cookie` and
   `build_refresh_cookie` there is `clear_refresh_cookie` *and* an inline removal
   cookie built directly in the `logout` handler. All four now go through one
   `build_cookie` helper — a removal cookie only clears the original when `Domain`
   and `Path` match, so a missed site would break logout on the deployed domain
   while still passing locally (where `domain` is `None` on both sides).

- [x] **Step 1: Find the cookie builder functions**

The functions are `build_auth_cookie` and `build_refresh_cookie` in `server/src/controllers/auth.rs`. Read around line 76 to locate them.

- [x] **Step 2: Update build_auth_cookie**

Replace `build_auth_cookie`:

```rust
#[must_use]
pub fn build_auth_cookie(token: String, cookie_settings: &CookieSettings) -> Cookie<'static> {
    let mut builder = Cookie::build("auth_token", token)
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/api/")
        .max_age(Duration::minutes(30))
        .secure(cookie_settings.secure);
    if let Some(domain) = &cookie_settings.domain {
        builder = builder.domain(domain.clone());
    }
    builder.finish()
}
```

- [x] **Step 3: Find and update build_refresh_cookie**

Search for `build_refresh_cookie` in the file. Apply the same pattern:

```rust
#[must_use]
pub fn build_refresh_cookie(token: String, cookie_settings: &CookieSettings) -> Cookie<'static> {
    let mut builder = Cookie::build("refresh_token", token)
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/api/auth/refresh")
        .max_age(Duration::days(30))
        .secure(cookie_settings.secure);
    if let Some(domain) = &cookie_settings.domain {
        builder = builder.domain(domain.clone());
    }
    builder.finish()
}
```

> **Note:** Confirm the path for `build_refresh_cookie` matches the current code before replacing. Read the function body first.

- [x] **Step 4: Check for logout cookie clearing**

Search for `Cookie::build` or `build_*cookie` calls used to clear cookies at logout. They also need `SameSite::Lax` and `domain` applied. The pattern is typically a zero/negative `max_age`. Apply the same builder pattern to any clearing cookies.

- [x] **Step 5: Compile + test**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server
```

Expected: all existing tests pass.

- [x] **Step 6: Commit**

```bash
git add server/src/controllers/auth.rs
git commit -m "fix: cookies use SameSite=Lax and support optional domain for subdomain auth"
```

---

## Task 6: Update config.toml.dist

**Files:**
- Modify: `config.toml.dist`

**Correction applied 2026-09-14:** the replacement block below is stale — it predates
`trust_proxy_header`, `[app]`, `[stripe]`, `[cache]`, `[limits]` and `[sharing]`. **Add**
the three new keys; do not paste it over the file. `database.toml.dist` also gained a
commented `url` key, which the plan omits.

- [x] **Step 1: Add new fields to config.toml.dist**

Replace the entire `config.toml.dist` with:

```toml
host = "127.0.0.1"
port = 8080
log_level = "debug"
google_client_id = ""
google_client_secret = ""
jwt_secret = ""
compress = true
cookie_secure = false
# Set to true only in development — exposes full API surface at /swagger-ui/
enable_docs = false

# Cookie domain for cross-subdomain auth (e.g. ".yourdomain.com").
# Leave unset in local dev.
# cookie_domain = ".yourdomain.com"

# Cloudflare origin secret — rejects requests not routed through Cloudflare.
# Leave unset in local dev (middleware disabled when empty/unset).
# cf_origin_secret = ""

[redis]
host = "127.0.0.1"
port = 6379
password = ""
db = 0
# Upstash TLS URL — overrides host/port/password when set.
# url = "rediss://..."

app_base_url = "http://localhost:5173"

[smtp]
host = ""
port = 587
username = ""
password = ""
from_address = "noreply@example.com"

[server.metrics]
# Expose Prometheus metrics on an HTTP listener. Bind to loopback so the
# endpoint is only reachable from other processes on the same host (or
# from Docker via host.docker.internal).
enabled = true
host = "127.0.0.1"
port = 9090
```

- [x] **Step 2: Commit**

```bash
git add config.toml.dist
git commit -m "docs: add cf_origin_secret, cookie_domain, redis.url to config.toml.dist"
```

---

## Task 7: Frontend nginx → envsubst Template

The frontend nginx config has `http://server:8080` hardcoded (Docker Compose hostname). In Cloud Run, the backend URL changes per environment. Replace with an envsubst template so one image works for both staging and production.

**Files:**
- Rename: `frontend/nginx.conf` → `frontend/nginx.conf.template`
- Modify: `frontend/Dockerfile`

**Corrections applied 2026-09-14:**

1. **The template below is a rewrite, not a port.** The real `nginx.conf` carries the
   SSE proxy settings (H-14), the `add_header` inheritance workaround, the security
   headers, and the `/invite/` and `/api/invitations/` location blocks. Convert the
   existing file by replacing the two `http://server:8080` occurrences with
   `${BACKEND_URL}`; do not swap in the simplified version.
2. **`/etc/nginx/conf.d` must be chowned to `nginx`.** The image runs as `USER nginx`,
   and the entrypoint writes the substituted config into that directory as that user.
   Without the chown the container fails to start.
3. **`docker-compose.yml` needs `BACKEND_URL` too.** Its frontend service relied on the
   hardcoded `http://server:8080`; templating it without adding an `environment:` entry
   breaks local compose, with nginx refusing to start on `proxy_pass /;`.

- [x] **Step 1: Rename and update nginx.conf to a template**

Create `frontend/nginx.conf.template` with:

```nginx
server {
    listen 80;
    server_name localhost;
    root /usr/share/nginx/html;
    index index.html;

    # Proxy API requests to the backend — BACKEND_URL is substituted at container startup
    location /api/ {
        proxy_pass ${BACKEND_URL}/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # SPA routing — all unmatched paths serve index.html so vue-router handles them
    location / {
        try_files $uri $uri/ /index.html;
    }

    # Cache-bust hashed assets aggressively
    location ~* \.(js|css|woff2?)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # Don't cache the root HTML (it references hashed assets)
    location = /index.html {
        add_header Cache-Control "no-cache";
    }
}
```

Delete the old `frontend/nginx.conf`:

```bash
git rm frontend/nginx.conf
```

- [x] **Step 2: Update frontend/Dockerfile**

Replace the entire `frontend/Dockerfile`:

```dockerfile
FROM node:22-alpine AS builder

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY . .

# No build args. Public bootstrap config (Google OAuth client ID, etc.) is
# delivered at runtime by the backend via GET /api/public-config — see
# server/src/controllers/public_config.rs and frontend/src/services/publicConfig.ts.

RUN npm run build

# ---------------------------------------------------------------------------
FROM nginx:alpine AS runtime

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf.template /etc/nginx/templates/default.conf.template

# Run nginx as non-root (nginx:alpine already sets up the nginx user)
RUN chown -R nginx:nginx /usr/share/nginx/html && \
    chown -R nginx:nginx /var/cache/nginx && \
    chown -R nginx:nginx /var/log/nginx && \
    touch /var/run/nginx.pid && \
    chown -R nginx:nginx /var/run/nginx.pid

USER nginx

EXPOSE 80

# nginx:alpine official image automatically runs envsubst on files in
# /etc/nginx/templates/ before starting nginx. BACKEND_URL must be set
# at container startup (Cloud Run env var or -e flag in docker run).
CMD ["nginx", "-g", "daemon off;"]
```

> **Why `/etc/nginx/templates/`?** The official `nginx:alpine` image ships a Docker entrypoint that automatically runs `envsubst` on every `*.template` file in `/etc/nginx/templates/` before starting nginx. No custom entrypoint script needed.

- [x] **Step 3: Test the Docker build locally**

```bash
cd frontend
docker build -t frontend-test .
docker run --rm -e BACKEND_URL=http://host.docker.internal:8080 -p 8081:80 frontend-test
```

In another terminal:

```bash
docker exec $(docker ps -q -f ancestor=frontend-test) cat /etc/nginx/conf.d/default.conf
```

Verify `${BACKEND_URL}` has been replaced with `http://host.docker.internal:8080`.

- [x] **Step 4: Commit**

```bash
git add frontend/nginx.conf.template frontend/Dockerfile
git commit -m "feat: nginx envsubst template for BACKEND_URL (single image, both environments)"
```

---

## Task 8: Database URL Support in main.rs

When `database.url` is set (Cloud Run uses Neon or Cloud SQL connection string), use it directly instead of building a DSN from individual fields.

**Files:**
- Modify: `server/src/main.rs`

**Correction applied 2026-09-14:** the DSN is not built in `main.rs` — there is no
`PgPoolOptions` call there. `main.rs` passes `db_config` to the mappers, and
`mappers/database.rs::Database::new` builds the connection string. Applying the change
there covers all eight pool constructions at once instead of one.

- [x] **Step 1: Read the current database pool creation in main.rs**

Find the line in `server/src/main.rs` that calls `PgPoolOptions` or `sqlx::postgres::PgPool` / `PgPoolOptions::new()`.

- [x] **Step 2: Update pool creation to use url field**

Locate the pool connection string construction. It should look something like:

```rust
let connection_string = format!(
    "postgres://{}:{}@{}:{}/{}",
    db_config.user,
    db_config.pass.expose_secret(),
    db_config.host,
    db_config.port,
    db_config.database
);
```

Replace it with:

```rust
use secrecy::ExposeSecret as _;
let connection_string = db_config.url
    .as_ref()
    .map(|u| u.expose_secret().to_owned())
    .unwrap_or_else(|| format!(
        "postgres://{}:{}@{}:{}/{}",
        db_config.user,
        db_config.pass.expose_secret(),
        db_config.host,
        db_config.port,
        db_config.database
    ));
```

- [x] **Step 3: Verify compile**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors.

- [x] **Step 4: Commit**

```bash
git add server/src/main.rs
git commit -m "feat: use database.url when set (Neon/Cloud SQL connection strings)"
```

---

## Task 9: GCP Infrastructure Setup

These are one-time manual steps in the GCP console / CLI. Complete them before writing the deploy workflow.

**Prerequisites:** GCP project created with billing enabled, `gcloud` CLI installed and authenticated.

- [ ] **Step 1: Enable required APIs**

```bash
gcloud services enable \
  run.googleapis.com \
  sqladmin.googleapis.com \
  artifactregistry.googleapis.com \
  secretmanager.googleapis.com \
  iam.googleapis.com \
  iamcredentials.googleapis.com \
  --project=YOUR_PROJECT_ID
```

- [ ] **Step 2: Create Artifact Registry repository**

```bash
gcloud artifacts repositories create anime-calendar \
  --repository-format=docker \
  --location=europe-west1 \
  --project=YOUR_PROJECT_ID
```

- [ ] **Step 3: Create Cloud SQL instance (production)**

```bash
gcloud sql instances create anime-calendar-prod \
  --database-version=POSTGRES_17 \
  --tier=db-f1-micro \
  --region=europe-west1 \
  --storage-auto-increase \
  --backup-start-time=03:00 \
  --project=YOUR_PROJECT_ID
```

Create the database and user:

```bash
gcloud sql databases create anime_calendar --instance=anime-calendar-prod --project=YOUR_PROJECT_ID
gcloud sql users create anime_user --instance=anime-calendar-prod --password=STRONG_PASSWORD --project=YOUR_PROJECT_ID
```

- [ ] **Step 4: Create service accounts**

```bash
# Runtime SA (used by Cloud Run containers)
gcloud iam service-accounts create cr-runtime \
  --display-name="Cloud Run Runtime" \
  --project=YOUR_PROJECT_ID

# Staging deploy SA (used by GitHub Actions staging job)
gcloud iam service-accounts create github-staging \
  --display-name="GitHub Actions Staging" \
  --project=YOUR_PROJECT_ID

# Production deploy SA (used by GitHub Actions prod job)
gcloud iam service-accounts create github-prod \
  --display-name="GitHub Actions Production" \
  --project=YOUR_PROJECT_ID
```

- [ ] **Step 5: Grant IAM permissions**

```bash
PROJECT=YOUR_PROJECT_ID
PROJECT_NUMBER=$(gcloud projects describe $PROJECT --format='value(projectNumber)')

# cr-runtime: read secrets + connect to Cloud SQL
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:cr-runtime@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor"
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:cr-runtime@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/cloudsql.client"

# github-staging: push images + deploy Cloud Run + read secrets + access Cloud SQL
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:github-staging@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/artifactregistry.writer"
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:github-staging@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/run.developer"
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:github-staging@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor"
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:github-staging@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/cloudsql.client"
# Allow github-staging to set the service account on Cloud Run (act as cr-runtime)
gcloud iam service-accounts add-iam-policy-binding \
  cr-runtime@$PROJECT.iam.gserviceaccount.com \
  --member="serviceAccount:github-staging@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"

# github-prod: deploy Cloud Run + read secrets + access Cloud SQL (NO image push)
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:github-prod@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/run.developer"
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:github-prod@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/secretmanager.secretAccessor"
gcloud projects add-iam-policy-binding $PROJECT \
  --member="serviceAccount:github-prod@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/cloudsql.client"
gcloud iam service-accounts add-iam-policy-binding \
  cr-runtime@$PROJECT.iam.gserviceaccount.com \
  --member="serviceAccount:github-prod@$PROJECT.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"
```

- [ ] **Step 6: Set up Workload Identity Federation**

```bash
# Create WIF pool
gcloud iam workload-identity-pools create github-pool \
  --location=global \
  --project=$PROJECT

# Create WIF provider
gcloud iam workload-identity-pools providers create-oidc github-provider \
  --location=global \
  --workload-identity-pool=github-pool \
  --issuer-uri=https://token.actions.githubusercontent.com \
  --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository" \
  --attribute-condition="assertion.repository == 'YOUR_GITHUB_USERNAME/anime-calendar'" \
  --project=$PROJECT

# Bind github-staging SA to the pool (for staging deploy job)
gcloud iam service-accounts add-iam-policy-binding \
  github-staging@$PROJECT.iam.gserviceaccount.com \
  --member="principalSet://iam.googleapis.com/projects/$PROJECT_NUMBER/locations/global/workloadIdentityPools/github-pool/attribute.repository/YOUR_GITHUB_USERNAME/anime-calendar" \
  --role="roles/iam.workloadIdentityUser"

# Bind github-prod SA to the pool (for prod deploy job)
gcloud iam service-accounts add-iam-policy-binding \
  github-prod@$PROJECT.iam.gserviceaccount.com \
  --member="principalSet://iam.googleapis.com/projects/$PROJECT_NUMBER/locations/global/workloadIdentityPools/github-pool/attribute.repository/YOUR_GITHUB_USERNAME/anime-calendar" \
  --role="roles/iam.workloadIdentityUser"
```

Save the WIF provider resource name for use in GitHub Secrets:

```bash
gcloud iam workload-identity-pools providers describe github-provider \
  --location=global \
  --workload-identity-pool=github-pool \
  --project=$PROJECT \
  --format="value(name)"
```

- [ ] **Step 7: Record outputs**

Note down (will be needed in later tasks):
- GCP project ID
- WIF provider resource name
- Service account emails
- Artifact Registry URL: `europe-west1-docker.pkg.dev/YOUR_PROJECT_ID/anime-calendar`
- Cloud SQL instance connection name: `YOUR_PROJECT_ID:europe-west1:anime-calendar-prod`

---

## Task 10: External Services Setup

> **BLOCKED on the two open decisions (see *Decisions of record*).** The steps below were
> written against the superseded April choices. Do **not** execute them as written — they would
> create an Upstash database this app's `SUBSCRIBE`-per-session shape is wrong for, and split
> staging and production across different vendors, which violates the owner's stated constraint.
> Settle Postgres and Redis first, then rewrite Steps 1–3 for the chosen vendors. Everything
> from Step 4 (Cloudflare) onward is vendor-independent and can proceed.

- [ ] **Step 1: Provision Postgres — SUPERSEDED, rewrite after the decision**

Whatever is chosen: PostgreSQL 17, **`europe-west1` or as close as the vendor offers**, and the
*same product* in staging and production. Capture the full connection string including any
required query parameters (`?sslmode=require`); `mappers/database.rs::connection_string` passes
`database.url` through verbatim precisely so those are not dropped. Size it against Task 15's
arithmetic, not against a guess.

<details><summary>Original (superseded) text</summary>

1. Go to neon.tech → create account → create project named `anime-calendar-staging`
2. Select PostgreSQL 17, region matching your GCP region
3. Note the connection string: `postgres://...@ep-xxx.neon.tech/neondb?sslmode=require`

</details>

- [ ] **Step 2: Provision Redis — SUPERSEDED, rewrite after the decision**

Whatever is chosen: same vendor and product in both environments, `rediss://` TLS endpoint, in
or near `europe-west1`. **Before committing, run a live `SUBSCRIBE` against the real endpoint**
and confirm a published message arrives — this is the check that was never done for Upstash, and
a docs page saying "pub/sub supported" is not it. Size against Task 16: if the multiplexing
refactor has landed, the budget is ~4 connections flat rather than scaling per viewer.

<details><summary>Original (superseded) text</summary>

1. Go to upstash.com → create account
2. Create database named `anime-calendar-staging` (free tier, region matching GCP)
3. Create database named `anime-calendar-prod` (pay-per-use, same region)
4. From each database page, copy the **TLS URL**: `rediss://default:...@...upstash.io:6379`

</details>

- [ ] **Step 3: Run the initial migration**

```bash
DATABASE_URL="<the connection string from Step 1>" \
  cargo sqlx migrate run --source migrations
```

Then **verify it applied** rather than trusting the exit code — `sqlx migrate info` should list
all 21 as applied. A migration run against the wrong database exits 0 just as happily.

- [ ] **Step 4: Configure Cloudflare**

1. Add your domain to Cloudflare (or register via Cloudflare Registrar)
2. Create DNS records:
   - `yourdomain.com` → Proxied CNAME to Cloud Run frontend (prod) URL
   - `api.yourdomain.com` → Proxied CNAME to Cloud Run backend (prod) URL
   - `staging.yourdomain.com` → Proxied CNAME to Cloud Run frontend (staging) URL
   - `staging-api.yourdomain.com` → Proxied CNAME to Cloud Run backend (staging) URL
3. Create Transform Rule: add header `X-CF-Origin-Secret: <GENERATED_SECRET>` to all requests
4. Generate the CF origin secret: `openssl rand -hex 32`

---

## Task 11: GCP Secret Manager Population

- [ ] **Step 1: Create all secrets**

Replace values with real credentials:

**The two URL values below are placeholders pending the vendor decisions** — the secret *names*
and the `gcloud` invocations are final, only the strings change.

```bash
PROJECT=YOUR_PROJECT_ID

# Staging secrets
echo -n "<STAGING_POSTGRES_URL — from Task 10 Step 1>" | \
  gcloud secrets create staging--database-url --data-file=- --project=$PROJECT

echo -n "<STAGING_REDIS_URL — rediss://…, from Task 10 Step 2>" | \
  gcloud secrets create staging--redis-url --data-file=- --project=$PROJECT

echo -n "$(openssl rand -hex 32)" | \
  gcloud secrets create staging--jwt-secret --data-file=- --project=$PROJECT

# Production secrets
# If the choice lands on Cloud SQL, the Unix-socket form below is correct and needs
# --add-cloudsql-instances on the Cloud Run service (no Serverless VPC connector, and
# therefore no ~$8/mo connector fee). Any other vendor takes a normal TCP URL.
echo -n "postgres://anime_user:STRONG_PASSWORD@/anime_calendar?host=/cloudsql/YOUR_PROJECT_ID:europe-west1:anime-calendar-prod" | \
  gcloud secrets create prod--database-url --data-file=- --project=$PROJECT

echo -n "<PROD_REDIS_URL — rediss://…, same vendor as staging>" | \
  gcloud secrets create prod--redis-url --data-file=- --project=$PROJECT

echo -n "$(openssl rand -hex 32)" | \
  gcloud secrets create prod--jwt-secret --data-file=- --project=$PROJECT

# Shared secrets
echo -n "YOUR_GOOGLE_CLIENT_ID" | \
  gcloud secrets create shared--google-client-id --data-file=- --project=$PROJECT

echo -n "YOUR_GOOGLE_CLIENT_SECRET" | \
  gcloud secrets create shared--google-client-secret --data-file=- --project=$PROJECT

echo -n "YOUR_CF_ORIGIN_SECRET" | \
  gcloud secrets create shared--cf-origin-secret --data-file=- --project=$PROJECT
```

> **Secret naming:** Using `--` as separator (avoids `/` which causes path issues in some tools). Adjust the deploy workflow to match this naming convention.

- [ ] **Step 2: Grant cr-runtime access to all secrets**

```bash
for secret in staging--database-url staging--redis-url staging--jwt-secret \
              prod--database-url prod--redis-url prod--jwt-secret \
              shared--google-client-id shared--google-client-secret shared--cf-origin-secret; do
  gcloud secrets add-iam-policy-binding $secret \
    --member="serviceAccount:cr-runtime@$PROJECT.iam.gserviceaccount.com" \
    --role="roles/secretmanager.secretAccessor" \
    --project=$PROJECT
done
```

---

## Task 12: GitHub Actions Deploy Workflow

**Files:**
- Create: `.github/workflows/deploy.yml`
- Modify: `.github/workflows/ci.yml` (add tag trigger)

- [ ] **Step 1: Add tag trigger to ci.yml**

In `.github/workflows/ci.yml`, update the `on:` section:

```yaml
on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]
```

- [ ] **Step 2: Create .github/workflows/deploy.yml**

```yaml
name: Deploy

on:
  push:
    branches: [main]
    tags: ['v*']

concurrency:
  group: deploy-${{ github.ref }}
  cancel-in-progress: false

permissions:
  contents: read
  id-token: write   # required for Workload Identity Federation

jobs:
  # ── Staging deploy (automatic on main push) ───────────────────────────────
  deploy-staging:
    name: Deploy → Staging
    runs-on: ubuntu-24.04
    if: github.ref == 'refs/heads/main'
    needs: []   # no CI dependency here — add `needs: [backend, frontend, openapi]`
                # once this file and ci.yml are in the same workflow file

    environment: staging

    env:
      PROJECT_ID: ${{ vars.GCP_PROJECT_ID }}
      REGION: europe-west1
      REGISTRY: europe-west1-docker.pkg.dev/${{ vars.GCP_PROJECT_ID }}/anime-calendar
      SHA: ${{ github.sha }}

    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd

      - name: Authenticate to GCP (staging SA)
        uses: google-github-actions/auth@6fc4af4b145ae7821d527454aa9bd537d1f2dc5f
        with:
          workload_identity_provider: ${{ secrets.WORKLOAD_IDENTITY_PROVIDER }}
          service_account: ${{ secrets.GCP_STAGING_SA }}

      - name: Set up gcloud CLI
        uses: google-github-actions/setup-gcloud@6189d56e4096ee891640bb02ac264be376592d6a

      - name: Configure Docker auth
        run: gcloud auth configure-docker europe-west1-docker.pkg.dev --quiet

      - name: Build and push backend image
        run: |
          docker build \
            --build-arg SQLX_OFFLINE=true \
            -t $REGISTRY/backend:$SHA \
            -f Dockerfile .
          docker push $REGISTRY/backend:$SHA

      - name: Build and push frontend image
        run: |
          # No build args — the frontend image is environment-agnostic.
          # Google OAuth client ID and any other public bootstrap config is
          # served by the backend at runtime via GET /api/public-config.
          docker build \
            -t $REGISTRY/frontend:$SHA \
            -f frontend/Dockerfile frontend/
          docker push $REGISTRY/frontend:$SHA

      - name: Read staging secrets from Secret Manager
        id: secrets
        run: |
          DB_URL=$(gcloud secrets versions access latest --secret="staging--database-url" --project=$PROJECT_ID)
          REDIS_URL=$(gcloud secrets versions access latest --secret="staging--redis-url" --project=$PROJECT_ID)
          JWT_SECRET=$(gcloud secrets versions access latest --secret="staging--jwt-secret" --project=$PROJECT_ID)
          GOOGLE_ID=$(gcloud secrets versions access latest --secret="shared--google-client-id" --project=$PROJECT_ID)
          GOOGLE_SECRET=$(gcloud secrets versions access latest --secret="shared--google-client-secret" --project=$PROJECT_ID)
          CF_SECRET=$(gcloud secrets versions access latest --secret="shared--cf-origin-secret" --project=$PROJECT_ID)
          echo "db_url=$DB_URL" >> $GITHUB_OUTPUT
          echo "redis_url=$REDIS_URL" >> $GITHUB_OUTPUT
          echo "jwt_secret=$JWT_SECRET" >> $GITHUB_OUTPUT
          echo "google_id=$GOOGLE_ID" >> $GITHUB_OUTPUT
          echo "google_secret=$GOOGLE_SECRET" >> $GITHUB_OUTPUT
          echo "cf_secret=$CF_SECRET" >> $GITHUB_OUTPUT

      - name: Run database migrations (staging)
        env:
          DATABASE_URL: ${{ steps.secrets.outputs.db_url }}
        run: |
          cargo install sqlx-cli --no-default-features --features postgres,native-tls
          sqlx migrate run --source migrations

      - name: Deploy backend to Cloud Run (staging)
        run: |
          gcloud run deploy anime-calendar-backend-staging \
            --image=$REGISTRY/backend:$SHA \
            --region=$REGION \
            --platform=managed \
            --service-account=cr-runtime@$PROJECT_ID.iam.gserviceaccount.com \
            --max-instances=2 \
            --memory=512Mi \
            --cpu=1 \
            --set-env-vars="HOST=0.0.0.0,PORT=8080,LOG_LEVEL=info,\
          ALLOWED_ORIGINS=https://staging.${{ vars.DOMAIN }},\
          COOKIE_SECURE=true,\
          COOKIE_DOMAIN=.${{ vars.DOMAIN }},\
          APP_BASE_URL=https://staging.${{ vars.DOMAIN }},\
          GOOGLE_CLIENT_ID=${{ steps.secrets.outputs.google_id }},\
          GOOGLE_CLIENT_SECRET=${{ steps.secrets.outputs.google_secret }},\
          JWT_SECRET=${{ steps.secrets.outputs.jwt_secret }},\
          REDIS__URL=${{ steps.secrets.outputs.redis_url }},\
          DATABASE__URL=${{ steps.secrets.outputs.db_url }},\
          CF_ORIGIN_SECRET=${{ steps.secrets.outputs.cf_secret }}" \
            --allow-unauthenticated \
            --project=$PROJECT_ID

      - name: Deploy frontend to Cloud Run (staging)
        run: |
          gcloud run deploy anime-calendar-frontend-staging \
            --image=$REGISTRY/frontend:$SHA \
            --region=$REGION \
            --platform=managed \
            --max-instances=2 \
            --memory=256Mi \
            --set-env-vars="BACKEND_URL=https://staging-api.${{ vars.DOMAIN }}" \
            --allow-unauthenticated \
            --project=$PROJECT_ID

      - name: Smoke test
        run: |
          sleep 10
          curl -sf https://staging.${{ vars.DOMAIN }}/api/health | grep '"status":"ok"'

  # ── Production deploy (tag-gated) ─────────────────────────────────────────
  deploy-production:
    name: Deploy → Production
    runs-on: ubuntu-24.04
    if: startsWith(github.ref, 'refs/tags/v')

    environment: production   # GitHub Environment approval gate

    env:
      PROJECT_ID: ${{ vars.GCP_PROJECT_ID }}
      REGION: europe-west1
      REGISTRY: europe-west1-docker.pkg.dev/${{ vars.GCP_PROJECT_ID }}/anime-calendar
      SHA: ${{ github.sha }}

    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd

      - name: Authenticate to GCP (prod SA)
        uses: google-github-actions/auth@6fc4af4b145ae7821d527454aa9bd537d1f2dc5f
        with:
          workload_identity_provider: ${{ secrets.WORKLOAD_IDENTITY_PROVIDER }}
          service_account: ${{ secrets.GCP_PROD_SA }}

      - name: Set up gcloud CLI
        uses: google-github-actions/setup-gcloud@6189d56e4096ee891640bb02ac264be376592d6a

      - name: Configure Docker auth
        run: gcloud auth configure-docker europe-west1-docker.pkg.dev --quiet

      - name: Read production secrets from Secret Manager
        id: secrets
        run: |
          DB_URL=$(gcloud secrets versions access latest --secret="prod--database-url" --project=$PROJECT_ID)
          REDIS_URL=$(gcloud secrets versions access latest --secret="prod--redis-url" --project=$PROJECT_ID)
          JWT_SECRET=$(gcloud secrets versions access latest --secret="prod--jwt-secret" --project=$PROJECT_ID)
          GOOGLE_ID=$(gcloud secrets versions access latest --secret="shared--google-client-id" --project=$PROJECT_ID)
          GOOGLE_SECRET=$(gcloud secrets versions access latest --secret="shared--google-client-secret" --project=$PROJECT_ID)
          CF_SECRET=$(gcloud secrets versions access latest --secret="shared--cf-origin-secret" --project=$PROJECT_ID)
          echo "db_url=$DB_URL" >> $GITHUB_OUTPUT
          echo "redis_url=$REDIS_URL" >> $GITHUB_OUTPUT
          echo "jwt_secret=$JWT_SECRET" >> $GITHUB_OUTPUT
          echo "google_id=$GOOGLE_ID" >> $GITHUB_OUTPUT
          echo "google_secret=$GOOGLE_SECRET" >> $GITHUB_OUTPUT
          echo "cf_secret=$CF_SECRET" >> $GITHUB_OUTPUT

      - name: Run database migrations (Cloud SQL)
        run: |
          # Download Cloud SQL Auth Proxy
          curl -o cloud-sql-proxy https://storage.googleapis.com/cloud-sql-connectors/cloud-sql-proxy/v2.14.1/cloud-sql-proxy.linux.amd64
          chmod +x cloud-sql-proxy
          # Start proxy in background (IAM auth via github-prod SA)
          ./cloud-sql-proxy --port=5433 $PROJECT_ID:$REGION:anime-calendar-prod &
          PROXY_PID=$!
          sleep 3
          # Run migrations via proxy
          DATABASE_URL="postgres://anime_user:${{ secrets.PROD_DB_PASSWORD }}@127.0.0.1:5433/anime_calendar" \
            cargo install sqlx-cli --no-default-features --features postgres,native-tls
          DATABASE_URL="postgres://anime_user:${{ secrets.PROD_DB_PASSWORD }}@127.0.0.1:5433/anime_calendar" \
            sqlx migrate run --source migrations
          kill $PROXY_PID

      - name: Save previous backend revision
        id: prev
        run: |
          PREV=$(gcloud run services describe anime-calendar-backend \
            --region=$REGION --project=$PROJECT_ID \
            --format='value(status.latestReadyRevisionName)' 2>/dev/null || echo "")
          echo "revision=$PREV" >> $GITHUB_OUTPUT

      - name: Deploy backend to Cloud Run (production)
        run: |
          gcloud run deploy anime-calendar-backend \
            --image=$REGISTRY/backend:$SHA \
            --region=$REGION \
            --platform=managed \
            --service-account=cr-runtime@$PROJECT_ID.iam.gserviceaccount.com \
            --max-instances=5 \
            --memory=512Mi \
            --cpu=1 \
            --add-cloudsql-instances=$PROJECT_ID:$REGION:anime-calendar-prod \
            --set-env-vars="HOST=0.0.0.0,PORT=8080,LOG_LEVEL=info,\
          ALLOWED_ORIGINS=https://${{ vars.DOMAIN }},\
          COOKIE_SECURE=true,\
          COOKIE_DOMAIN=.${{ vars.DOMAIN }},\
          APP_BASE_URL=https://${{ vars.DOMAIN }},\
          GOOGLE_CLIENT_ID=${{ steps.secrets.outputs.google_id }},\
          GOOGLE_CLIENT_SECRET=${{ steps.secrets.outputs.google_secret }},\
          JWT_SECRET=${{ steps.secrets.outputs.jwt_secret }},\
          REDIS__URL=${{ steps.secrets.outputs.redis_url }},\
          DATABASE__URL=${{ steps.secrets.outputs.db_url }},\
          CF_ORIGIN_SECRET=${{ steps.secrets.outputs.cf_secret }}" \
            --allow-unauthenticated \
            --project=$PROJECT_ID

      - name: Deploy frontend to Cloud Run (production)
        run: |
          gcloud run deploy anime-calendar-frontend \
            --image=$REGISTRY/frontend:$SHA \
            --region=$REGION \
            --platform=managed \
            --max-instances=5 \
            --memory=256Mi \
            --set-env-vars="BACKEND_URL=https://api.${{ vars.DOMAIN }}" \
            --allow-unauthenticated \
            --project=$PROJECT_ID

      - name: Smoke test
        id: smoke
        run: |
          sleep 10
          curl -sf https://${{ vars.DOMAIN }}/api/health | grep '"status":"ok"'

      - name: Rollback on failure
        if: failure() && steps.smoke.outcome == 'failure' && steps.prev.outputs.revision != ''
        run: |
          gcloud run services update-traffic anime-calendar-backend \
            --to-revisions=${{ steps.prev.outputs.revision }}=100 \
            --region=$REGION \
            --project=$PROJECT_ID
```

> **Action versions:** The `google-github-actions/auth` and `google-github-actions/setup-gcloud` actions above use commit hash pins. Verify the latest commit hashes at github.com/google-github-actions before using in production.

> **`needs` dependency:** The staging deploy job has `needs: []` with a comment. Once CI and deploy are confirmed working, add `needs: [backend, frontend, openapi]` referencing the ci.yml job IDs — or merge both files and use explicit `needs`.

- [ ] **Step 3: Add GitHub repository variables and secrets**

In GitHub → repo → Settings → Secrets and variables → Actions:

**Repository variables** (not secrets — not sensitive):
- `GCP_PROJECT_ID` — your GCP project ID
- `DOMAIN` — your domain (e.g. `yourdomain.com`)

**Repository secrets:**
- `WORKLOAD_IDENTITY_PROVIDER` — WIF provider resource name from Task 9
- `GCP_STAGING_SA` — `github-staging@YOUR_PROJECT_ID.iam.gserviceaccount.com`
- `GCP_PROD_SA` — `github-prod@YOUR_PROJECT_ID.iam.gserviceaccount.com`
- `PROD_DB_PASSWORD` — Cloud SQL user password

> The Google OAuth client ID is **not** a GitHub secret in this design — it lives in `shared/google-client-id` in GCP Secret Manager (already listed in the Secrets table) and is served to the SPA at runtime by the backend's `/api/public-config` endpoint. The CI job no longer needs it.

- [ ] **Step 4: Create GitHub Environments**

In GitHub → repo → Settings → Environments:

1. Create environment named `staging` (no approval required)
2. Create environment named `production`:
   - Required reviewers: add yourself
   - Deployment branches: Tags matching `v*`

- [ ] **Step 5: Commit the workflow**

```bash
git add .github/workflows/deploy.yml .github/workflows/ci.yml
git commit -m "ci: add staging + production deploy workflows (WIF auth, Cloud Run)"
```

---

## Task 13: First Deployment

- [ ] **Step 1: Verify all compile checks pass**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test --workspace
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -A clippy::missing_docs_in_private_items -A clippy::separated_literal_suffix -A clippy::implicit_return -A clippy::print_stderr -A clippy::exhaustive_enums -A clippy::exhaustive_structs -A clippy::single_char_lifetime_names -A clippy::missing_inline_in_public_items -A clippy::self_named_module_files -A clippy::wildcard_enum_match_arm -A clippy::pattern_type_mismatch -A clippy::std-instead-of-core
```

- [ ] **Step 2: Push to trigger staging deploy**

```bash
git push origin main
```

Watch the GitHub Actions run. Staging deploy should complete automatically.

- [ ] **Step 3: Verify staging**

```bash
curl https://staging.yourdomain.com/api/health
# Expected: {"status":"ok"}
```

- [ ] **Step 4: Create a production tag**

```bash
git tag v0.1.0
git push origin v0.1.0
```

Approve the production deploy in the GitHub Environment gate.

- [ ] **Step 5: Verify production**

```bash
curl https://yourdomain.com/api/health
# Expected: {"status":"ok"}
```

---

## Task 20: Settle the Migration Strategy (B-1)

**Added 2026-09-22.** The two documents disagree and the disagreement is substantive, not a slip.

> **Path correction, applied 2026-09-22 — this would have failed every deploy.** All three
> migration invocations in this plan said `--source server/migrations`. **That directory does
> not exist.** The 21 migrations live at **`./migrations`** at the repo root. Corrected in place
> at the three call sites (Task 10 Step 3, and both deploy jobs in Task 12).
>
> Verified the same day by applying all 21 in order to a fresh PostgreSQL 16 container: 21
> applied, 0 failures, 11 tables. So the migration set itself is sound from zero — the
> `schema.sql` drift described in B-1 is a property of that file alone, not of the migrations.

Readiness **B-1** records the decision as *"run `sqlx::migrate!` at server startup and drop the
`schema.sql` mount"*, on the grounds that migrations become the single source of truth so the
drift cannot recur, and that under Cloud Run there is no host to shell into. This plan instead
runs `sqlx migrate run` from a **GitHub Actions step** before each deploy (Tasks 12, lines
~1542 and ~1635).

Both work. The CI approach is arguably *better* under Cloud Run, because it runs exactly once
per deploy instead of racing across however many instances cold-start simultaneously — a race
`_sqlx_migrations` handles, but noisily. What it gives up is that a container started outside
the workflow (a local `docker run`, a manual `gcloud run deploy` of an older image) gets no
migration at all.

- [ ] **Step 1: Pick one and record why in `docs/deployment-readiness.md`**

Do not leave both documents asserting different things — that is how the `schema.sql` drift
happened in the first place.

- [ ] **Step 2: Whichever is chosen, drop the `schema.sql` mount**

`docker-compose.yml:10` still mounts `./schema.sql` into `/docker-entrypoint-initdb.d/`. It
defines 4 tables against 21 migrations, so a fresh `docker compose up` produces an April-era
database in which refresh, password reset, billing and all of co-editor sharing fail at runtime.
That is true regardless of which migration strategy wins, and it is a hard blocker for anyone
setting the project up fresh.

- [ ] **Step 3: If CI-driven, make a container started without it fail loudly**

Rather than serving traffic against a schema it cannot satisfy. `sqlx::migrate!` exposes the
applied-versions list without applying anything — a startup check that refuses to serve on a
mismatch gets the safety of the startup approach without the race.

---

## Task 14: Reconcile Loop → Cloud Scheduler

**Added 2026-09-22.** Absent from the original plan entirely, and the one item readiness §2
identified as the genuine serverless obstacle.

`services/reconcile.rs:440` spawns an in-process `tokio::time::interval` that fires hourly,
guarded by `pg_try_advisory_lock`. **Cloud Run throttles CPU between requests by default, so
that timer never fires** and Stripe subscription state silently stops reconciling. Nothing
errors; the drift just accumulates.

The codebase already anticipated this. `ReconcileConfig::interval_secs = 0` disables the loop
and is documented as *"used in tests and in deployments that don't want background work"*, and
`run_pass` is already `pub async fn` (`reconcile.rs:149`).

**Files:**
- Create: `server/src/controllers/internal.rs` — `POST /internal/reconcile`
- Modify: `server/src/controllers.rs`, `server/src/server.rs`

- [ ] **Step 1: Expose `run_pass` behind an authenticated endpoint**

`POST /internal/reconcile` calling `run_pass`. Keep the advisory lock — it stays the guard
against overlapping passes, now across instances rather than across threads. Per
`.claude/memory/feedback_singleton_loop_advisory_lock_outside_tested_fn.md` the
`pg_try_advisory_lock` belongs in a wrapper **outside** `run_pass`, not inside it, or parallel
tests contend on the global key.

- [ ] **Step 2: Protect it with platform OIDC, not a shared secret**

Cloud Scheduler signs requests with an OIDC token for a dedicated service account. Verify the
token rather than inventing another bearer secret. Note the Cloudflare origin middleware sits
**outermost** (`server.rs`), so a Scheduler request arriving directly at the `*.run.app` URL
will be refused with 403 before reaching the handler — either route Scheduler through the
public domain with the CF header, or exempt `/internal/` in the middleware. Decide explicitly;
both are defensible and the failure mode is a silent 403 in a log nobody reads.

- [ ] **Step 3: Set `RECONCILE__INTERVAL_SECS=0` in both deploy jobs**

Otherwise both mechanisms are live and they race for the same advisory lock.

- [ ] **Step 4: Create the Cloud Scheduler job**

Hourly, `europe-west1`, OIDC audience set to the backend service URL. Cloud Scheduler is free
for the first 3 jobs.

- [ ] **Step 5: Verify it actually fired**

A scheduler job that 403s looks identical to one that succeeded if you only read the job list.
Check the Scheduler job's *execution* status **and** grep the backend log for the pass's own
completion line.

---

## Task 15: Cap the Postgres Pools

**Added 2026-09-22.** Not in readiness §2, which budgeted Redis connections but not Postgres.

`server/src/main.rs` calls `Database::new` **15 times**, each building an independent `PgPool`.
Confirmed against sqlx 0.8.6 source (`sqlx-core-0.8.6/src/pool/options.rs:143-166`):
`max_connections: 10`, `min_connections: 0`, `idle_timeout: 10 min`, `max_lifetime: 30 min`,
`test_before_acquire: true`.

So idle settles near zero, but the **ceiling is 150 connections per instance**, and Task 12
deploys production with `--max-instances=5`. A `db-f1-micro` tops out around 25. This is a hard
constraint on the Postgres choice and it must be settled before Task 9 provisions anything.

**DONE 2026-09-22 — and done as the sharing refactor, not the cap, because the cap alone does
not fit the cheapest tier.**

- [x] **Step 1: Shared one pool rather than capping fifteen**

The plan said cap first and treat sharing as follow-up. That was wrong on both counts once the
blast radius was measured: `Mapper::new` is called **only from `main.rs`**, and all ten mappers
had an identical three-line constructor, so sharing cost ten mechanical edits. And capping alone
could not have worked — fifteen pools at even 2 connections each is 30, already past a
`db-f1-micro`'s ~25 before multiplying by `max-instances`.

`main.rs` now builds one `Database` and hands out clones (Arc-backed, genuinely cheap). Mapper
constructors take `Database` instead of `DatabaseConfig` and are no longer `async` or fallible.

A detail worth recording: four call sites carried comments describing their pool as *"Cheap
(Arc-backed)"*. They were not — each `Database::new` opened a fresh pool. **The comments asserted
the property the code lacked**, which is why the cost stayed invisible. They are true now.

- [x] **Step 2: `max_connections` is configurable, defaulting to 5, with the arithmetic written down**

`database.toml` gains `max_connections` (env: `DATABASE__MAX_CONNECTIONS`). Because there is now
exactly one pool, this number *is* the per-instance budget, and the formula simplifies to
`max_connections × max-instances + headroom ≤ server limit`. The default of 5 fits four
instances inside a `db-f1-micro` with five spare for migrations, the CLI and a `psql` session.
A test asserts both the default and that the arithmetic holds.

- [ ] **Step 3: If the choice lands on Neon, also set `idle_timeout`**

Still open, and still contingent on the Postgres decision. sqlx's 10-minute default outlives
Neon's 5-minute autosuspend. `test_before_acquire` already defaults to `true` and catches a
killed connection, so this is churn-avoidance rather than a correctness fix.

---

## Task 16: Multiplex the Redis Subscriber Connections

**Added 2026-09-22.** Not a deploy blocker — it removes a constraint rather than fixing a fault,
and it is worth doing whichever Redis vendor is chosen.

`server/src/redis_pubsub.rs:235-237` opens a dedicated TCP connection per channel:

```rust
let client = Client::open(url.expose_secret())...;
let mut pubsub = client.get_async_pubsub().await...;
pubsub.subscribe(channel).await...;
```

Redis pub/sub permits one connection to `SUBSCRIBE` to many channels. Multiplexing onto a
single shared subscriber connection with a demux by channel name takes readiness §2's budget
from `3 fixed + 2/calendar + 1/viewer` to roughly **4 flat**, which takes a free tier's
30-connection cap off the table as a sizing constraint.

**DONE 2026-09-22 — Steps 1–3. Step 4 is deliberately left open; see below.**

- [x] **Step 1: Replace per-channel connections with one shared subscriber**

`PubSub::split()` gives a `(PubSubSink, PubSubStream)` pair: the sink issues
`SUBSCRIBE`/`UNSUBSCRIBE` while the stream is being polled. One driver task owns the stream and
demultiplexes on `Msg::get_channel_name()` into the existing
`HashMap<String, Sender<String>>`, which is untouched as the plan intended.

Two things the vendored crate settled that the published docs did not. `PubSubSink::subscribe`
takes `impl ToRedisArgs` in **redis 1.0.3** (`aio/pubsub.rs:297`), not the
`IntoIterator<Item = Into<Bytes>>` that docs.rs/latest shows — a newer signature that has not
shipped here. And `PubSubSink::send_recv` goes through an internally-spawned `PipelineSink`
rather than through whoever polls the stream, which is what makes it safe for `subscribe()` to
await the `SUBSCRIBE` ack while the driver task is between polls. Guessing either would have
produced a deadlock or a type error.

The connection is opened **lazily**, on the first `subscribe()`. An instance with no SSE viewers
holds no subscriber connection at all, which is strictly better than the per-channel design and
matters under scale-to-zero.

Lock ordering is `subscribers` before `sink`, everywhere. The reconnect path originally read the
channel set while holding the `sink` write lock, which deadlocks against `subscribe()` holding
`subscribers` and wanting `sink`; each acquisition is now separately scoped.

- [x] **Step 2: Convert reaping from drop-the-connection to `UNSUBSCRIBE`**

The driver never exits. A channel whose last receiver has gone is removed from the map and
`UNSUBSCRIBE`d; the connection carries on serving every other channel.

One hole the plan did not name: the driver only learns a broadcast is empty when the **next
message arrives on it**, so a channel that simply goes quiet would keep its subscription — and
be restored on every reconnect — for the life of the process. A sweep on the `subscribe` slow
path closes it without adding a timer.

- [x] **Step 3: Preserve the reconnect path**

`reconnect()` re-`SUBSCRIBE`s the full live set read from `subscribers`, in one command, which is
why that map is the authoritative record rather than a cache of one.

Tested, and the tests were **confirmed by mutation, not by passing**. Breaking `reconnect` to
restore only `&channels[0..1]` fails `reconnect_restores_every_live_channel` and nothing else;
disabling the sweep fails `dropping_a_receiver_unsubscribes_without_closing_the_connection` and
nothing else. The reconnect test kills the real socket with `CLIENT KILL ID`, targeting the id it
identified by diffing `CLIENT LIST TYPE pubsub` around its own setup, so it cannot collaterally
kill a parallel test's connection.

- [ ] **Step 4: Confirm the ops/sec ceiling is now the binding limit, not connections**

**Still open, and deliberately not ticked.** The connection half is measured — a test diffs
`CLIENT LIST TYPE pubsub` across five `subscribe()` calls and requires growth of exactly one, so
4 flat per replica is a finding rather than a claim. The **throughput** half is not: nothing has
measured the app's actual ops·s⁻¹ against the free tier's 100, and multiplexing changed the
connection count, not the message count. Do not read "Task 16 done" as "the free tier fits".

---

## Task 17: Cloudflare Access + a Smoke Test That Survives It

**Added 2026-09-22.** The original plan has **no Access steps at all** — Task 4's origin secret
is a different mechanism (it stops the `*.run.app` URL bypassing the edge; it does not gate
humans).

Readiness §5 decided **Cloudflare Access** for gating: free ≤50 users, email OTP, per-person
revocation. The collision: Task 12's post-deploy smoke test curls `https://${DOMAIN}/api/health`,
and Access will answer that with a login redirect. The deploy will report failure on a service
that is actually healthy.

- [ ] **Step 1: Create the Access application and policy**

Cover both hosts. Email OTP, allow-list the tester addresses.

- [ ] **Step 2: Give CI a way through**

Either an Access **service token** (`CF-Access-Client-Id` / `CF-Access-Client-Secret` headers,
stored in Secret Manager) or a bypass policy scoped to the health path alone. Prefer the service
token: a bypass policy is a permanent hole that outlives the reason for it.

- [ ] **Step 3: Fix the smoke test to assert the body, not just the status**

An Access login page can return 200. Assert `{"status":"ok"}`, not the status code — otherwise
the test passes against the login screen. (Readiness §2's own lesson: a well-formed response to
an adjacent question is the hardest kind of false pass to notice.)

- [ ] **Step 4: Check Cloud Run's own probe is not caught by the origin middleware**

`CloudflareOrigin` wraps outermost, so an **HTTP** startup probe against `/health` gets 403 —
the container would never become ready, and the error says only "forbidden". Cloud Run's
*default* probe is TCP, so this only bites if Task 9 or 12 configures an HTTP one. Either leave
it TCP or exempt the path.

---

## Task 18: A Comp Path That Works in Production

**Added 2026-09-22.** Raised by the owner's requirement to use the product himself without
paying and to give some friends free access.

Entitlement is a `tier` column on `subscriptions`, so the capability exists. The only tool that
writes it without Stripe is the `set_subscription` CLI, and `server/src/bin/set_subscription.rs:287`
**refuses to run when `APP_ENV` is `production` or `prod`** — a deliberate guard.

The readiness doc's workaround was to set `environment = "staging"`, but that also disables the
`cookie_secure` validation `Config::validate` enforces under production
(`server/src/config/server.rs:619`). Trading a real safety check for a comp mechanism is the
wrong trade.

**DONE 2026-09-22.**

- [x] **Step 1: Added `--i-know-this-is-production` to the CLI**

Chose the flag over an admin endpoint: the endpoint needs an admin role the app does not have,
and the flag keeps the blast radius inside a binary that is not in the production image
(`Dockerfile:62` copies only `target/release/server`). The guard stays on by default; bypassing
it is explicit and shows up in shell history.

- [x] **Step 2: `environment = "production"` stays truthful**

Which was the point — the old workaround silently disarmed the `cookie_secure` check.

- [x] **Step 3: Fixed a defect that made the whole thing moot**

The CLI assembled its own DSN from the individual config fields and **ignored `url` entirely**,
so it could not reach a managed Postgres at all — precisely the case this task exists for. It
now resolves the DSN through the same `connection_string` helper the server uses.

- [x] **Step 4: The target database is printed before any mutation**

`APP_ENV` states *intent*; it says nothing about which database is on the other end of the
socket, and it is typically **unset** when running from a laptop through a Cloud SQL proxy —
the exact case you would most want caught. So the guard is not the real safety mechanism here.
The printed target is: host and database only, credentials stripped, on every run. Three tests
cover redaction, including a credential-free DSN and Cloud SQL's Unix-socket form (whose
instance name is the only way to tell staging from production at a glance).

---

## Task 19: Clear cargo-deny Before Deploying

**Added 2026-09-22.** Readiness §B-2 recorded one advisory; there are now **three**, verified
against CI run `35268609234` (2026-09-17). Frontend, Backend, OpenAPI and E2E all pass —
cargo-deny is still the only red job.

| Advisory | Crate | Note |
|---|---|---|
| `RUSTSEC-2026-0204` | crossbeam-epoch | Invalid pointer dereference in `fmt::Pointer`. Transitive. The one already recorded. |
| `RUSTSEC-2026-0258` | h2 | **Unbounded empty DATA frames — remote DoS.** New, and the one that actually matters once this is publicly reachable. |
| `RUSTSEC-2026-0285` | rustls | TLS 1.3 handshake messages accepted across encryption level boundaries. New. |
| yanked | spin | Warning only. |

**DONE 2026-09-22. `cargo deny check` now reports `advisories ok, bans ok, licenses ok, sources ok`
with zero additions to the `ignore` list.**

- [x] **Step 1: Targeted `cargo update` cleared three of the four**

`crossbeam-epoch` 0.9.18 → 0.9.21, `rustls` 0.23.31 → 0.23.45, `spin` 0.9.8 → 0.9.9 (un-yanked),
and the `h2` **0.4** line 0.4.12 → 0.4.19. `aws-lc-sys` came along for the ride, 0.41.0 → 0.45.0;
the workspace still builds and the full suite still passes on it.

- [x] **Step 2: `h2` 0.3 could not be updated — it was removed instead**

The interesting one. `actix-http` 3.x requires `h2 ^0.3`, and **0.3.27 is the last release of that
line** — confirmed against the crates.io version list; the fix landed in 0.4.16 and no 0.3.x
backport exists. Bumping `actix-web` to 4.12.1 / `actix-http` 3.13.6 did *not* move it, so no
dependency update could clear the advisory. That bump was reverted as unrelated churn.

The fix: **drop actix-web's `http2` feature**, which removes the crate from the graph entirely.
Verified reachable-by-nobody first, from actix-web's own source rather than by assumption —
`HttpServer::bind()` calls `listen()`, which builds `.tcp()`; HTTP/2 is reached only via
`.tcp_auto_h2c()` (behind the opt-in `bind_auto_h2c()`) or via ALPN on
`bind_rustls()`/`bind_openssl()`. This app calls plain `.bind()` at `server/src/server.rs:251`
and terminates TLS at nginx/Cloudflare, so the h2 code was compiled but unreachable.

Checked for feature unification before trusting it: `actix-cors`, `actix-web-lab` and
`utoipa-swagger-ui` all declare `actix-web` with `default-features = false`, so `http2` came
only from our own manifest and removing it actually takes effect. Without that check this would
have been a change that is accepted and silently does nothing.

- [x] **Step 3: Nothing added to `deny.toml`**

`deny.toml:15-18`'s convention was never invoked. Worth noting for next time: an ignore entry
would have been the *worse* outcome even though the advisory was genuinely unreachable, because
the feature removal also drops dead code from the binary and cannot silently stop being true.

**If HTTP/2 is ever wanted at the origin**, re-add the feature *and* re-check the advisory — do
not assume it is still unfixed.

---

## Spec Review

After completing all tasks, the following items from `docs/superpowers/specs/2026-04-20-deployment-design.md` are implemented:

| Spec item | Task |
|---|---|
| Cloud Run backend + frontend | Tasks 9, 12 |
| Postgres (vendor TBD — same in both envs) | Tasks 9, 10, 11, 15 |
| Redis (vendor TBD — same in both envs) | Tasks 3, 10, 11, 16 |
| Cloudflare edge | Task 10 |
| Cloudflare Access gating | Task 17 |
| Reconcile via Cloud Scheduler | Task 14 |
| Migration strategy settled | Task 20 |
| Comp path for unpaid access | Task 18 |
| cargo-deny green | Task 19 |
| GCP Secret Manager | Tasks 9, 11 |
| Workload Identity Federation | Task 9 |
| Staging auto-deploy on main | Task 12 |
| Production tag-gated deploy | Task 12 |
| sqlx migrations in CI | Task 12 |
| Smoke test | Tasks 1, 12 |
| CF origin secret middleware | Task 4 |
| Health endpoint | Task 1 |
| Single frontend image (envsubst) | Task 7 |
| Cookie SameSite=Lax + domain | Task 5 |
| Rollback on failure | Task 12 |

**Spec inaccuracy (not blocking):** The spec describes `frontend-staging:{sha}` and `frontend-prod:{sha}` as two separate images. The correct implementation (Task 7) uses one image with `BACKEND_URL` envsubst at runtime, since `VITE_API_BASE_URL` is never read by the frontend source. As of 2026-05-03, `VITE_GOOGLE_CLIENT_ID` has also been removed in favour of a runtime `/api/public-config` endpoint, so the frontend image truly has zero build-time configuration — one identical image deploys to both environments.
