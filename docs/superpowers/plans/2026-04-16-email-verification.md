# Email Verification on Register — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Block unverified accounts — users must click a link in a verification email before they can log in with a password.

**Architecture:** On register, create the user account, store a SHA-256–hashed one-time token in a new `email_verification_tokens` table, and email the raw token. The `/auth/verify-email` endpoint validates the token, atomically marks the user verified and deletes the token, then issues the auth cookie (so the user is logged in immediately after verifying). Login is blocked for unverified password-based accounts. Google OAuth users are auto-verified at creation time. Existing users are backfilled as verified in the migration so no one is locked out on deploy.

**Tech Stack:** Rust/Actix-Web 4, sqlx 0.8 (PostgreSQL), lettre (SMTP), `rand`/`sha2` (already in Cargo.toml), Vue 3.5 + TypeScript, vue-i18n 11, Pinia, vue-router 4

---

## File Map

### New files
| Path | Responsibility |
|------|---------------|
| `migrations/20260416000000_add_email_verification.sql` | Add `email_verified_at` to users; create `email_verification_tokens` table |
| `server/src/mappers/email_verification.rs` | CRUD for `email_verification_tokens` — `replace_token`, `find_valid_token`, `consume_and_verify` |
| `server/src/controllers/email_verification.rs` | `POST /auth/verify-email`, `POST /auth/resend-verification` |
| `frontend/src/components/VerifyEmailPendingPage.vue` | "Check your inbox" page shown after register |
| `frontend/src/components/VerifyEmailConfirmPage.vue` | SPA landing when user clicks email link — calls backend, then logs user in |

### Modified files
| Path | What changes |
|------|-------------|
| `server/src/services/auth.rs` | Add `generate_random_token()` and `hash_token()` utilities |
| `server/src/controllers/password_reset.rs` | Import token utilities from `services::auth`; remove local duplicates |
| `server/src/entity/user.rs` | Add `email_verified_at: Option<NaiveDateTime>` |
| `server/src/mappers/user.rs` | Add field to all SELECT/RETURNING queries; add `mark_email_verified` method |
| `server/src/error.rs` | Add `EmailNotVerified` (403) and `InvalidVerificationToken` (400) variants |
| `server/src/services/email.rs` | Add `send_verification_email` method |
| `server/src/controllers/auth.rs` | Move `MessageResponse` here; modify `register` (no cookie, send verification email); modify `login` (block unverified); update tests |
| `server/src/controllers/oauth.rs` | Call `mark_email_verified` after `create_user` for new OAuth users |
| `server/src/mappers.rs` | Add `pub mod email_verification` |
| `server/src/controllers.rs` | Add `pub mod email_verification` |
| `server/src/server.rs` | Add `EmailVerificationMapper` param; register new routes |
| `server/src/main.rs` | Instantiate `EmailVerificationMapper` |
| `frontend/src/stores/auth.ts` | Update `register` (no longer sets user state); add `verifyEmail` action |
| `frontend/src/components/Register.vue` | Redirect to `/verify-email/pending` on success |
| `frontend/src/router/index.ts` | Add `/register`, `/verify-email/pending`, `/verify-email` routes |
| `frontend/src/locales/en.json` | Add `auth.verifyEmail.*` keys |
| `frontend/src/locales/pt.json` | Add `auth.verifyEmail.*` keys |

---

## Task 1: Database Migration

**Files:**
- Create: `migrations/20260416000000_add_email_verification.sql`

- [ ] **Step 1: Write the migration**

```sql
-- Add email_verified_at to users.
-- NULL = unverified; NOT NULL = verified at that timestamp.
--
-- Backfill: existing users are treated as already verified so they are not
-- locked out after deploy. Only accounts created after this migration must
-- go through the verification flow.
--
-- email_verification_tokens stores short-lived one-time tokens:
--   - token_hash: SHA-256 hex of the raw token sent in the email link.
--   - expires_at: 24-hour window; expired tokens are rejected.
--   - ON DELETE CASCADE removes tokens when the user is deleted.
--   - UNIQUE on token_hash prevents two concurrent tokens resolving the same hash.

ALTER TABLE users ADD COLUMN email_verified_at TIMESTAMPTZ;

UPDATE users SET email_verified_at = created_at WHERE email_verified_at IS NULL;

CREATE TABLE email_verification_tokens (
    id          SERIAL       PRIMARY KEY,
    user_id     INTEGER      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT         NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ  NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_evtk_token_hash ON email_verification_tokens(token_hash);
CREATE INDEX idx_evtk_user_id    ON email_verification_tokens(user_id);
```

- [ ] **Step 2: Verify cargo check compiles (sqlx will pick up the new table in tests)**

```bash
cargo check --package server
```
Expected: no errors (sqlx in offline mode — `.sqlx/` not yet updated, but the new table has no queries yet).

- [ ] **Step 3: Commit**

```bash
git add migrations/20260416000000_add_email_verification.sql
git commit -m "feat(db): add email_verified_at to users + email_verification_tokens table"
```

---

## Task 2: Token Utilities in services/auth.rs

**Files:**
- Modify: `server/src/services/auth.rs`
- Modify: `server/src/controllers/password_reset.rs`

Both the register handler and the new verification controller need to generate and hash one-time tokens. The `rand`/`sha2` crates are already in `Cargo.toml` (added for password reset). We centralise the utilities in `services/auth.rs` — the natural home for auth crypto — then update `password_reset.rs` to import from there instead of defining them locally.

- [ ] **Step 1: Write the failing test for hash_token**

In `server/src/services/auth.rs`, add to its `#[cfg(test)]` block:

```rust
#[test]
fn hash_token_is_deterministic_and_64_hex_chars() {
    let h1 = hash_token("abc");
    let h2 = hash_token("abc");
    assert_eq!(h1, h2, "same input must produce same hash");
    assert_eq!(h1.len(), 64, "SHA-256 hex is 64 chars");
    assert_ne!(hash_token("abc"), hash_token("xyz"), "different inputs differ");
}

#[test]
fn generate_random_token_is_64_hex_and_unique() {
    let t1 = generate_random_token();
    let t2 = generate_random_token();
    assert_eq!(t1.len(), 64);
    assert_ne!(t1, t2, "two calls should produce different tokens");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --package server services::auth
```
Expected: FAIL — `hash_token` and `generate_random_token` not found.

- [ ] **Step 3: Add the utility functions to services/auth.rs**

At the top of `server/src/services/auth.rs`, add these imports (keep existing ones):

```rust
use rand::RngCore as _;
use sha2::{Digest as _, Sha256};
```

Add these two functions (after the existing public functions):

```rust
/// Generate a cryptographically random 32-byte token as a 64-char hex string.
#[must_use]
pub fn generate_random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 hash a raw token, returning a 64-char hex string.
#[must_use]
pub fn hash_token(raw_token: &str) -> String {
    let hash = Sha256::digest(raw_token.as_bytes());
    hash.iter().map(|b| format!("{b:02x}")).collect()
}
```

