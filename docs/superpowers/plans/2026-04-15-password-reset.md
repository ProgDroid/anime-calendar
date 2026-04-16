# Password Reset Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a secure, email-based password reset flow — `POST /auth/forgot-password` + `POST /auth/reset-password` endpoints, a `password_reset_tokens` DB table, an async SMTP email service, and `ForgotPasswordPage` / `ResetPasswordPage` Vue components.

**Architecture:** The raw token (32 cryptographically random bytes, hex-encoded) is emailed to the user; only its SHA-256 hash is stored in the DB — a DB breach alone cannot trigger resets. Tokens expire after 1 hour and are single-use (marked via `used_at`). The `forgot-password` endpoint always returns 200 to prevent user enumeration. If SMTP host is empty (dev), the reset URL is logged at WARN level instead of sent.

**Tech Stack:** `lettre 0.11` (async SMTP), `sha2 0.10` (token hashing), `rand 0.9` (already present — token generation), `sqlx` (existing DB layer), Vue 3 + vue-router + vue-i18n + Pinia (existing frontend stack)

---

## File Map

**Create:**
- `migrations/20260415000000_add_password_reset_tokens.sql`
- `server/src/mappers/password_reset.rs`
- `server/src/services/email.rs`
- `server/src/controllers/password_reset.rs`
- `frontend/src/components/ForgotPasswordPage.vue`
- `frontend/src/components/ResetPasswordPage.vue`

**Modify:**
- `server/Cargo.toml` — add `lettre`, `sha2`
- `server/src/mappers.rs` — register `pub mod password_reset`
- `server/src/services.rs` — register `pub mod email`
- `server/src/controllers.rs` — register `pub mod password_reset`
- `server/src/error.rs` — add `InvalidResetToken`, `EmailError` variants
- `server/src/config/server.rs` — add `SmtpConfig`, `AppBaseUrl` newtype, `app_base_url` + `smtp` fields on `Server`
- `server/src/server.rs` — update `start()` signature; register routes + inject data
- `server/src/main.rs` — create `PasswordResetMapper`, `EmailService`; pass to `start()`
- `server/src/openapi.rs` — register new paths and request/response schemas
- `config.toml.dist` — add `app_base_url`, `[smtp]` section
- `frontend/src/router/index.ts` — add `/forgot-password`, `/reset-password` routes
- `frontend/src/locales/en.json` — add `auth.forgotPassword.*`, `auth.resetPassword.*`
- `frontend/src/locales/pt.json` — mirror same keys in Portuguese
- `frontend/src/components/LoginPage.vue` — add "Forgot password?" link below password input

---

## Task 1: DB Migration

**Files:**
- Create: `migrations/20260415000000_add_password_reset_tokens.sql`

- [ ] **Step 1: Write the migration file**

```sql
-- migrations/20260415000000_add_password_reset_tokens.sql
CREATE TABLE password_reset_tokens (
    id          SERIAL       PRIMARY KEY,
    user_id     INTEGER      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT         NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ  NOT NULL,
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_prt_token_hash ON password_reset_tokens(token_hash);
CREATE INDEX idx_prt_user_id    ON password_reset_tokens(user_id);
```

- [ ] **Step 2: Apply migration**

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/anime_calendar sqlx migrate run
```

Expected output: `Applied 20260415000000/migrate add_password_reset_tokens`

- [ ] **Step 3: Commit**

```bash
git add migrations/20260415000000_add_password_reset_tokens.sql
git commit -m "feat: add password_reset_tokens migration"
```

---

## Task 2: `PasswordResetMapper`

**Files:**
- Create: `server/src/mappers/password_reset.rs`
- Modify: `server/src/mappers.rs`

- [ ] **Step 1: Write the tests + implementation in one file**

Create `server/src/mappers/password_reset.rs` with the full contents below:

```rust
use sqlx::PgPool;

use crate::{config::database::DatabaseConfig, mappers::database::Database, ServerResult};

#[derive(Clone)]
pub struct PasswordResetMapper {
    db: Database,
}

pub struct PasswordResetToken {
    pub id: i32,
    pub user_id: i32,
}

