# Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make anime-calendar deployable to GCP Cloud Run with full CI/CD automation, Cloudflare protection, and two isolated environments (staging/production).

**Architecture:** Rust backend + Vue frontend as separate Cloud Run services behind Cloudflare. Staging uses Neon (free Postgres) + Upstash Redis free tier; production uses Cloud SQL db-f1-micro + Upstash pay-per-use. Secrets injected at runtime from GCP Secret Manager. No config files in production — env vars only.

**Tech Stack:** GCP Cloud Run, Cloud SQL (PostgreSQL 17), Neon, Upstash Redis, Cloudflare (free), GCP Secret Manager, GCP Artifact Registry, GitHub Actions, Workload Identity Federation, sqlx migrations, nginx envsubst, Docker multi-stage builds.

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

- [ ] **Step 1: Write the health controller**

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

- [ ] **Step 2: Register the module**

Edit `server/src/controllers.rs`, add:

```rust
pub mod health;
```

- [ ] **Step 3: Register the route in server.rs**

In `server/src/server.rs`, add to the `use crate::controllers` import:

```rust
use crate::controllers::{
    auth, calendar, email_verification, health, item, items, oauth, password_reset, refresh, user,
};
```

Add `.service(health::health)` to the service list (before `.service(item::get)`).

- [ ] **Step 4: Verify it compiles**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors.

- [ ] **Step 5: Write a test for the health endpoint**

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

- [ ] **Step 6: Run tests**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server controllers::health
```

Expected: 2 tests pass.

- [ ] **Step 7: Commit**

```bash
git add server/src/controllers/health.rs server/src/controllers.rs server/src/server.rs
git commit -m "feat: add GET /health endpoint"
```

---

## Task 2: Environment Variable Config Override

The `config` crate supports layered sources — file first, then env vars override. Cloud Run has no config files, so env vars must work standalone.

**Files:**
- Modify: `server/src/config/server.rs`
- Modify: `server/src/config/database.rs`

- [ ] **Step 1: Update Server::new() to support env vars**

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

- [ ] **Step 2: Add optional redis.url field to RedisConfig**

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

- [ ] **Step 3: Add cf_origin_secret and cookie_domain to Server**

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

- [ ] **Step 4: Add cookie_domain to CookieSettings**

Update the `CookieSettings` struct at the bottom of `server/src/config/server.rs`:

```rust
#[derive(Clone)]
pub struct CookieSettings {
    pub secure: bool,
    pub domain: Option<String>,
}
```

- [ ] **Step 5: Update Database::new() for env vars and add url field**

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
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()
    }
}
```

- [ ] **Step 6: Verify compile**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors (some `unused field` warnings for `url` are acceptable — they disappear in the next task).

- [ ] **Step 7: Commit**

```bash
git add server/src/config/server.rs server/src/config/database.rs
git commit -m "feat: support env var config override for Cloud Run (no config files required)"
```

---

## Task 3: Upstash Redis TLS Support

Upstash requires `rediss://` (TLS). `Cache::new()` builds only `redis://`. Add `Cache::from_url()` and wire it in `main.rs`.

**Files:**
- Modify: `server/src/cache.rs`
- Modify: `server/src/main.rs`

- [ ] **Step 1: Add Cache::from_url() constructor**

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

- [ ] **Step 2: Update main.rs to use from_url when redis.url is set**

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

- [ ] **Step 3: Verify compile**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors.

- [ ] **Step 4: Write a test for from_url**

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

- [ ] **Step 5: Run tests**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server cache
```

Expected: all cache tests pass (including new `from_url` test if Redis test server is available).

- [ ] **Step 6: Commit**

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

- [ ] **Step 1: Write the middleware**

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

- [ ] **Step 2: Register the module**

Edit `server/src/middleware.rs`:

```rust
pub mod auth;
pub mod cloudflare;
```

- [ ] **Step 3: Add futures-util to Cargo.toml**

Check if `futures-util` is already a dependency:

```bash
grep "futures" server/Cargo.toml
```

If not present, add to `server/Cargo.toml` under `[dependencies]`:

```toml
futures-util = "0.3"
```

- [ ] **Step 4: Wire the middleware in server.rs conditionally**

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

- [ ] **Step 5: Update CookieSettings construction in server.rs**

Replace the `CookieSettings` construction:

```rust
let cookie_settings = CookieSettings {
    secure: config.cookie_secure,
    domain: config.cookie_domain.clone(),
};
```

- [ ] **Step 6: Verify compile**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors.

- [ ] **Step 7: Run middleware tests**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server middleware::cloudflare
```

Expected: 3 tests pass.

- [ ] **Step 8: Commit**

```bash
git add server/src/middleware/cloudflare.rs server/src/middleware.rs server/src/server.rs
git commit -m "feat: add Cloudflare origin secret middleware (empty secret = disabled)"
```

---

## Task 5: Cookie Domain + SameSite::Lax

Cookies need `Domain=.yourdomain.com` and `SameSite=Lax` to work across subdomains behind Cloudflare.

**Files:**
- Modify: `server/src/controllers/auth.rs`

- [ ] **Step 1: Find the cookie builder functions**

The functions are `build_auth_cookie` and `build_refresh_cookie` in `server/src/controllers/auth.rs`. Read around line 76 to locate them.

- [ ] **Step 2: Update build_auth_cookie**

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

- [ ] **Step 3: Find and update build_refresh_cookie**

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

- [ ] **Step 4: Check for logout cookie clearing**