- [ ] **Step 4: Update password_reset.rs to use the shared utilities**

In `server/src/controllers/password_reset.rs`, update the imports block — remove the `rand`/`sha2` imports and the two local functions, and add to the `services::auth` import:

```rust
// Replace:
use rand::RngCore as _;
use sha2::{Digest as _, Sha256};
// (and delete the fn generate_raw_token and fn hash_reset_token bodies)

// Add hash_token + generate_random_token to the existing services import:
use crate::services::{
    auth::{generate_random_token, hash_password, hash_token, validate_password_strength},
    email::EmailService,
};
```

Keep `pub fn hash_reset_token` as a thin delegate so existing tests compile without change:

```rust
/// SHA-256 hash a reset token. Delegates to [`crate::services::auth::hash_token`].
#[must_use]
pub fn hash_reset_token(raw_token: &str) -> String {
    hash_token(raw_token)
}
```

In the `forgot_password` body, replace `generate_raw_token()` → `generate_random_token()` and `hash_reset_token` stays the same (it delegates).

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test --package server
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add server/src/services/auth.rs server/src/controllers/password_reset.rs
git commit -m "refactor: centralise token generation/hashing in services::auth"
```

---

## Task 3: User Entity + UserMapper

**Files:**
- Modify: `server/src/entity/user.rs`
- Modify: `server/src/mappers/user.rs`

The `User` struct gains `email_verified_at`. Every query that returns a `User` must be updated. We also add a `mark_email_verified` method used by the oauth controller and (via `consume_and_verify` in Task 4) by the verification mapper.

- [ ] **Step 1: Write a failing test for mark_email_verified**

In `server/src/mappers/user.rs`, add to its `#[cfg(test)]` module:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn mark_email_verified_sets_timestamp(pool: PgPool) {
    let m = mapper(pool);
    let user = m.create_user("verifytest", "verify@example.com", Some("hash")).await.unwrap();
    assert!(user.email_verified_at.is_none(), "new user should be unverified");
    m.mark_email_verified(user.id).await.unwrap();
    let fetched = m.get_user_by_id(user.id).await.unwrap();
    assert!(fetched.email_verified_at.is_some(), "should be verified after mark");
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test --package server mark_email_verified_sets_timestamp
```
Expected: FAIL — field and method not yet defined.

- [ ] **Step 3: Update the User struct**

Replace the entire `server/src/entity/user.rs`:

```rust
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>, // Optional for OAuth users
    pub email_verified_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
```

- [ ] **Step 4: Update all queries in UserMapper**

In `server/src/mappers/user.rs`, apply these four changes:

**get_user_by_email** — add `email_verified_at` to SELECT and struct construction:
```rust
pub async fn get_user_by_email(&self, email: &str) -> ServerResult<User> {
    let user = sqlx::query!(
        "SELECT id, username, email, password_hash, email_verified_at, created_at, updated_at \
         FROM users WHERE email = $1 AND deleted_at IS NULL",
        email
    )
    .fetch_one(&self.db.pool)
    .await?;

    Ok(User {
        id: user.id,
        username: user.username,
        email: user.email,
        password_hash: user.password_hash,
        email_verified_at: user.email_verified_at,
        created_at: user.created_at,
        updated_at: user.updated_at,
    })
}
```

**create_user** — add `email_verified_at` to RETURNING and struct construction:
```rust
pub async fn create_user(
    &self,
    username: &str,
    email: &str,
    password_hash: Option<&str>,
) -> ServerResult<User> {
    let user = sqlx::query!(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) \
         RETURNING id, username, email, password_hash, email_verified_at, created_at, updated_at",
        username,
        email,
        password_hash
    )
    .fetch_one(&self.db.pool)
    .await?;

    Ok(User {
        id: user.id,
        username: user.username,
        email: user.email,
        password_hash: user.password_hash,
        email_verified_at: user.email_verified_at,
        created_at: user.created_at,
        updated_at: user.updated_at,
    })
}
```

**get_user_by_id** — add `email_verified_at` to SELECT and struct construction:
```rust
pub async fn get_user_by_id(&self, id: i32) -> ServerResult<User> {
    let user = sqlx::query!(
        "SELECT id, username, email, password_hash, email_verified_at, created_at, updated_at \
         FROM users WHERE id = $1 AND deleted_at IS NULL",
        id
    )
    .fetch_one(&self.db.pool)
    .await?;

    Ok(User {
        id: user.id,
        username: user.username,
        email: user.email,
        password_hash: user.password_hash,
        email_verified_at: user.email_verified_at,
        created_at: user.created_at,
        updated_at: user.updated_at,
    })
}
```

**update_user** — add `email_verified_at` to RETURNING and struct construction:
```rust
pub async fn update_user(&self, id: i32, username: &str, email: &str) -> ServerResult<User> {
    let user = sqlx::query!(
        "UPDATE users SET username = $1, email = $2, updated_at = NOW() \
         WHERE id = $3 AND deleted_at IS NULL \
         RETURNING id, username, email, password_hash, email_verified_at, created_at, updated_at",
        username,
        email,
        id
    )
    .fetch_one(&self.db.pool)
    .await?;

    Ok(User {
        id: user.id,
        username: user.username,
        email: user.email,
        password_hash: user.password_hash,
        email_verified_at: user.email_verified_at,
        created_at: user.created_at,
        updated_at: user.updated_at,
    })
}
```

- [ ] **Step 5: Add the mark_email_verified method**

Add after `update_user_password` in `server/src/mappers/user.rs`:

```rust
/// Set `email_verified_at` to the current timestamp for the given user.
///
/// # Errors
/// Returns an error if the query fails.
pub async fn mark_email_verified(&self, user_id: i32) -> ServerResult<()> {
    sqlx::query!(
        "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() \
         WHERE id = $1 AND deleted_at IS NULL",
        user_id,
    )
    .execute(&self.db.pool)
    .await?;
    Ok(())
}
```

- [ ] **Step 6: Update seed_user in auth.rs tests to mark users verified**

In `server/src/controllers/auth.rs`, in the `integration_tests` module, update `seed_user` so that existing login tests still pass (seeded users are pre-verified, as they represent accounts that existed before the verification flow):

```rust
async fn seed_user(pool: &PgPool, username: &str, email: &str) -> i32 {
    let hash = hash_password(STRONG_PW).unwrap();
    let mapper = UserMapper::from_pool(pool.clone());
    let user = mapper
        .create_user(username, email, Some(&hash))
        .await
        .unwrap();
    mapper.mark_email_verified(user.id).await.unwrap();
    user.id
}
```

Do the same in `server/src/controllers/password_reset.rs` tests:

```rust
async fn seed_user(pool: &PgPool, email: &str) -> i32 {
    let mapper = UserMapper::from_pool(pool.clone());
    let user = mapper
        .create_user("tester", email, Some(&hash_password(STRONG_PW).unwrap()))
        .await
        .unwrap();
    mapper.mark_email_verified(user.id).await.unwrap();
    user.id
}
```

- [ ] **Step 7: Run tests to verify they pass**

```bash
cargo test --package server
```
Expected: all existing tests pass (sqlx will complain about stale offline cache — that's OK; we regenerate it in Task 11).

> **Note:** If sqlx refuses to compile due to stale `.sqlx/` cache, set `SQLX_OFFLINE=false` and provide a live `DATABASE_URL`, or hold off on running tests until Task 11. You can also do a `cargo check` without the test flag as a quick compile sanity check.

- [ ] **Step 8: Commit**

```bash
git add server/src/entity/user.rs server/src/mappers/user.rs server/src/controllers/auth.rs server/src/controllers/password_reset.rs
git commit -m "feat: add email_verified_at to User entity and UserMapper"
```

---

## Task 4: EmailVerificationMapper

**Files:**
- Create: `server/src/mappers/email_verification.rs`

Follows the exact same pattern as `PasswordResetMapper`. The key difference: instead of marking a token `used_at`, we delete it and mark the user verified atomically in `consume_and_verify`.

- [ ] **Step 1: Write the failing tests**

Create `server/src/mappers/email_verification.rs` with only the test module to start:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mappers::user::UserMapper;
    use crate::services::auth::hash_password;
    use sqlx::PgPool;

    async fn seed_user(pool: &PgPool) -> i32 {
        let mapper = UserMapper::from_pool(pool.clone());
        let user = mapper
            .create_user("evtest", "ev@test.com", Some(&hash_password("TestPass12!@").unwrap()))
            .await
            .unwrap();
        user.id
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn replace_token_creates_and_find_valid_returns_it(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = EmailVerificationMapper::from_pool(pool);
        mapper.replace_token(user_id, "hash_abc").await.unwrap();
        let token = mapper.find_valid_token("hash_abc").await.unwrap();
        assert_eq!(token.user_id, user_id);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn replace_token_removes_previous_token(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = EmailVerificationMapper::from_pool(pool);
        mapper.replace_token(user_id, "old_hash").await.unwrap();
        mapper.replace_token(user_id, "new_hash").await.unwrap();
        assert!(mapper.find_valid_token("old_hash").await.is_err(), "old token must be gone");
        mapper.find_valid_token("new_hash").await.unwrap();
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn find_valid_token_not_found_returns_error(pool: PgPool) {
        let mapper = EmailVerificationMapper::from_pool(pool);
        assert!(mapper.find_valid_token("nonexistent").await.is_err());
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn consume_and_verify_deletes_token_and_marks_user_verified(pool: PgPool) {
        let user_id = seed_user(&pool).await;
        let mapper = EmailVerificationMapper::from_pool(pool.clone());
        mapper.replace_token(user_id, "consume_hash").await.unwrap();
        let token = mapper.find_valid_token("consume_hash").await.unwrap();

        mapper.consume_and_verify(token.id, user_id).await.unwrap();

        // Token is gone
        assert!(mapper.find_valid_token("consume_hash").await.is_err());

        // User is now verified
        let verified_at: Option<chrono::NaiveDateTime> =
            sqlx::query_scalar("SELECT email_verified_at FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(verified_at.is_some(), "user should be verified after consume_and_verify");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --package server email_verification
```
Expected: FAIL — `EmailVerificationMapper` not defined.