impl PasswordResetMapper {
    /// # Errors
    /// Fails if the database connection cannot be established.
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self { db: Database::new(config).await? })
    }

    /// Construct from a bare pool — for integration tests only.
    #[cfg(test)]
    pub fn from_pool(pool: PgPool) -> Self {
        Self { db: Database { pool } }
    }

    /// Delete all tokens for the given user (expired, used, or pending).
    /// Call before `create_token` to keep the table tidy.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn invalidate_previous_tokens(&self, user_id: i32) -> ServerResult<()> {
        sqlx::query!(
            "DELETE FROM password_reset_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }

    /// Store a new reset token that expires 1 hour from now.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn create_token(&self, user_id: i32, token_hash: &str) -> ServerResult<()> {
        sqlx::query!(
            "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
            user_id,
            token_hash,
        )
        .execute(&self.db.pool)
        .await?;
        Ok(())
    }

    /// Return the token row if it exists, is not yet used, and has not expired.
    ///
    /// # Errors
    /// Returns `Error::Database` (row-not-found) if no valid token matches.
    pub async fn find_valid_token(&self, token_hash: &str) -> ServerResult<PasswordResetToken> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM password_reset_tokens \
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()",
            token_hash,
        )
        .fetch_one(&self.db.pool)
        .await?;

        Ok(PasswordResetToken { id: row.id, user_id: row.user_id })
    }

    /// Mark the token as used AND update the user's password in one transaction.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn complete_reset(
        &self,
        token_id: i32,
        user_id: i32,
        password_hash: &str,
    ) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1",
            token_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "UPDATE users SET password_hash = $1, updated_at = NOW() \
             WHERE id = $2 AND deleted_at IS NULL",
            password_hash,
            user_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mappers::user::UserMapper, services::auth::hash_password};
    use sqlx::PgPool;

    const PW: &str = "TestPass12!@";

    async fn seed_user(pool: &PgPool, email: &str) -> i32 {
        UserMapper::from_pool(pool.clone())
            .create_user("testuser", email, Some(&hash_password(PW).unwrap()))
            .await
            .unwrap()
            .id
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn create_and_find_valid_token(pool: PgPool) {
        let user_id = seed_user(&pool, "a@test.com").await;
        let mapper = PasswordResetMapper::from_pool(pool);
        mapper.create_token(user_id, "hash_abc").await.unwrap();

        let token = mapper.find_valid_token("hash_abc").await.unwrap();
        assert_eq!(token.user_id, user_id);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn find_valid_token_not_found_returns_error(pool: PgPool) {
        let mapper = PasswordResetMapper::from_pool(pool);
        assert!(mapper.find_valid_token("nonexistent").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn complete_reset_marks_token_used(pool: PgPool) {
        let user_id = seed_user(&pool, "b@test.com").await;
        let mapper = PasswordResetMapper::from_pool(pool);
        mapper.create_token(user_id, "hash_used").await.unwrap();
        let token = mapper.find_valid_token("hash_used").await.unwrap();

        let new_hash = hash_password("NewPass12!@").unwrap();
        mapper.complete_reset(token.id, user_id, &new_hash).await.unwrap();

        // Token is now used — find_valid_token must fail
        assert!(mapper.find_valid_token("hash_used").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn invalidate_previous_tokens_removes_all_for_user(pool: PgPool) {
        let user_id = seed_user(&pool, "c@test.com").await;
        let mapper = PasswordResetMapper::from_pool(pool);
        mapper.create_token(user_id, "hash_1").await.unwrap();
        mapper.create_token(user_id, "hash_2").await.unwrap();

        mapper.invalidate_previous_tokens(user_id).await.unwrap();

        assert!(mapper.find_valid_token("hash_1").await.is_err());
        assert!(mapper.find_valid_token("hash_2").await.is_err());
    }
}
```

- [ ] **Step 2: Register the module in `server/src/mappers.rs`**

Add after the last existing line:
```rust
pub mod password_reset;
```

- [ ] **Step 3: Run tests (requires live DB)**

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/anime_calendar \
  cargo test --package server mappers::password_reset
```

Expected: 4 tests pass.

- [ ] **Step 4: Regenerate `.sqlx/` offline cache**

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/anime_calendar \
  cargo sqlx prepare --workspace
```

- [ ] **Step 5: Commit**

```bash
git add server/src/mappers/password_reset.rs server/src/mappers.rs .sqlx/
git commit -m "feat: add PasswordResetMapper with token lifecycle + tests"
```

---

## Task 3: Email service + config

**Files:**
- Modify: `server/Cargo.toml`
- Modify: `server/src/config/server.rs`
- Modify: `config.toml.dist`
- Create: `server/src/services/email.rs`
- Modify: `server/src/services.rs`
- Modify: `server/src/error.rs`

- [ ] **Step 1: Add dependencies to `server/Cargo.toml`**

In the `[dependencies]` table add:
```toml
lettre = { version = "0.11", features = ["tokio1-native-tls", "smtp-transport", "builder"] }
sha2 = "0.10"
```

- [ ] **Step 2: Add `SmtpConfig` and `AppBaseUrl` to `server/src/config/server.rs`**

Add after the existing `RedisConfig` struct:

```rust
#[derive(Debug, Deserialize, Clone, Default)]
pub struct SmtpConfig {
    #[serde(default)]
    pub host: String,
    #[serde(default = "default_smtp_port")]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: SecretString,
    #[serde(default)]
    pub from_address: String,
}

fn default_smtp_port() -> u16 {
    587
}

/// Newtype wrapper for the frontend base URL — injected as `web::Data<AppBaseUrl>`.
#[derive(Clone)]
pub struct AppBaseUrl(String);

impl AppBaseUrl {
    #[must_use]
    pub fn new(url: String) -> Self {
        Self(url)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

Add two fields inside the `Server` struct (after the existing `cookie_secure` field):

```rust
#[serde(default = "default_app_base_url")]
pub app_base_url: String,
#[serde(default)]
pub smtp: SmtpConfig,
```

Add the default function alongside the others:

```rust
fn default_app_base_url() -> String {
    "http://localhost:5173".to_string()
}
```

- [ ] **Step 3: Update `config.toml.dist`**

Add after the existing fields:
```toml
app_base_url = "http://localhost:5173"

[smtp]
host = ""
port = 587
username = ""
password = ""
from_address = "noreply@example.com"
```

- [ ] **Step 4: Create `server/src/services/email.rs`**

```rust
use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use log::warn;
use secrecy::ExposeSecret as _;

use crate::{config::server::SmtpConfig, error::Error, ServerResult};

#[derive(Clone)]
pub struct EmailService {
    config: SmtpConfig,
}

impl EmailService {
    #[must_use]
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    /// Send a password reset email.
    ///
    /// If `smtp.host` is empty (e.g. local dev), logs the reset URL at WARN
    /// level instead of attempting a connection.
    ///
    /// # Errors
    /// Fails if SMTP connection fails or the email cannot be built/sent.
    pub async fn send_password_reset(&self, to_email: &str, reset_url: &str) -> ServerResult<()> {
        if self.config.host.is_empty() {
            warn!("SMTP not configured — password reset URL for {to_email}: {reset_url}");
            return Ok(());
        }

        let from = self
            .config
            .from_address
            .parse()
            .map_err(|e: lettre::address::AddressError| Error::EmailError(e.to_string()))?;

        let to = to_email
            .parse()
            .map_err(|e: lettre::address::AddressError| Error::EmailError(e.to_string()))?;

        let email = Message::builder()
            .from(from)
            .to(to)
            .subject("Reset your Anime Calendar password")
            .header(ContentType::TEXT_PLAIN)
            .body(format!(
                "Someone requested a password reset for your Anime Calendar account.\n\n\
                 Click the link below to choose a new password (expires in 1 hour):\n\n\
                 {reset_url}\n\n\
                 If you did not request this, you can safely ignore this email."
            ))
            .map_err(|e| Error::EmailError(e.to_string()))?;

        let creds = Credentials::new(
            self.config.username.clone(),
            self.config.password.expose_secret().to_string(),
        );

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host)
            .map_err(|e| Error::EmailError(e.to_string()))?
            .port(self.config.port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| Error::EmailError(e.to_string()))?;

        Ok(())
    }
}
```

- [ ] **Step 5: Register in `server/src/services.rs`**

Add:
```rust
pub mod email;
```

- [ ] **Step 6: Add `InvalidResetToken` and `EmailError` to `server/src/error.rs`**

In the `Error` enum, add after `InvalidPassword`:
```rust
#[error("Reset token is invalid or has expired")]
InvalidResetToken,
#[error("Failed to send email: {0}")]
EmailError(String),
```

In the `status_code` match arm, add `InvalidResetToken` to the 400 group:
```rust
Self::InvalidRequest
| Self::UserAlreadyExists
| Self::InvalidPassword
| Self::InvalidResetToken
| Self::CannotHashPassword(_)
| Self::CannotGenerateAuthToken(_) => StatusCode::BAD_REQUEST,
```

Add `EmailError` to the 500 group:
```rust
Self::Database(_)
| Self::Config(_)
| Self::Server(_)
| Self::GovernorConfig
| Self::EmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
```

- [ ] **Step 7: Verify compilation**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo check --package server
```

Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add server/Cargo.toml server/src/config/server.rs server/src/services/email.rs \
        server/src/services.rs server/src/error.rs config.toml.dist
git commit -m "feat: add EmailService (SMTP + dev log fallback), SmtpConfig, AppBaseUrl"
```

---

## Task 4: Backend controller — `forgot_password` + `reset_password`

**Files:**
- Create: `server/src/controllers/password_reset.rs`
- Modify: `server/src/controllers.rs`

- [ ] **Step 1: Create `server/src/controllers/password_reset.rs`**

```rust
use actix_web::{post, web, HttpResponse};
use log::error;
use rand::RngCore as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use utoipa::ToSchema;

use crate::{
    config::server::AppBaseUrl,
    controllers::auth::ErrorResponse,
    error::Error,
    mappers::{password_reset::PasswordResetMapper, user::UserMapper},
    services::{
        auth::{hash_password, validate_password_strength},
        email::EmailService,
    },
};

#[derive(Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

fn generate_raw_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[must_use]
pub fn hash_reset_token(raw_token: &str) -> String {
    let hash = Sha256::digest(raw_token.as_bytes());
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

const RESET_RESPONSE: &str =
    "If an account exists with that email, you'll receive a reset link shortly.";

#[utoipa::path(
    post,
    path = "/auth/forgot-password",
    tag = "auth",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Always returned — prevents user enumeration", body = MessageResponse),
        (status = 500, description = "Email or DB error", body = ErrorResponse),
    )
)]
#[post("/auth/forgot-password")]
pub async fn forgot_password(
    user_mapper: web::Data<UserMapper>,
    token_mapper: web::Data<PasswordResetMapper>,
    email_service: web::Data<EmailService>,
    app_base_url: web::Data<AppBaseUrl>,
    body: web::Json<ForgotPasswordRequest>,
) -> HttpResponse {
    let ok = HttpResponse::Ok().json(MessageResponse { message: RESET_RESPONSE.into() });

    // Always return 200 — do not reveal whether the account exists.
    let Ok(user) = user_mapper.get_user_by_email(&body.email).await else {
        return ok;
    };

    // OAuth-only users have no password — silently do nothing.
    if user.password_hash.is_none() {
        return ok;
    }

    let raw_token = generate_raw_token();
    let token_hash = hash_reset_token(&raw_token);
    let reset_url = format!(
        "{}/reset-password?token={raw_token}",
        app_base_url.as_str()
    );

    // Best-effort cleanup — don't abort the flow if old tokens can't be deleted.
    if let Err(e) = token_mapper.invalidate_previous_tokens(user.id).await {
        error!("Failed to invalidate previous reset tokens for user {}: {e}", user.id);
    }

    if let Err(e) = token_mapper.create_token(user.id, &token_hash).await {
        error!("{e}");
        return e.error_response();
    }

    if let Err(e) = email_service
        .send_password_reset(&user.email, &reset_url)
        .await
    {
        error!("{e}");
        return e.error_response();
    }

    HttpResponse::Ok().json(MessageResponse { message: RESET_RESPONSE.into() })
}

#[utoipa::path(
    post,
    path = "/auth/reset-password",
    tag = "auth",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password updated successfully"),
        (status = 400, description = "Invalid/expired token or weak password", body = ErrorResponse),
    )
)]
#[post("/auth/reset-password")]
pub async fn reset_password(
    token_mapper: web::Data<PasswordResetMapper>,
    body: web::Json<ResetPasswordRequest>,
) -> HttpResponse {
    // Validate password before touching the DB.
    if body.new_password.len() > 128 || !validate_password_strength(&body.new_password) {
        return Error::InvalidPassword.error_response();
    }

    let token_hash = hash_reset_token(&body.token);

    let token = match token_mapper.find_valid_token(&token_hash).await {
        Ok(t) => t,
        Err(_) => return Error::InvalidResetToken.error_response(),
    };

    let new_hash = match hash_password(&body.new_password) {
        Ok(h) => h,
        Err(e) => return e.error_response(),
    };

    match token_mapper
        .complete_reset(token.id, token.user_id, &new_hash)
        .await
    {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(e) => {
            error!("{e}");
            e.error_response()
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        config::server::{AppBaseUrl, SmtpConfig},
        mappers::{password_reset::PasswordResetMapper, user::UserMapper},
        services::{auth::hash_password, email::EmailService},
    };
    use actix_web::{http::StatusCode, test, web, App};
    use sqlx::PgPool;

    const STRONG_PW: &str = "SecurePass12!@";

    fn dev_email() -> web::Data<EmailService> {
        web::Data::new(EmailService::new(SmtpConfig::default()))
    }

    fn base_url() -> web::Data<AppBaseUrl> {
        web::Data::new(AppBaseUrl::new("http://localhost:5173".to_string()))
    }

    async fn seed_user(pool: &PgPool, email: &str) -> i32 {
        UserMapper::from_pool(pool.clone())
            .create_user("tester", email, Some(&hash_password(STRONG_PW).unwrap()))
            .await
            .unwrap()
            .id
    }

    // ─── POST /auth/forgot-password ──────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn forgot_password_known_email_returns_200(pool: PgPool) {
        seed_user(&pool, "alice@test.com").await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(PasswordResetMapper::from_pool(pool)))
                .app_data(dev_email())
                .app_data(base_url())
                .service(forgot_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/forgot-password")
            .set_json(serde_json::json!({ "email": "alice@test.com" }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn forgot_password_unknown_email_also_returns_200(pool: PgPool) {
        // Must return 200 — no user enumeration.
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(PasswordResetMapper::from_pool(pool)))
                .app_data(dev_email())
                .app_data(base_url())
                .service(forgot_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/forgot-password")
            .set_json(serde_json::json!({ "email": "nobody@test.com" }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn forgot_password_oauth_user_returns_200_without_creating_token(pool: PgPool) {
        // OAuth users have no password_hash — no token should be created.
        UserMapper::from_pool(pool.clone())
            .create_user("oauth", "oauth@test.com", None)
            .await
            .unwrap();

        let token_mapper = PasswordResetMapper::from_pool(pool.clone());
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(web::Data::new(token_mapper.clone()))
                .app_data(dev_email())
                .app_data(base_url())
                .service(forgot_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/forgot-password")
            .set_json(serde_json::json!({ "email": "oauth@test.com" }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
        // No token was created — find_valid_token returns nothing findable
        assert!(token_mapper.find_valid_token("anything").await.is_err());
    }

    // ─── POST /auth/reset-password ───────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn reset_password_valid_token_returns_200(pool: PgPool) {
        let user_id = seed_user(&pool, "bob@test.com").await;
        let token_mapper = PasswordResetMapper::from_pool(pool.clone());

        // Simulate a token as it would appear in the email URL.
        let raw = "a".repeat(64);
        let hash = hash_reset_token(&raw);
        token_mapper.create_token(user_id, &hash).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(token_mapper))
                .service(reset_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/reset-password")
            .set_json(serde_json::json!({
                "token": raw,
                "new_password": "NewSecurePass12!@"
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn reset_password_invalid_token_returns_400(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(PasswordResetMapper::from_pool(pool)))
                .service(reset_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/reset-password")
            .set_json(serde_json::json!({
                "token": "totally-fake-token",
                "new_password": "NewSecurePass12!@"
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn reset_password_used_token_returns_400(pool: PgPool) {
        let user_id = seed_user(&pool, "carol@test.com").await;
        let token_mapper = PasswordResetMapper::from_pool(pool.clone());

        let raw = "b".repeat(64);
        let hash = hash_reset_token(&raw);
        token_mapper.create_token(user_id, &hash).await.unwrap();
        let token = token_mapper.find_valid_token(&hash).await.unwrap();
        token_mapper
            .complete_reset(token.id, user_id, &hash_password("TempPass12!@").unwrap())
            .await
            .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(PasswordResetMapper::from_pool(pool)))
                .service(reset_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/reset-password")
            .set_json(serde_json::json!({
                "token": raw,
                "new_password": "AnotherNewPass12!@"
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn reset_password_weak_new_password_returns_400(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(PasswordResetMapper::from_pool(pool)))
                .service(reset_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/reset-password")
            .set_json(serde_json::json!({
                "token": "anytoken",
                "new_password": "weak"
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
}
```

- [ ] **Step 2: Register in `server/src/controllers.rs`**

Add:
```rust
pub mod password_reset;
```

- [ ] **Step 3: Run the integration tests**

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/anime_calendar \
  cargo test --package server controllers::password_reset
```

Expected: 7 tests pass.

- [ ] **Step 4: Regenerate `.sqlx/` offline cache**

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/anime_calendar \
  cargo sqlx prepare --workspace
```

- [ ] **Step 5: Commit**

```bash
git add server/src/controllers/password_reset.rs server/src/controllers.rs .sqlx/
git commit -m "feat: add forgot_password and reset_password endpoints with integration tests"
```

---

## Task 5: Wire up in `server.rs`, `main.rs`, and OpenAPI

**Files:**
- Modify: `server/src/server.rs`
- Modify: `server/src/main.rs`
- Modify: `server/src/openapi.rs`

- [ ] **Step 1: Update `server/src/server.rs`**

Update the import at the top — add `password_reset` to the controllers import and the new types:

```rust
use crate::{
    // existing...
    controllers::{auth, cache_metrics, calendar, item, items, oauth, password_reset, user},
    config::server::{AppBaseUrl, CookieSettings, JwtSecret, Server as ServerConfig},
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, google_oauth::GoogleOauth,
        password_reset::PasswordResetMapper, user::UserMapper,
        user_settings::UserSettingsMapper,
    },
    services::email::EmailService,
    // existing...
};
```

Update the `start()` signature to accept two new parameters (add after `user_settings_mapper`):

```rust
pub fn start(
    config: ServerConfig,
    anilist: Anilist,
    google_oauth: GoogleOauth,
    cache: Cache,
    user_mapper: UserMapper,
    calendar_mapper: CalendarMapper,
    user_settings_mapper: UserSettingsMapper,
    token_mapper: PasswordResetMapper,   // new
    email_service: EmailService,         // new
) -> ServerResult<Server> {
```

Inside the `HttpServer::new(move || { ... })` closure, add the new data and services (alongside the existing `.app_data(...)` calls):

```rust
.app_data(web::Data::new(token_mapper.clone()))
.app_data(web::Data::new(email_service.clone()))
.app_data(web::Data::new(AppBaseUrl::new(config.app_base_url.clone())))
```

Register the two new routes (alongside the existing `.service(auth::...)` calls):

```rust
.service(password_reset::forgot_password)
.service(password_reset::reset_password)
```

- [ ] **Step 2: Update `server/src/main.rs`**

Create the two new instances (after the existing mapper creation, before `server::start()`):

```rust
let token_mapper = PasswordResetMapper::new(db_config.clone()).await?;
let email_service = EmailService::new(settings.smtp.clone());
```

Add the two new arguments to the `server::start(...)` call:

```rust
Ok(server::start(
    settings,
    anilist,
    google_oauth,
    cache,
    user_mapper,
    calendar_mapper,
    user_settings_mapper,
    token_mapper,    // new
    email_service,   // new
)?
.await?)
```

Add the required imports at the top of `main.rs`:

```rust
use crate::{
    mappers::password_reset::PasswordResetMapper,
    services::email::EmailService,
    // existing imports...
};
```

- [ ] **Step 3: Update `server/src/openapi.rs`**

In the `paths(...)` block, add under the `// auth` section:

```rust
crate::controllers::password_reset::forgot_password,
crate::controllers::password_reset::reset_password,
```

In the `components(schemas(...))` block, add:

```rust
crate::controllers::password_reset::ForgotPasswordRequest,
crate::controllers::password_reset::ResetPasswordRequest,
crate::controllers::password_reset::MessageResponse,
```

- [ ] **Step 4: Verify full build**

```bash
AWS_LC_SYS_PREBUILT_NASM=1 cargo build --package server
```

Expected: builds cleanly.

- [ ] **Step 5: Run full test suite**

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/anime_calendar \
  cargo test --package server
```

Expected: all tests pass.

- [ ] **Step 6: Regenerate `.sqlx/` and update `docs/openapi.yaml`**

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/anime_calendar \
  cargo sqlx prepare --workspace
```

Start the server, then export the spec:
```bash
curl http://localhost:8080/api-docs/openapi.json > /tmp/openapi.json
# Convert to YAML (requires python3+pyyaml) or use a JSON→YAML tool:
python3 -c "import sys, json, yaml; yaml.dump(json.load(open('/tmp/openapi.json')), open('docs/openapi.yaml','w'), default_flow_style=False, allow_unicode=True)"
```

- [ ] **Step 7: Commit**

```bash
git add server/src/server.rs server/src/main.rs server/src/openapi.rs .sqlx/ docs/openapi.yaml
git commit -m "feat: wire up password reset routes, inject PasswordResetMapper + EmailService"
```

---

## Task 6: Frontend — `ForgotPasswordPage.vue`

**Files:**
- Create: `frontend/src/components/ForgotPasswordPage.vue`
- Modify: `frontend/src/router/index.ts`
- Modify: `frontend/src/locales/en.json`
- Modify: `frontend/src/locales/pt.json`

- [ ] **Step 1: Add i18n keys to `frontend/src/locales/en.json`**

Inside the `"auth"` object, add after the existing `"login"` section:

```json
"forgotPassword": {
  "title": "Forgot password",
  "subtitle": "Enter your email address and we'll send you a reset link.",
  "emailLabel": "Email",
  "emailPlaceholder": "your@email.com",
  "submit": "Send reset link",
  "sending": "Sending...",
  "successMessage": "If an account exists with that email, you'll receive a reset link shortly.",
  "link": "Forgot password?"
},
```

- [ ] **Step 2: Mirror in `frontend/src/locales/pt.json`**

Inside the `"auth"` object, add:

```json
"forgotPassword": {
  "title": "Esqueci a senha",
  "subtitle": "Insira seu endereço de e-mail e enviaremos um link de redefinição.",
  "emailLabel": "E-mail",
  "emailPlaceholder": "seu@email.com",
  "submit": "Enviar link de redefinição",
  "sending": "Enviando...",
  "successMessage": "Se existir uma conta com esse e-mail, você receberá um link de redefinição em breve.",
  "link": "Esqueceu a senha?"
},
```

- [ ] **Step 3: Create `frontend/src/components/ForgotPasswordPage.vue`**

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import axios from 'axios'

const { t } = useI18n()

const email = ref('')
const loading = ref(false)
const submitted = ref(false)

async function handleSubmit(e: Event) {
  e.preventDefault()
  loading.value = true
  try {
    await axios.post('/api/auth/forgot-password', { email: email.value })
  } catch {
    // Always show the same success message — no user enumeration.
  } finally {
    loading.value = false
    submitted.value = true
  }
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-base-200">
    <div class="card w-full max-w-md bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">{{ t('auth.forgotPassword.title') }}</h2>

        <div v-if="submitted" class="alert alert-success">
          <span>{{ t('auth.forgotPassword.successMessage') }}</span>
        </div>

        <form v-else @submit="handleSubmit" class="space-y-4">
          <p class="text-sm text-base-content/70">
            {{ t('auth.forgotPassword.subtitle') }}
          </p>
          <div class="form-control">
            <label class="label mb-2">
              <span class="label-text">{{ t('auth.forgotPassword.emailLabel') }}</span>
            </label>
            <input
              v-model="email"
              type="email"
              required
              class="input input-bordered w-full"
              :placeholder="t('auth.forgotPassword.emailPlaceholder')"
            />
          </div>
          <button
            type="submit"
            class="btn btn-primary w-full"
            :disabled="loading"
          >
            {{ loading ? t('auth.forgotPassword.sending') : t('auth.forgotPassword.submit') }}
          </button>
        </form>

        <div class="text-center mt-2">
          <router-link to="/login" class="link link-primary text-sm">
            {{ t('auth.login.title') }}
          </router-link>
        </div>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 4: Add the route to `frontend/src/router/index.ts`**

Add after the `/login` route:

```typescript
{
  path: '/forgot-password',
  name: 'ForgotPassword',
  component: () => import('@/components/ForgotPasswordPage.vue'),
  meta: { public: true }
},
```

- [ ] **Step 5: Verify the frontend builds**

```bash
cd frontend && npm run build
```

Expected: exits 0 with no TypeScript errors.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/components/ForgotPasswordPage.vue \
        frontend/src/router/index.ts \
        frontend/src/locales/en.json \
        frontend/src/locales/pt.json
git commit -m "feat: add ForgotPasswordPage with i18n and router route"
```

---

## Task 7: Frontend — `ResetPasswordPage.vue` + LoginPage link

**Files:**
- Create: `frontend/src/components/ResetPasswordPage.vue`
- Modify: `frontend/src/router/index.ts`
- Modify: `frontend/src/locales/en.json`
- Modify: `frontend/src/locales/pt.json`
- Modify: `frontend/src/components/LoginPage.vue`

- [ ] **Step 1: Add i18n keys to `frontend/src/locales/en.json`**

Inside `"auth"`, add after `"forgotPassword"`:

```json
"resetPassword": {
  "title": "Reset password",
  "newPasswordLabel": "New password",
  "newPasswordPlaceholder": "At least 12 characters",
  "confirmPasswordLabel": "Confirm password",
  "confirmPasswordPlaceholder": "Repeat your new password",
  "submit": "Reset password",
  "resetting": "Resetting...",
  "successMessage": "Your password has been reset. Redirecting to login...",
  "passwordMismatch": "Passwords do not match.",
  "invalidToken": "This reset link is invalid or has expired. Please request a new one."
},
```

- [ ] **Step 2: Mirror in `frontend/src/locales/pt.json`**

Inside `"auth"`, add after `"forgotPassword"`:

```json
"resetPassword": {
  "title": "Redefinir senha",
  "newPasswordLabel": "Nova senha",
  "newPasswordPlaceholder": "Mínimo 12 caracteres",
  "confirmPasswordLabel": "Confirmar senha",
  "confirmPasswordPlaceholder": "Repita sua nova senha",
  "submit": "Redefinir senha",
  "resetting": "Redefinindo...",
  "successMessage": "Sua senha foi redefinida. Redirecionando para o login...",
  "passwordMismatch": "As senhas não coincidem.",
  "invalidToken": "Este link de redefinição é inválido ou expirou. Por favor, solicite um novo."
},
```

- [ ] **Step 3: Create `frontend/src/components/ResetPasswordPage.vue`**

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import axios from 'axios'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

const token = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const loading = ref(false)
const errorMessage = ref('')
const success = ref(false)

onMounted(() => {
  const q = route.query.token
  if (!q || typeof q !== 'string') {
    router.replace('/forgot-password')
    return
  }
  token.value = q
})

async function handleSubmit(e: Event) {
  e.preventDefault()
  errorMessage.value = ''

  if (newPassword.value !== confirmPassword.value) {
    errorMessage.value = t('auth.resetPassword.passwordMismatch')
    return
  }

  loading.value = true
  try {
    await axios.post('/api/auth/reset-password', {
      token: token.value,
      new_password: newPassword.value
    })
    success.value = true
    setTimeout(() => router.push('/login'), 2000)
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 400) {
      errorMessage.value = t('auth.resetPassword.invalidToken')
    } else {
      errorMessage.value = t('errors.generic')
    }
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-base-200">
    <div class="card w-full max-w-md bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">{{ t('auth.resetPassword.title') }}</h2>

        <div v-if="success" class="alert alert-success">
          <span>{{ t('auth.resetPassword.successMessage') }}</span>
        </div>

        <template v-else>
          <div v-if="errorMessage" class="alert alert-error">
            <span>{{ errorMessage }}</span>
          </div>

          <form @submit="handleSubmit" class="space-y-4">
            <div class="form-control">
              <label class="label mb-2">
                <span class="label-text">{{ t('auth.resetPassword.newPasswordLabel') }}</span>
              </label>
              <input
                v-model="newPassword"
                type="password"
                required
                class="input input-bordered w-full"
                :placeholder="t('auth.resetPassword.newPasswordPlaceholder')"
                maxlength="128"
              />
            </div>
            <div class="form-control">
              <label class="label mb-2">
                <span class="label-text">{{ t('auth.resetPassword.confirmPasswordLabel') }}</span>
              </label>
              <input
                v-model="confirmPassword"
                type="password"
                required
                class="input input-bordered w-full"
                :placeholder="t('auth.resetPassword.confirmPasswordPlaceholder')"
                maxlength="128"
              />
            </div>
            <button
              type="submit"
              class="btn btn-primary w-full"
              :disabled="loading"
            >
              {{ loading ? t('auth.resetPassword.resetting') : t('auth.resetPassword.submit') }}
            </button>
          </form>
        </template>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 4: Add the route to `frontend/src/router/index.ts`**

Add after the `/forgot-password` route (from Task 6):

```typescript
{
  path: '/reset-password',
  name: 'ResetPassword',
  component: () => import('@/components/ResetPasswordPage.vue'),
  meta: { public: true }
},
```

- [ ] **Step 5: Add "Forgot password?" link to `frontend/src/components/LoginPage.vue`**

After the closing `</div>` of the password input's `form-control` block (around line 92), and only shown when not registering, add:

```vue
<div v-if="!isRegistering" class="text-right -mt-2">
  <router-link to="/forgot-password" class="link link-primary text-sm">
    {{ $t('auth.forgotPassword.link') }}
  </router-link>
</div>
```

- [ ] **Step 6: Verify the frontend builds**

```bash
cd frontend && npm run build
```

Expected: exits 0.

- [ ] **Step 7: Run frontend unit tests and linter**

```bash
cd frontend && npm run test:unit && npm run lint
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/components/ResetPasswordPage.vue \
        frontend/src/components/LoginPage.vue \
        frontend/src/router/index.ts \
        frontend/src/locales/en.json \
        frontend/src/locales/pt.json
git commit -m "feat: add ResetPasswordPage, forgot-password link on LoginPage, i18n keys"
```

---

## Task 8: Final verification

- [ ] **Step 1: Run the complete backend test suite**

```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/anime_calendar \
  AWS_LC_SYS_PREBUILT_NASM=1 cargo test --package server
```

Expected: all tests pass.

- [ ] **Step 2: Run the frontend test suite**

```bash
cd frontend && npm run test:unit
```

Expected: all tests pass.

- [ ] **Step 3: Smoke-test the full flow manually**

Start the backend and frontend dev servers. Then:

1. Go to `/login` — verify "Forgot password?" link is visible below the password field.
2. Click it — verify `/forgot-password` page loads correctly.
3. Submit an email that exists in the DB — check the server log for the reset URL (printed at WARN because SMTP is not configured).
4. Open the logged URL — verify `/reset-password?token=...` loads with the new-password form.
5. Submit a new strong password — verify redirect to `/login`.
6. Log in with the new password — verify it works.
7. Submit the reset URL a second time — verify a 400 error message is shown (token already used).

- [ ] **Step 4: Final commit**

```bash
git add .
git commit -m "chore: verify password reset flow end-to-end"
```