Search for `Cookie::build` or `build_*cookie` calls used to clear cookies at logout. They also need `SameSite::Lax` and `domain` applied. The pattern is typically a zero/negative `max_age`. Apply the same builder pattern to any clearing cookies.

- [ ] **Step 5: Compile + test**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo test -p server
```

Expected: all existing tests pass.

- [ ] **Step 6: Commit**

```bash
git add server/src/controllers/auth.rs
git commit -m "fix: cookies use SameSite=Lax and support optional domain for subdomain auth"
```

---

## Task 6: Update config.toml.dist

**Files:**
- Modify: `config.toml.dist`

- [ ] **Step 1: Add new fields to config.toml.dist**

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

- [ ] **Step 2: Commit**

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

- [ ] **Step 1: Rename and update nginx.conf to a template**

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

- [ ] **Step 2: Update frontend/Dockerfile**

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

- [ ] **Step 3: Test the Docker build locally**

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

- [ ] **Step 4: Commit**

```bash
git add frontend/nginx.conf.template frontend/Dockerfile
git commit -m "feat: nginx envsubst template for BACKEND_URL (single image, both environments)"
```

---

## Task 8: Database URL Support in main.rs

When `database.url` is set (Cloud Run uses Neon or Cloud SQL connection string), use it directly instead of building a DSN from individual fields.

**Files:**
- Modify: `server/src/main.rs`

- [ ] **Step 1: Read the current database pool creation in main.rs**

Find the line in `server/src/main.rs` that calls `PgPoolOptions` or `sqlx::postgres::PgPool` / `PgPoolOptions::new()`.

- [ ] **Step 2: Update pool creation to use url field**

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

- [ ] **Step 3: Verify compile**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 SQLX_OFFLINE=true cargo check --workspace
```

Expected: no errors.

- [ ] **Step 4: Commit**

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
  --location=us-central1 \
  --project=YOUR_PROJECT_ID
```

- [ ] **Step 3: Create Cloud SQL instance (production)**

```bash
gcloud sql instances create anime-calendar-prod \
  --database-version=POSTGRES_17 \
  --tier=db-f1-micro \
  --region=us-central1 \
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
- Artifact Registry URL: `us-central1-docker.pkg.dev/YOUR_PROJECT_ID/anime-calendar`
- Cloud SQL instance connection name: `YOUR_PROJECT_ID:us-central1:anime-calendar-prod`

---

## Task 10: External Services Setup

- [ ] **Step 1: Create Neon account + database (staging)**

1. Go to neon.tech → create account → create project named `anime-calendar-staging`
2. Select PostgreSQL 17, region matching your GCP region
3. Note the connection string: `postgres://...@ep-xxx.neon.tech/neondb?sslmode=require`

- [ ] **Step 2: Create Upstash Redis databases**

1. Go to upstash.com → create account
2. Create database named `anime-calendar-staging` (free tier, region matching GCP)
3. Create database named `anime-calendar-prod` (pay-per-use, same region)
4. From each database page, copy the **TLS URL**: `rediss://default:...@...upstash.io:6379`

- [ ] **Step 3: Run initial Neon migration**

With the Neon connection string:

```bash
DATABASE_URL="postgres://...@ep-xxx.neon.tech/neondb?sslmode=require" \
  cargo sqlx migrate run --source server/migrations
```

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

```bash
PROJECT=YOUR_PROJECT_ID

# Staging secrets
echo -n "postgres://...@ep-xxx.neon.tech/neondb?sslmode=require" | \
  gcloud secrets create staging--database-url --data-file=- --project=$PROJECT

echo -n "rediss://default:...@...upstash.io:6379" | \
  gcloud secrets create staging--redis-url --data-file=- --project=$PROJECT

echo -n "$(openssl rand -hex 32)" | \
  gcloud secrets create staging--jwt-secret --data-file=- --project=$PROJECT

# Production secrets
echo -n "postgres://anime_user:STRONG_PASSWORD@/anime_calendar?host=/cloudsql/YOUR_PROJECT_ID:us-central1:anime-calendar-prod" | \
  gcloud secrets create prod--database-url --data-file=- --project=$PROJECT

echo -n "rediss://default:...@...upstash.io:6379" | \
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
      REGION: us-central1
      REGISTRY: us-central1-docker.pkg.dev/${{ vars.GCP_PROJECT_ID }}/anime-calendar
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
        run: gcloud auth configure-docker us-central1-docker.pkg.dev --quiet

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

      - name: Run database migrations (Neon staging)
        env:
          DATABASE_URL: ${{ steps.secrets.outputs.db_url }}
        run: |
          cargo install sqlx-cli --no-default-features --features postgres,native-tls
          sqlx migrate run --source server/migrations

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
      REGION: us-central1
      REGISTRY: us-central1-docker.pkg.dev/${{ vars.GCP_PROJECT_ID }}/anime-calendar
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
        run: gcloud auth configure-docker us-central1-docker.pkg.dev --quiet

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
            sqlx migrate run --source server/migrations
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

## Spec Review

After completing all tasks, the following items from `docs/superpowers/specs/2026-04-20-deployment-design.md` are implemented:

| Spec item | Task |
|---|---|
| Cloud Run backend + frontend | Tasks 9, 12 |
| Cloud SQL (prod) + Neon (staging) | Tasks 9, 10, 11 |
| Upstash Redis (both envs) | Tasks 3, 10, 11 |
| Cloudflare edge | Task 10 |
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