- [ ] **Step 3: Implement EmailVerificationMapper**

Replace `server/src/mappers/email_verification.rs` with the full implementation:

```rust
use crate::{
    config::database::Database as DatabaseConfig,
    error::Error,
    mappers::database::Database,
    ServerResult,
};

#[derive(Clone)]
pub struct EmailVerificationMapper {
    db: Database,
}

/// Identifies a valid, unexpired token row.
#[must_use]
pub struct EmailVerificationToken {
    pub id: i32,
    pub user_id: i32,
}

impl EmailVerificationMapper {
    /// # Errors
    /// Fails if the database connection cannot be established.
    pub async fn new(config: DatabaseConfig) -> ServerResult<Self> {
        Ok(Self { db: Database::new(config).await? })
    }

    /// Construct from a bare pool — for integration tests only.
    #[cfg(test)]
    pub fn from_pool(pool: sqlx::PgPool) -> Self {
        Self { db: Database { pool } }
    }

    /// Atomically delete any existing token for `user_id` and insert a new one
    /// that expires 24 hours from now.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn replace_token(&self, user_id: i32, token_hash: &str) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "DELETE FROM email_verification_tokens WHERE user_id = $1",
            user_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "INSERT INTO email_verification_tokens (user_id, token_hash, expires_at) \
             VALUES ($1, $2, NOW() + INTERVAL '24 hours')",
            user_id,
            token_hash,
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

    /// Return the token row if it exists and has not expired.
    ///
    /// # Errors
    /// Returns `Error::InvalidVerificationToken` if no valid token matches.
    pub async fn find_valid_token(
        &self,
        token_hash: &str,
    ) -> ServerResult<EmailVerificationToken> {
        let row = sqlx::query!(
            "SELECT id, user_id FROM email_verification_tokens \
             WHERE token_hash = $1 AND expires_at > NOW()",
            token_hash,
        )
        .fetch_one(&self.db.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::InvalidVerificationToken,
            other => Error::Database(other),
        })?;

        Ok(EmailVerificationToken { id: row.id, user_id: row.user_id })
    }

    /// Atomically delete the verification token AND mark the user's email as
    /// verified. Both changes succeed or both are rolled back.
    ///
    /// # Errors
    /// Rolls back and propagates the error if either query fails.
    pub async fn consume_and_verify(&self, token_id: i32, user_id: i32) -> ServerResult<()> {
        let mut tx = self.db.pool.begin().await?;

        if let Err(e) = sqlx::query!(
            "DELETE FROM email_verification_tokens WHERE id = $1",
            token_id,
        )
        .execute(&mut *tx)
        .await
        {
            tx.rollback().await?;
            return Err(e.into());
        }

        if let Err(e) = sqlx::query!(
            "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND deleted_at IS NULL",
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
    // ... (the tests written in Step 1 go here)
}
```

- [ ] **Step 4: Register the module in mappers.rs**

In `server/src/mappers.rs`, add:

```rust
pub mod email_verification;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test --package server email_verification
```
Expected: all 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add server/src/mappers/email_verification.rs server/src/mappers.rs
git commit -m "feat: add EmailVerificationMapper with replace_token, find_valid_token, consume_and_verify"
```

---

## Task 5: Error Variants + EmailService

**Files:**
- Modify: `server/src/error.rs`
- Modify: `server/src/services/email.rs`

- [ ] **Step 1: Add two new error variants to error.rs**

In the `Error` enum in `server/src/error.rs`, add after `InvalidResetToken`:

```rust
#[error("Email not verified — please check your inbox")]
EmailNotVerified,
#[error("Invalid or expired verification token")]
InvalidVerificationToken,
```

In the `status_code` match, add these arms:

```rust
Self::EmailNotVerified => StatusCode::FORBIDDEN,
Self::InvalidVerificationToken => StatusCode::BAD_REQUEST,
```

- [ ] **Step 2: Write a failing compile test (just cargo check)**

```bash
cargo check --package server
```
Expected: FAIL — `InvalidVerificationToken` used in `email_verification.rs` (Task 4) should now resolve.

- [ ] **Step 3: Add send_verification_email to EmailService**

In `server/src/services/email.rs`, add after `send_password_reset`:

```rust
/// Send an email verification link.
///
/// If `smtp.host` is empty (e.g. local dev), logs the URL at WARN level
/// instead of attempting a connection.
///
/// # Errors
/// Fails if SMTP connection fails or the email cannot be built/sent.
pub async fn send_verification_email(
    &self,
    to_email: &str,
    verify_url: &str,
) -> ServerResult<()> {
    if self.config.host.is_empty() {
        warn!(
            "SMTP not configured — email verification URL for {to_email}: {verify_url}"
        );
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
        .subject("Verify your Anime Calendar email address")
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Welcome to Anime Calendar!\n\n\
             Please verify your email address by clicking the link below \
             (the link expires in 24 hours):\n\n\
             {verify_url}\n\n\
             If you did not create an account, you can safely ignore this email."
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
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo check --package server
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add server/src/error.rs server/src/services/email.rs
git commit -m "feat: add EmailNotVerified/InvalidVerificationToken errors and send_verification_email"
```

---

## Task 6: Move MessageResponse + Modify register

**Files:**
- Modify: `server/src/controllers/auth.rs`
- Modify: `server/src/controllers/password_reset.rs`

The `register` handler no longer issues an auth cookie. Instead it creates the user, sends a verification email, and returns a plain message. We move `MessageResponse` to `auth.rs` so both controllers share it.

- [ ] **Step 1: Update the existing register test to expect the new behaviour**

In `server/src/controllers/auth.rs`, in the `integration_tests` module, replace `register_new_user_sets_cookie_and_returns_username` with:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn register_new_user_returns_200_with_message_and_no_cookie(pool: PgPool) {
    use crate::mappers::email_verification::EmailVerificationMapper;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
            .app_data(web::Data::new(
                EmailVerificationMapper::from_pool(pool),
            ))
            .app_data(web::Data::new(EmailService::new(
                crate::config::server::SmtpConfig::default(),
            )))
            .app_data(web::Data::new(AppBaseUrl::new(
                "http://localhost:5173".to_string(),
            )))
            .service(register),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/register")
        .set_json(serde_json::json!({
            "username": "carol",
            "email": "carol@test.com",
            "password": STRONG_PW
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // No auth cookie — user must verify email before logging in
    let has_auth_cookie = resp
        .headers()
        .get("Set-Cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("auth_token="))
        .unwrap_or(false);
    assert!(!has_auth_cookie, "register must not issue auth cookie before verification");

    let body: Value = test::read_body_json(resp).await;
    assert!(body.get("message").is_some(), "response must contain a 'message' field");
}
```

Also add the new imports at the top of the `integration_tests` module:

```rust
use crate::{
    config::server::{AppBaseUrl, SmtpConfig},
    mappers::email_verification::EmailVerificationMapper,
    services::email::EmailService,
};
```

- [ ] **Step 2: Run to verify the test fails**

```bash
cargo test --package server register_new_user_returns_200_with_message_and_no_cookie
```
Expected: FAIL — register still returns a cookie.

- [ ] **Step 3: Move MessageResponse to auth.rs**

In `server/src/controllers/auth.rs`, add after `AuthResponse`:

```rust
#[derive(Serialize, utoipa::ToSchema)]
pub struct MessageResponse {
    pub message: String,
}
```

In `server/src/controllers/password_reset.rs`, remove the local `MessageResponse` definition and add to the imports:

```rust
use crate::controllers::auth::{ErrorResponse, MessageResponse};
```

- [ ] **Step 4: Update the register handler**

In `server/src/controllers/auth.rs`, add these imports:

```rust
use crate::{
    config::server::AppBaseUrl,
    mappers::email_verification::EmailVerificationMapper,
    services::{
        auth::{generate_random_token, hash_token},
        email::EmailService,
    },
};
use log::error;
```

Replace the full `register` function with:

```rust
#[utoipa::path(
    post,
    path = "/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration accepted — verification email sent", body = MessageResponse),
        (status = 400, description = "Invalid request, weak password, or duplicate email", body = ErrorResponse),
    )
)]
#[post("/register")]
pub async fn register(
    db: web::Data<UserMapper>,
    verification_mapper: web::Data<EmailVerificationMapper>,
    email_service: web::Data<EmailService>,
    app_base_url: web::Data<AppBaseUrl>,
    user_data: web::Json<RegisterRequest>,
) -> HttpResponse {
    // Check if user already exists
    if (db.get_user_by_email(&user_data.email).await).is_ok() {
        return Error::UserAlreadyExists.error_response();
    }

    if user_data.username.len() > 50 {
        return Error::InvalidRequest.error_response();
    }

    if !is_valid_email(&user_data.email) {
        return Error::InvalidRequest.error_response();
    }

    if user_data.password.expose_secret().len() > 128 {
        return Error::InvalidRequest.error_response();
    }

    if !validate_password_strength(user_data.password.expose_secret()) {
        return Error::InvalidPassword.error_response();
    }

    let hashed_password = match hash_password(user_data.password.expose_secret()) {
        Ok(h) => h,
        Err(e) => return e.error_response(),
    };

    let user = match db
        .create_user(&user_data.username, &user_data.email, Some(&hashed_password))
        .await
    {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    let raw_token = generate_random_token();
    let token_hash = hash_token(&raw_token);
    let verify_url = format!(
        "{}/verify-email?token={raw_token}",
        app_base_url.as_str()
    );

    if let Err(e) = verification_mapper.replace_token(user.id, &token_hash).await {
        error!("{e}");
        return e.error_response();
    }

    if let Err(e) = email_service
        .send_verification_email(&user.email, &verify_url)
        .await
    {
        error!("{e}");
        return e.error_response();
    }

    HttpResponse::Ok().json(MessageResponse {
        message: "Verification email sent. Please check your inbox.".into(),
    })
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test --package server
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add server/src/controllers/auth.rs server/src/controllers/password_reset.rs
git commit -m "feat: register sends verification email instead of issuing auth cookie"
```

---

## Task 7: Block Unverified Users at Login

**Files:**
- Modify: `server/src/controllers/auth.rs`

After validating the password, check `email_verified_at`. If the user hasn't verified their email, return `Error::EmailNotVerified` (403).

- [ ] **Step 1: Write the failing test**

In `server/src/controllers/auth.rs`, in the `integration_tests` module, add:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn login_unverified_user_returns_403(pool: PgPool) {
    // Create a user WITHOUT calling mark_email_verified
    let hash = hash_password(STRONG_PW).unwrap();
    UserMapper::from_pool(pool.clone())
        .create_user("unverified", "unverified@test.com", Some(&hash))
        .await
        .unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool)))
            .app_data(jwt_data())
            .app_data(cookie_data())
            .service(login),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/login")
        .set_json(serde_json::json!({
            "email": "unverified@test.com",
            "password": STRONG_PW
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::FORBIDDEN
    );
}
```

- [ ] **Step 2: Run to verify it fails**

```bash
cargo test --package server login_unverified_user_returns_403
```
Expected: FAIL — returns 200 instead of 403.

- [ ] **Step 3: Add the verification check to login**

In the `login` handler in `server/src/controllers/auth.rs`, update the `Some(hash)` arm:

```rust
match user.password_hash {
    Some(hash) => {
        if validate_password(credentials.password.expose_secret(), &hash) {
            // Block login if email has not been verified yet.
            if user.email_verified_at.is_none() {
                return Error::EmailNotVerified.error_response();
            }

            let token = match generate_token(&user.id, jwt_secret.expose_secret()) {
                Ok(token) => token,
                Err(e) => return e.error_response(),
            };

            let cookie = build_auth_cookie(token, &cookie_settings);
            HttpResponse::Ok()
                .cookie(cookie)
                .json(AuthResponse { username: user.username })
        } else {
            Error::Unauthorised.error_response()
        }
    }
    None => Error::Unauthorised.error_response(),
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test --package server
```
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add server/src/controllers/auth.rs
git commit -m "feat: block login for unverified email addresses (403)"
```

---

## Task 8: Auto-Verify OAuth Users

**Files:**
- Modify: `server/src/controllers/oauth.rs`

Google OAuth users don't go through the email verification flow — their email is pre-verified by Google. When a new OAuth user is created, immediately call `mark_email_verified`.

- [ ] **Step 1: Write the failing test**

In `server/src/controllers/oauth.rs`, add a unit-level integration test at the bottom. Because `google_oauth.validate_id_token` calls Google's servers, we only test the mapper interaction. Add to an `#[cfg(test)]` block:

```rust
#[cfg(test)]
mod tests {
    use crate::{mappers::user::UserMapper, services::auth::hash_password};
    use sqlx::PgPool;

    #[sqlx::test(migrations = "../migrations")]
    async fn oauth_create_user_is_unverified_until_mark_called(pool: PgPool) {
        let mapper = UserMapper::from_pool(pool.clone());
        let user = mapper.create_user("oauthtest", "oauth@test.com", None).await.unwrap();
        assert!(user.email_verified_at.is_none(), "freshly created user is unverified");
        mapper.mark_email_verified(user.id).await.unwrap();
        let fetched = mapper.get_user_by_id(user.id).await.unwrap();
        assert!(fetched.email_verified_at.is_some());
    }
}
```

- [ ] **Step 2: Run to verify it passes (it's testing the mapper, which we already built)**

```bash
cargo test --package server oauth_create_user_is_unverified_until_mark_called
```
Expected: PASS.

- [ ] **Step 3: Update the OAuth controller**

In `server/src/controllers/oauth.rs`, add `UserMapper` to the imports (it's already there) and add a `mark_email_verified` call for newly created users:

```rust
Err(_) => {
    match user_mapper
        .create_user(&google_user.id, &google_user.email, None)
        .await
    {
        Ok(user) => {
            // Mark the email as verified — Google has already confirmed ownership.
            if let Err(e) = user_mapper.mark_email_verified(user.id).await {
                error!("Failed to mark OAuth user {} as verified: {e}", user.id);
                return e.error_response();
            }
            user
        }
        Err(e) => return e.error_response(),
    }
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo check --package server
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add server/src/controllers/oauth.rs
git commit -m "feat: auto-verify email for new Google OAuth users"
```

---

## Task 9: Email Verification Controller

**Files:**
- Create: `server/src/controllers/email_verification.rs`

Two endpoints:
- `POST /auth/verify-email` — validates the token, marks the user verified (via `consume_and_verify`), then issues the auth cookie so the user is immediately logged in.
- `POST /auth/resend-verification` — always returns 200 (anti-enumeration); generates a fresh token and resends the email if the account exists and is unverified.

- [ ] **Step 1: Write the failing tests**

Create `server/src/controllers/email_verification.rs` with only the test module first:

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        config::server::{AppBaseUrl, CookieSettings, JwtSecret, SmtpConfig},
        mappers::{email_verification::EmailVerificationMapper, user::UserMapper},
        services::{auth::{hash_password, hash_token}, email::EmailService},
    };
    use actix_web::{http::StatusCode, test, web, App};
    use secrecy::SecretString;
    use sqlx::PgPool;

    const STRONG_PW: &str = "SecurePass12!@";
    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }
    fn cookie_data() -> web::Data<CookieSettings> {
        web::Data::new(CookieSettings { secure: false })
    }
    fn dev_email() -> web::Data<EmailService> {
        web::Data::new(EmailService::new(SmtpConfig::default()))
    }
    fn base_url() -> web::Data<AppBaseUrl> {
        web::Data::new(AppBaseUrl::new("http://localhost:5173".to_string()))
    }

    async fn seed_unverified_user(pool: &PgPool) -> (i32, String) {
        let mapper = UserMapper::from_pool(pool.clone());
        let user = mapper
            .create_user("verifyme", "verify@test.com", Some(&hash_password(STRONG_PW).unwrap()))
            .await
            .unwrap();
        (user.id, user.email)
    }

    // ─── POST /auth/verify-email ─────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn verify_email_valid_token_issues_cookie_and_marks_verified(pool: PgPool) {
        let (user_id, _email) = seed_unverified_user(&pool).await;
        let ev_mapper = EmailVerificationMapper::from_pool(pool.clone());
        let raw = "validtoken123";
        ev_mapper.replace_token(user_id, &hash_token(raw)).await.unwrap();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(ev_mapper))
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(verify_email),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/verify-email")
            .set_json(serde_json::json!({ "token": raw }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let set_cookie = resp
            .headers()
            .get("Set-Cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(set_cookie.contains("auth_token="), "must issue auth cookie");
        assert!(set_cookie.contains("HttpOnly"), "cookie must be HttpOnly");

        let verified_at: Option<chrono::NaiveDateTime> =
            sqlx::query_scalar("SELECT email_verified_at FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(verified_at.is_some(), "user must be verified in DB");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn verify_email_invalid_token_returns_400(pool: PgPool) {
        let ev_mapper = EmailVerificationMapper::from_pool(pool.clone());
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(ev_mapper))
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(verify_email),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/verify-email")
            .set_json(serde_json::json!({ "token": "not-a-real-token" }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::BAD_REQUEST);
    }

    // ─── POST /auth/resend-verification ─────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn resend_verification_known_unverified_email_returns_200(pool: PgPool) {
        let (user_id, _) = seed_unverified_user(&pool).await;
        let ev_mapper = EmailVerificationMapper::from_pool(pool.clone());
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(ev_mapper.clone()))
                .app_data(dev_email())
                .app_data(base_url())
                .service(resend_verification),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/resend-verification")
            .set_json(serde_json::json!({ "email": "verify@test.com" }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

        // Token row should exist after resend
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM email_verification_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1, "one token row must exist after resend");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn resend_verification_unknown_email_also_returns_200(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(pool)))
                .app_data(dev_email())
                .app_data(base_url())
                .service(resend_verification),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/resend-verification")
            .set_json(serde_json::json!({ "email": "nobody@test.com" }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }
}
```

- [ ] **Step 2: Run to verify they fail**

```bash
cargo test --package server email_verification
```
Expected: FAIL — handlers not yet defined.

- [ ] **Step 3: Implement the controller**

Replace `server/src/controllers/email_verification.rs` with:

```rust
use actix_web::{post, web, HttpResponse, ResponseError};
use log::error;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    config::server::{AppBaseUrl, CookieSettings, JwtSecret},
    controllers::auth::{build_auth_cookie, AuthResponse, ErrorResponse, MessageResponse},
    error::Error,
    mappers::{email_verification::EmailVerificationMapper, user::UserMapper},
    services::{
        auth::{generate_random_token, generate_token, hash_token},
        email::EmailService,
    },
};

#[derive(Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ResendVerificationRequest {
    pub email: String,
}

const RESEND_RESPONSE: &str =
    "If an unverified account exists with that email, a new verification link has been sent.";

#[utoipa::path(
    post,
    path = "/auth/verify-email",
    tag = "auth",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified — auth cookie issued", body = AuthResponse),
        (status = 400, description = "Invalid or expired token", body = ErrorResponse),
    )
)]
#[post("/auth/verify-email")]
pub async fn verify_email(
    verification_mapper: web::Data<EmailVerificationMapper>,
    user_mapper: web::Data<UserMapper>,
    jwt_secret: web::Data<JwtSecret>,
    cookie_settings: web::Data<CookieSettings>,
    body: web::Json<VerifyEmailRequest>,
) -> HttpResponse {
    let token_hash = hash_token(&body.token);

    let token = match verification_mapper.find_valid_token(&token_hash).await {
        Ok(t) => t,
        Err(_) => return Error::InvalidVerificationToken.error_response(),
    };

    if let Err(e) = verification_mapper
        .consume_and_verify(token.id, token.user_id)
        .await
    {
        error!("{e}");
        return e.error_response();
    }

    let user = match user_mapper.get_user_by_id(token.user_id).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    let jwt = match generate_token(&user.id, jwt_secret.expose_secret()) {
        Ok(t) => t,
        Err(e) => return e.error_response(),
    };

    let cookie = build_auth_cookie(jwt, &cookie_settings);
    HttpResponse::Ok()
        .cookie(cookie)
        .json(AuthResponse { username: user.username })
}

#[utoipa::path(
    post,
    path = "/auth/resend-verification",
    tag = "auth",
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Always returned — prevents user enumeration", body = MessageResponse),
        (status = 500, description = "DB or email error", body = ErrorResponse),
    )
)]
#[post("/auth/resend-verification")]
pub async fn resend_verification(
    user_mapper: web::Data<UserMapper>,
    verification_mapper: web::Data<EmailVerificationMapper>,
    email_service: web::Data<EmailService>,
    app_base_url: web::Data<AppBaseUrl>,
    body: web::Json<ResendVerificationRequest>,
) -> HttpResponse {
    let ok = HttpResponse::Ok()
        .json(MessageResponse { message: RESEND_RESPONSE.into() });

    let Ok(user) = user_mapper.get_user_by_email(&body.email).await else {
        return ok;
    };

    // Already verified — silently do nothing
    if user.email_verified_at.is_some() {
        return ok;
    }

    let raw_token = generate_random_token();
    let token_hash = hash_token(&raw_token);
    let verify_url = format!(
        "{}/verify-email?token={raw_token}",
        app_base_url.as_str()
    );

    if let Err(e) = verification_mapper.replace_token(user.id, &token_hash).await {
        error!("{e}");
        return e.error_response();
    }

    if let Err(e) = email_service
        .send_verification_email(&user.email, &verify_url)
        .await
    {
        error!("{e}");
        return e.error_response();
    }

    ok
}

#[cfg(test)]
mod integration_tests {
    // ... (the tests written in Step 1 go here)
}
```

- [ ] **Step 4: Register the module in controllers.rs**

In `server/src/controllers.rs`, add:

```rust
pub mod email_verification;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test --package server email_verification
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add server/src/controllers/email_verification.rs server/src/controllers.rs
git commit -m "feat: add verify-email and resend-verification endpoints"
```

---

## Task 10: Wire Up in server.rs and main.rs

**Files:**
- Modify: `server/src/server.rs`
- Modify: `server/src/main.rs`

- [ ] **Step 1: Update server.rs**

In `server/src/server.rs`, add `EmailVerificationMapper` to imports:

```rust
use crate::{
    // ... existing imports ...
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, email_verification::EmailVerificationMapper,
        google_oauth::GoogleOauth, password_reset::PasswordResetMapper, user::UserMapper,
        user_settings::UserSettingsMapper,
    },
    // ...
    controllers::{auth, cache_metrics, calendar, email_verification, item, items, oauth,
                  password_reset, user},
};
```

Add `EmailVerificationMapper` to the `start` function signature (after `token_mapper`):

```rust
pub fn start(
    config: ServerConfig,
    anilist: Anilist,
    google_oauth: GoogleOauth,
    cache: Cache,
    user_mapper: UserMapper,
    calendar_mapper: CalendarMapper,
    user_settings_mapper: UserSettingsMapper,
    token_mapper: PasswordResetMapper,
    verification_mapper: EmailVerificationMapper,
    email_service: EmailService,
) -> ServerResult<Server> {
```

Add `.app_data(web::Data::new(verification_mapper.clone()))` after the `token_mapper` line:

```rust
.app_data(web::Data::new(token_mapper.clone()))
.app_data(web::Data::new(verification_mapper.clone()))
.app_data(web::Data::new(email_service.clone()))
```

Register the two new services (after `password_reset::reset_password`):

```rust
.service(email_verification::verify_email)
.service(email_verification::resend_verification)
```

- [ ] **Step 2: Update main.rs**

In `server/src/main.rs`, add `EmailVerificationMapper` to imports:

```rust
use crate::{
    // ...
    mappers::{
        anilist::Anilist, calendar::CalendarMapper, email_verification::EmailVerificationMapper,
        google_oauth::GoogleOauth, password_reset::PasswordResetMapper, user::UserMapper,
        user_settings::UserSettingsMapper,
    },
    // ...
};
```

Add instantiation after `token_mapper`:

```rust
let token_mapper = PasswordResetMapper::new(db_config.clone()).await?;
let verification_mapper = EmailVerificationMapper::new(db_config.clone()).await?;
let email_service = EmailService::new(settings.smtp.clone());
```

Pass it to `server::start`:

```rust
Ok(server::start(
    settings,
    anilist,
    google_oauth,
    cache,
    user_mapper,
    calendar_mapper,
    user_settings_mapper,
    token_mapper,
    verification_mapper,
    email_service,
)?
.await?)
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo check --package server
```
Expected: no errors.

- [ ] **Step 4: Run all tests**

```bash
cargo test --package server
```
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add server/src/server.rs server/src/main.rs
git commit -m "feat: wire EmailVerificationMapper and register verify-email routes"
```

---

## Task 11: Regenerate .sqlx/ Offline Cache

**Files:**
- Modify: `.sqlx/` directory (generated)

sqlx verifies queries at compile time. All the new `sqlx::query!` calls (plus the updated queries with `email_verified_at`) are not yet in `.sqlx/`, so CI builds will fail without regenerating it.

- [ ] **Step 1: Run sqlx prepare against a live database**

```bash
DATABASE_URL=postgresql://user:pass@host:port/dbname cargo sqlx prepare --workspace
```
Expected: `.sqlx/` directory is updated with new query fingerprints.

- [ ] **Step 2: Verify offline build passes**

```bash
cargo build
```
Expected: builds successfully without `DATABASE_URL`.

- [ ] **Step 3: Run all tests one more time**

```bash
cargo test --package server
```
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add .sqlx/
git commit -m "chore: regenerate sqlx offline query cache for email verification"
```

---

## Task 12: Frontend i18n Keys

**Files:**
- Modify: `frontend/src/locales/en.json`
- Modify: `frontend/src/locales/pt.json`

Both files must be updated together — the app will throw a console warning for any missing translation key.

- [ ] **Step 1: Add English keys**

In `frontend/src/locales/en.json`, add inside the `"auth"` object (after `"resetPassword"`):

```json
"verifyEmail": {
  "pending": {
    "title": "Check your email",
    "subtitle": "We've sent a verification link to your email address. Click the link to activate your account.",
    "resendButton": "Resend email",
    "resending": "Resending...",
    "resendSuccess": "Verification email resent. Please check your inbox.",
    "resendError": "Failed to resend. Please try again."
  },
  "confirm": {
    "verifying": "Verifying your email...",
    "success": "Email verified! You are now logged in.",
    "error": "This verification link is invalid or has expired.",
    "loginLink": "Go to login"
  }
}
```

- [ ] **Step 2: Add Portuguese keys**

In `frontend/src/locales/pt.json`, add the equivalent inside `"auth"`:

```json
"verifyEmail": {
  "pending": {
    "title": "Verifique o seu email",
    "subtitle": "Enviámos um link de verificação para o seu endereço de email. Clique no link para ativar a sua conta.",
    "resendButton": "Reenviar email",
    "resending": "A reenviar...",
    "resendSuccess": "Email de verificação reenviado. Por favor verifique a sua caixa de entrada.",
    "resendError": "Falha ao reenviar. Por favor tente novamente."
  },
  "confirm": {
    "verifying": "A verificar o seu email...",
    "success": "Email verificado! Já está autenticado.",
    "error": "Este link de verificação é inválido ou expirou.",
    "loginLink": "Ir para o login"
  }
}
```

- [ ] **Step 3: Verify frontend still compiles**

```bash
cd frontend && npm run build
```
Expected: no type errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/locales/en.json frontend/src/locales/pt.json
git commit -m "feat(i18n): add email verification translation keys"
```

---

## Task 13: VerifyEmailPendingPage.vue

**Files:**
- Create: `frontend/src/components/VerifyEmailPendingPage.vue`

Shown immediately after registration. Tells the user to check their email. Has a resend button that calls `POST /auth/resend-verification`. The page receives the user's email via Vue Router navigation state (set by `Register.vue` in Task 15) — this avoids exposing the email in the URL.

- [ ] **Step 1: Write the component**

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import api from '@/config/api'
import { i18n } from '@/plugins/i18n'

const { t } = i18n.global
const router = useRouter()

// Email is passed via router navigation state from Register.vue
const email = (history.state?.email as string) ?? ''
const resending = ref(false)
const resendMessage = ref<string | null>(null)
const resendError = ref<string | null>(null)

const handleResend = async () => {
  resending.value = true
  resendMessage.value = null
  resendError.value = null

  try {
    await api.post('/auth/resend-verification', { email })
    resendMessage.value = t('auth.verifyEmail.pending.resendSuccess')
  } catch {
    resendError.value = t('auth.verifyEmail.pending.resendError')
  } finally {
    resending.value = false
  }
}
</script>

<template>
  <div class="min-h-[calc(100vh-6rem)] bg-base-200 flex items-center justify-center">
    <div class="card bg-base-100 w-full max-w-md shadow-xl">
      <div class="card-body items-center text-center gap-4">
        <h2 class="card-title text-2xl">{{ $t('auth.verifyEmail.pending.title') }}</h2>
        <p class="text-base-content/70">{{ $t('auth.verifyEmail.pending.subtitle') }}</p>

        <div v-if="resendMessage" class="alert alert-success w-full">
          {{ resendMessage }}
        </div>
        <div v-if="resendError" class="alert alert-error w-full">
          {{ resendError }}
        </div>

        <button
          v-if="email"
          :disabled="resending"
          class="btn btn-outline btn-sm"
          data-testid="resend-button"
          @click="handleResend"
        >
          {{ resending ? $t('auth.verifyEmail.pending.resending') : $t('auth.verifyEmail.pending.resendButton') }}
        </button>
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Verify it builds**

```bash
cd frontend && npm run build
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/components/VerifyEmailPendingPage.vue
git commit -m "feat: add VerifyEmailPendingPage (post-register check-your-inbox screen)"
```

---

## Task 14: VerifyEmailConfirmPage.vue

**Files:**
- Create: `frontend/src/components/VerifyEmailConfirmPage.vue`

The SPA route `/verify-email` that users land on after clicking the email link. On mount it calls `POST /auth/verify-email` with the `token` query param. On success the backend issues the auth cookie, we set auth store state, and redirect to `/my-calendars`.

- [ ] **Step 1: Write the component**

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { i18n } from '@/plugins/i18n'

const { t } = i18n.global
const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const status = ref<'verifying' | 'success' | 'error'>('verifying')

onMounted(async () => {
  const token = route.query.token as string | undefined
  if (!token) {
    status.value = 'error'
    return
  }

  try {
    const username = await authStore.verifyEmail(token)
    status.value = 'success'
    // Brief pause so the user sees the success message, then redirect
    setTimeout(() => router.push('/my-calendars'), 1500)
  } catch {
    status.value = 'error'
  }
})
</script>

<template>
  <div class="min-h-[calc(100vh-6rem)] bg-base-200 flex items-center justify-center">
    <div class="card bg-base-100 w-full max-w-md shadow-xl">
      <div class="card-body items-center text-center gap-4">

        <span
          v-if="status === 'verifying'"
          class="loading loading-spinner loading-lg"
          data-testid="verifying-spinner"
        />
        <p v-if="status === 'verifying'">{{ $t('auth.verifyEmail.confirm.verifying') }}</p>

        <div v-if="status === 'success'" class="alert alert-success" data-testid="success-message">
          {{ $t('auth.verifyEmail.confirm.success') }}
        </div>

        <template v-if="status === 'error'">
          <div class="alert alert-error" data-testid="error-message">
            {{ $t('auth.verifyEmail.confirm.error') }}
          </div>
          <router-link to="/login" class="btn btn-primary btn-sm">
            {{ $t('auth.verifyEmail.confirm.loginLink') }}
          </router-link>
        </template>

      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Verify it builds**

```bash
cd frontend && npm run build
```
Expected: FAIL — `authStore.verifyEmail` not yet defined (that's added in Task 15).

- [ ] **Step 3: Commit the file (will be fixed in Task 15)**

```bash
git add frontend/src/components/VerifyEmailConfirmPage.vue
git commit -m "feat: add VerifyEmailConfirmPage (email link landing)"
```

---

## Task 15: Auth Store + Router + Register.vue

**Files:**
- Modify: `frontend/src/stores/auth.ts`
- Modify: `frontend/src/router/index.ts`
- Modify: `frontend/src/components/Register.vue`

- [ ] **Step 1: Add verifyEmail action and update register in auth.ts**

In `frontend/src/stores/auth.ts`, update `register` (no longer sets user state) and add `verifyEmail`:

```typescript
const register = async (username: string, email: string, password: string) => {
  try {
    const response = await api.post('/register', { username, email, password })
    // No cookie is issued — user must verify email before logging in.
    return response.data
  } catch (err) {
    if (axios.isAxiosError(err)) {
      throw new Error(err.response?.data?.error || 'Registration failed')
    }
    throw err
  }
}

const verifyEmail = async (token: string): Promise<string> => {
  try {
    const response = await api.post('/auth/verify-email', { token })
    // Backend issues the auth cookie; set local state
    user.value = response.data.username
    return response.data.username as string
  } catch (err) {
    if (axios.isAxiosError(err)) {
      throw new Error(err.response?.data?.error || 'Verification failed')
    }
    throw err
  }
}
```

Export `verifyEmail` in the return object:

```typescript
return {
  user,
  user_avatar,
  name,
  isAuthenticated,
  login,
  register,
  verifyEmail,
  oauthLogin,
  logout,
  initAuth,
}
```

- [ ] **Step 2: Update Register.vue to redirect to the pending page**

In `frontend/src/components/Register.vue`, update `handleSubmit`:

```typescript
const handleSubmit = async (e: Event) => {
  e.preventDefault()
  loading.value = true
  error.value = null

  try {
    await authStore.register(username.value, email.value, password.value)
    router.push({
      name: 'VerifyEmailPending',
      state: { email: email.value },
    })
  } catch (err) {
    error.value = t('errors.generic')
  } finally {
    loading.value = false
  }
}
```

- [ ] **Step 3: Add routes to router/index.ts**

In `frontend/src/router/index.ts`, add these routes (before the catch-all `/:pathMatch` route):

```typescript
{
  path: '/register',
  name: 'Register',
  component: () => import('@/components/Register.vue'),
  meta: { public: true }
},
{
  path: '/verify-email/pending',
  name: 'VerifyEmailPending',
  component: () => import('@/components/VerifyEmailPendingPage.vue'),
  meta: { public: true }
},
{
  path: '/verify-email',
  name: 'VerifyEmailConfirm',
  component: () => import('@/components/VerifyEmailConfirmPage.vue'),
  meta: { public: true }
},
```

- [ ] **Step 4: Verify it builds**

```bash
cd frontend && npm run build
```
Expected: no errors.

- [ ] **Step 5: Run frontend tests**

```bash
cd frontend && npm run test:unit
```
Expected: all existing tests pass (the register test in `Register.spec.ts` may need updating — see note below).

> **Note:** If there is a test asserting that `register` redirects to `/my-calendars`, update it to assert a redirect to `/verify-email/pending` instead.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/stores/auth.ts frontend/src/components/Register.vue frontend/src/router/index.ts
git commit -m "feat: wire up email verification in frontend (pending page, confirm page, store, router)"
```

---

## Task 16: OpenAPI Annotations

**Files:**
- Modify: `server/src/openapi.rs`

The `#[utoipa::path]` macros are already on the new handlers (added in Tasks 6 and 9). We just need to register the new path structs in the `ApiDoc` so they appear in the Swagger UI.

- [ ] **Step 1: Update openapi.rs**

In `server/src/openapi.rs`, find the `#[derive(OpenApi)]` `#[openapi(...)]` block and add to the `paths(...)` list:

```rust
controllers::email_verification::verify_email,
controllers::email_verification::resend_verification,
```

And add to `components(schemas(...))`:

```rust
controllers::email_verification::VerifyEmailRequest,
controllers::email_verification::ResendVerificationRequest,
controllers::auth::MessageResponse,
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check --package server
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add server/src/openapi.rs
git commit -m "docs(openapi): register verify-email and resend-verification in ApiDoc"
```

---

## Final Verification Checklist

Run these before declaring the feature complete:

- [ ] `cargo test --package server` — all backend tests pass
- [ ] `cd frontend && npm run test:unit` — all frontend tests pass
- [ ] `cd frontend && npm run build` — frontend build succeeds
- [ ] Manual smoke test:
  - Register a new user — no cookie is issued; you land on the pending page
  - Try to log in with the unverified account — expect 403 "Email not verified"
  - Click resend — confirm a new token row appears in the DB (or check WARN log in dev)
  - Click the verification link — expect the auth cookie to be set and redirect to `/my-calendars`
  - Log out and log in again with the now-verified account — expect success
  - Register via Google OAuth — expect immediate login with no verification step
- [ ] Update `~/.claude/plans/anime-calendar-next-features.md` — mark "Email verification on register" as Done

---

`★ Insight ─────────────────────────────────────`
**Why `consume_and_verify` is a single transaction:** If we deleted the token first and then the user-update failed, the user would have no token to retry with and no way to self-serve a re-verify — they'd be permanently locked out unless we supported resend. Doing both in one atomic transaction means either both succeed or the token is still there for the next attempt.

**Why OAuth users are auto-verified:** Google has already asserted that the user controls that email address by issuing them an ID token. Requiring them to also click a separate link would be redundant and confusing.

**Why the verify endpoint issues the auth cookie:** Requiring the user to navigate to the login page after clicking the verify link creates unnecessary friction. Since we've just proven they own the email (they clicked the link), issuing the cookie at that point is the same trust level as a successful login.
`─────────────────────────────────────────────────`
