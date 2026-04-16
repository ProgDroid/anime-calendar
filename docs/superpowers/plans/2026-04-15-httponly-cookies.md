# httpOnly Cookie JWT Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace JWT storage in `localStorage` with an httpOnly, SameSite=Strict cookie so the token is never readable by JavaScript.

**Architecture:** Same-domain nginx reverse proxy routes `/api/*` to the backend, making cookies same-origin. The frontend's axios instance drops its `Authorization: Bearer` interceptor and adds `withCredentials: true`. Backend middleware switches from reading the `Authorization` header to reading the `auth_token` cookie. A new `POST /auth/logout` endpoint clears the cookie server-side.

**Tech Stack:** actix-web 4 (`actix_web::cookie::{Cookie, SameSite}`, `actix_web::cookie::time::Duration`), Vite `server.proxy`, Vue 3 Pinia

---

## File Map

| File | Change |
|------|--------|
| `server/src/config/server.rs` | Add `cookie_secure: bool` + `CookieSettings` newtype |
| `config.toml.dist` | Add `cookie_secure = false` |
| `config.docker.toml.dist` | Add `cookie_secure = true` |
| `server/src/server.rs` | Inject `CookieSettings`; add `.supports_credentials()`; register `auth::logout` |
| `server/src/middleware/auth.rs` | Read `auth_token` cookie instead of `Authorization` header; update 5 tests |
| `server/src/controllers/auth.rs` | Rename `LoginResponse` → `AuthResponse`; set cookie in login/register; add `logout` handler; update integration tests |
| `server/src/controllers/oauth.rs` | Remove `token` from `GoogleOAuthResponse`; set cookie |
| `frontend/vite.config.ts` | Add `server.proxy` for `/api/` |
| `frontend/src/config/api.ts` | `baseURL = '/api'`; `withCredentials = true`; remove Bearer interceptor; fix `getApiUrl` |
| `frontend/src/stores/auth.ts` | Full rewrite: no token ref, `initAuth()` → `GET /user`, cookie-based logout |
| `frontend/src/router/index.ts` | Call `authStore.initAuth()` at top of `beforeEach` |
| `frontend/nginx.conf` | Add `/api/` proxy location block |
| `frontend/src/__tests__/LoginPage.spec.ts` | Update mock responses (remove `token`) |
| `frontend/src/__tests__/routerGuard.spec.ts` | Add `initAuth: vi.fn()` to auth store mock |

---

## Task 1: Config — cookie_secure flag and CookieSettings newtype

**Files:**
- Modify: `server/src/config/server.rs`
- Modify: `config.toml.dist`
- Modify: `config.docker.toml.dist`

- [ ] **Step 1: Add `cookie_secure` field and `CookieSettings` newtype to server config**

In `server/src/config/server.rs`, add `cookie_secure` to the `Server` struct and add a `CookieSettings` newtype after the `JwtSecret` newtype:

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
}
```

Also update the `Default` impl to include `cookie_secure: false`:

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
        }
    }
}
```

Add `CookieSettings` at the end of the file, after the `JwtSecret` block:

```rust
/// Controls whether the `auth_token` cookie is sent with `Secure` attribute.
/// Set `false` in local development (HTTP); `true` in production (HTTPS).
#[derive(Clone)]
pub struct CookieSettings {
    pub secure: bool,
}
```

- [ ] **Step 2: Add cookie_secure to config dist files**

In `config.toml.dist`, add after the `compress` line:

```toml
cookie_secure = false
```

In `config.docker.toml.dist`, add after the `compress` line:

```toml
cookie_secure = true
```

- [ ] **Step 3: Compile check**

```bash
cd G:/rustDev/anime-calendar
AWS_LC_SYS_PREBUILT_NASM=1 cargo check -p server 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add server/src/config/server.rs config.toml.dist config.docker.toml.dist
git commit -m "feat: add cookie_secure config flag and CookieSettings newtype"
```

---

## Task 2: Backend — CORS allow credentials

**Files:**
- Modify: `server/src/server.rs`

- [ ] **Step 1: Add `CookieSettings` injection and `.supports_credentials()` to CORS**

In `server/src/server.rs`, add `CookieSettings` to the imports and extract it from config:

```rust
use crate::config::server::{CookieSettings, JwtSecret, Server as ServerConfig};
```

In the `start` function body, after `let jwt_secret = JwtSecret::new(config.jwt_secret);`, add:

```rust
let cookie_settings = CookieSettings { secure: config.cookie_secure };
```

In the CORS builder, add `.supports_credentials()` to the explicit-origins branch only (not the allow-any-origin fallback, which is incompatible with credentials):

```rust
let cors = if allowed_origins.is_empty() {
    Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
} else {
    let mut cors = Cors::default();
    for origin in &allowed_origins {
        cors = cors.allowed_origin(origin);
    }
    cors.allow_any_method()
        .allow_any_header()
        .supports_credentials()
};
```

Add `CookieSettings` to `app_data` in the `App::new()` chain, after `jwt_secret`:

```rust
.app_data(web::Data::new(jwt_secret.clone()))
.app_data(web::Data::new(cookie_settings.clone()))
```

- [ ] **Step 2: Compile check**

```bash
cd G:/rustDev/anime-calendar
AWS_LC_SYS_PREBUILT_NASM=1 cargo check -p server 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add server/src/server.rs
git commit -m "feat: inject CookieSettings; enable CORS supports_credentials for explicit origins"
```

---

## Task 3: Backend — middleware reads auth_token cookie

**Files:**
- Modify: `server/src/middleware/auth.rs`

- [ ] **Step 1: Update the `from_request` implementation to read a cookie**

Replace the `from_request` body inside `impl FromRequest for Claims` in `server/src/middleware/auth.rs`:

```rust
fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
    let req = req.clone();
    let fut = async move {
        let token = req
            .cookie("auth_token")
            .ok_or(Error::Unauthorised)?
            .value()
            .to_owned();

        let Some(jwt_secret) = req.app_data::<actix_web::web::Data<JwtSecret>>() else {
            return Err(Error::Unauthorised);
        };

        let secret = jwt_secret.expose_secret().as_bytes();
        let decoding_key = DecodingKey::from_secret(secret);
        let validation = Validation::default();

        let token_data = decode::<Self>(&token, &decoding_key, &validation)
            .map_err(|_| Error::Unauthorised)?;

        Ok(token_data.claims)
    };

    Box::pin(fut)
}
```

- [ ] **Step 2: Update all 5 middleware tests to send a Cookie header instead of Authorization**

In the `#[cfg(test)] mod tests` block, update every `TestRequest` that currently has `.insert_header(("Authorization", format!("Bearer {token}")))` to use:

```rust
.insert_header(("Cookie", format!("auth_token={token}")))
```

The five tests to update:
- `valid_jwt_allows_access`
- `expired_jwt_returns_401` (the `Token ` prefix test — change header name to Cookie, value to `auth_token=<token>`)
- `non_bearer_scheme_returns_401` → rename to `missing_cookie_returns_401` and remove the `Cookie` header entirely (test that a request with no cookie returns 401)
- `wrong_secret_returns_401` → send a `Cookie: auth_token=<bad-token>` header
- `missing_authorization_header_returns_401` → rename to `no_cookie_returns_401`

After the edit the test block should look like:

```rust
#[actix_web::test]
async fn valid_jwt_allows_access() {
    let app = test::init_service(
        App::new().app_data(secret_data()).service(guarded),
    )
    .await;
    let token = generate_token(&99, SECRET).unwrap();
    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Cookie", format!("auth_token={token}")))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
}

#[actix_web::test]
async fn expired_jwt_returns_401() {
    let app = test::init_service(
        App::new().app_data(secret_data()).service(guarded),
    )
    .await;
    let claims = Claims { sub: "1".to_owned(), exp: 0 };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap();
    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Cookie", format!("auth_token={token}")))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn no_cookie_returns_401() {
    let app = test::init_service(
        App::new().app_data(secret_data()).service(guarded),
    )
    .await;
    let req = test::TestRequest::get().uri("/protected").to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn wrong_cookie_name_returns_401() {
    let app = test::init_service(
        App::new().app_data(secret_data()).service(guarded),
    )
    .await;
    let token = generate_token(&1, SECRET).unwrap();
    // "session" instead of "auth_token"
    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Cookie", format!("session={token}")))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn wrong_secret_returns_401() {
    let app = test::init_service(
        App::new().app_data(secret_data()).service(guarded),
    )
    .await;
    let token = generate_token(&1, "a-completely-different-secret!!").unwrap();
    let req = test::TestRequest::get()
        .uri("/protected")
        .insert_header(("Cookie", format!("auth_token={token}")))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 3: Run middleware tests**

```bash
cd G:/rustDev/anime-calendar
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server -- middleware --test-threads=4 2>&1 | tail -15
```

Expected: `test result: ok. 5 passed`

- [ ] **Step 4: Commit**

```bash
git add server/src/middleware/auth.rs
git commit -m "feat: middleware reads auth_token cookie instead of Authorization header"
```

---

## Task 4: Backend — login/register set cookie, AuthResponse replaces LoginResponse

**Files:**
- Modify: `server/src/controllers/auth.rs`

- [ ] **Step 1: Add cookie imports and rename LoginResponse to AuthResponse**

At the top of `server/src/controllers/auth.rs`, add:

```rust
use actix_web::cookie::{time::Duration, Cookie, SameSite};
use crate::config::server::CookieSettings;
```

Replace the `LoginResponse` struct and its `#[derive]` with `AuthResponse`:

```rust
#[derive(Serialize, utoipa::ToSchema)]
pub struct AuthResponse {
    pub username: String,
}
```

- [ ] **Step 2: Add a cookie-building helper function**

After `AuthResponse`, add:

```rust
fn build_auth_cookie(token: String, cookie_settings: &CookieSettings) -> Cookie<'static> {
    Cookie::build("auth_token", token)
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/")
        .max_age(Duration::hours(24))
        .secure(cookie_settings.secure)
        .finish()
}
```

- [ ] **Step 3: Update the login handler**

Replace the `login` handler signature and body:

```rust
#[utoipa::path(
    post,
    path = "/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
    )
)]
#[post("/login")]
pub async fn login(
    db: web::Data<UserMapper>,
    credentials: web::Json<LoginRequest>,
    jwt_secret: web::Data<JwtSecret>,
    cookie_settings: web::Data<CookieSettings>,
) -> HttpResponse {
    let Ok(user) = db.get_user_by_email(&credentials.email).await else {
        return Error::Unauthorised.error_response();
    };

    match user.password_hash {
        Some(hash) => {
            if validate_password(credentials.password.expose_secret(), &hash) {
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
}
```

- [ ] **Step 4: Update the register handler**

Replace the `register` handler signature and final response:

```rust
#[utoipa::path(
    post,
    path = "/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration successful", body = AuthResponse),
        (status = 400, description = "Invalid request or weak password", body = ErrorResponse),
    )
)]
#[post("/register")]
pub async fn register(
    db: web::Data<UserMapper>,
    user_data: web::Json<RegisterRequest>,
    jwt_secret: web::Data<JwtSecret>,
    cookie_settings: web::Data<CookieSettings>,
) -> HttpResponse {
    // ... all validation unchanged ...

    let token = match generate_token(&user.id, jwt_secret.expose_secret()) {
        Ok(token) => token,
        Err(e) => return e.error_response(),
    };
    let cookie = build_auth_cookie(token, &cookie_settings);
    HttpResponse::Ok()
        .cookie(cookie)
        .json(AuthResponse { username: user.username })
}
```

(Keep all validation logic unchanged — only the final response block changes.)

- [ ] **Step 5: Update get_current_user**

Replace the `get_current_user` handler body:

```rust
#[get("/user")]
pub async fn get_current_user(db: web::Data<UserMapper>, claims: Claims) -> HttpResponse {
    let user = match db.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => return e.error_response(),
    };
    HttpResponse::Ok().json(AuthResponse { username: user.username })
}
```

- [ ] **Step 6: Update integration tests for login and register**

In `mod integration_tests`, the `jwt_data()` helper stays. Add `cookie_data()`:

```rust
fn cookie_data() -> web::Data<CookieSettings> {
    web::Data::new(CookieSettings { secure: false })
}
```

Update `login_valid_credentials_returns_token_and_username` — rename it and change assertions:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn login_valid_credentials_sets_cookie_and_returns_username(pool: PgPool) {
    seed_user(&pool, "alice", "alice@test.com").await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool)))
            .app_data(jwt_data())
            .app_data(cookie_data())
            .service(login)
            .service(register)
            .service(get_current_user),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/login")
        .set_json(serde_json::json!({ "email": "alice@test.com", "password": STRONG_PW }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Cookie must be set with HttpOnly
    let set_cookie = resp.headers().get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(set_cookie.contains("auth_token="), "auth_token cookie missing");
    assert!(set_cookie.contains("HttpOnly"), "HttpOnly flag missing");

    // Body has username, no token
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["username"], "alice");
    assert!(body.get("token").is_none(), "token must not appear in body");
}
```

Update `register_new_user_returns_token_and_username` similarly:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn register_new_user_sets_cookie_and_returns_username(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool)))
            .app_data(jwt_data())
            .app_data(cookie_data())
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

    let set_cookie = resp.headers().get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(set_cookie.contains("auth_token="), "auth_token cookie missing");
    assert!(set_cookie.contains("HttpOnly"), "HttpOnly flag missing");

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["username"], "carol");
    assert!(body.get("token").is_none(), "token must not appear in body");
}
```

Add `cookie_data()` to all remaining test apps that use `login` or `register`:
- `login_wrong_password_returns_401` → add `.app_data(cookie_data())`
- `login_unknown_email_returns_401_not_404` → add `.app_data(cookie_data())`
- `login_oauth_user_without_password_returns_401` → add `.app_data(cookie_data())`
- `register_duplicate_email_returns_400` → add `.app_data(cookie_data())`
- `register_weak_password_returns_400` → add `.app_data(cookie_data())`
- `register_invalid_email_returns_400` → add `.app_data(cookie_data())`
- `register_username_too_long_returns_400` → add `.app_data(cookie_data())`

The `get_current_user` tests send a `Cookie: auth_token=<token>` header (since middleware now reads the cookie). Update:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn get_current_user_with_valid_jwt_returns_username(pool: PgPool) {
    let user_id = seed_user(&pool, "grace", "grace@test.com").await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool)))
            .app_data(jwt_data())
            .app_data(cookie_data())
            .service(get_current_user),
    )
    .await;
    let token = generate_token(&user_id, SECRET).unwrap();
    let req = test::TestRequest::get()
        .uri("/user")
        .insert_header(("Cookie", format!("auth_token={token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["username"], "grace");
    assert!(body.get("token").is_none());
}

#[sqlx::test(migrations = "../migrations")]
async fn get_current_user_without_token_returns_401(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool)))
            .app_data(jwt_data())
            .app_data(cookie_data())
            .service(get_current_user),
    )
    .await;
    let req = test::TestRequest::get().uri("/user").to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 7: Run auth controller tests**

```bash
cd G:/rustDev/anime-calendar
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server -- controllers::auth --test-threads=4 2>&1 | tail -20
```

Expected: `test result: ok. 11 passed` (same count, updated logic).

- [ ] **Step 8: Commit**

```bash
git add server/src/controllers/auth.rs
git commit -m "feat: login/register set httpOnly cookie; AuthResponse replaces LoginResponse"
```

---

## Task 5: Backend — OAuth sets cookie, removes token from response

**Files:**
- Modify: `server/src/controllers/oauth.rs`

- [ ] **Step 1: Update GoogleOAuthResponse and set cookie**

Replace the `google_oauth` handler in `server/src/controllers/oauth.rs`:

```rust
use crate::config::server::{CookieSettings, JwtSecret};
use crate::controllers::auth::build_auth_cookie;
use crate::mappers::google_oauth::GoogleOauth;
use crate::mappers::user::UserMapper;
use crate::services::auth::generate_token;
use actix_web::{post, web, HttpResponse, ResponseError};
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GoogleOAuthRequest {
    pub token: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GoogleOAuthResponse {
    pub username: String,
    pub email: String,
    pub avatar: String,
}

#[utoipa::path(
    post,
    path = "/auth/google",
    tag = "auth",
    request_body = GoogleOAuthRequest,
    responses(
        (status = 200, description = "OAuth login successful", body = GoogleOAuthResponse),
        (status = 401, description = "Invalid Google ID token", body = crate::controllers::auth::ErrorResponse),
    )
)]
#[post("/auth/google")]
pub async fn google_oauth(
    user_mapper: web::Data<UserMapper>,
    google_oauth: web::Data<GoogleOauth>,
    google_request: web::Json<GoogleOAuthRequest>,
    jwt_secret: web::Data<JwtSecret>,
    cookie_settings: web::Data<CookieSettings>,
) -> HttpResponse {
    match google_oauth.validate_id_token(&google_request.token).await {
        Ok(google_user) => {
            let user = match user_mapper.get_user_by_email(&google_user.email).await {
                Ok(user) => user,
                Err(_) => {
                    match user_mapper
                        .create_user(&google_user.id, &google_user.email, None)
                        .await
                    {
                        Ok(user) => user,
                        Err(e) => return e.error_response(),
                    }
                }
            };

            let token = match generate_token(&user.id, jwt_secret.expose_secret()) {
                Ok(token) => token,
                Err(e) => return e.error_response(),
            };

            let cookie = build_auth_cookie(token, &cookie_settings);
            let response = GoogleOAuthResponse {
                username: google_user.full_name,
                email: user.email,
                avatar: google_user.avatar_url,
            };

            HttpResponse::Ok().cookie(cookie).json(response)
        }
        Err(e) => {
            error!("{e}");
            e.error_response()
        }
    }
}
```

Note: `build_auth_cookie` must be `pub` in `auth.rs`. Change it from `fn` to `pub fn`.

- [ ] **Step 2: Compile check**

```bash
cd G:/rustDev/anime-calendar
AWS_LC_SYS_PREBUILT_NASM=1 cargo check -p server 2>&1 | tail -5
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add server/src/controllers/oauth.rs server/src/controllers/auth.rs
git commit -m "feat: OAuth handler sets httpOnly cookie; remove token from GoogleOAuthResponse"
```

---

## Task 6: Backend — add POST /auth/logout endpoint

**Files:**
- Modify: `server/src/controllers/auth.rs`
- Modify: `server/src/server.rs`

- [ ] **Step 1: Add the logout handler**

In `server/src/controllers/auth.rs`, add the logout handler after `verify_token_endpoint`:

```rust
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "Logged out successfully"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[post("/auth/logout")]
pub async fn logout(
    _claims: Claims,
    cookie_settings: web::Data<CookieSettings>,
) -> HttpResponse {
    let removal_cookie = Cookie::build("auth_token", "")
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/")
        .max_age(Duration::ZERO)
        .secure(cookie_settings.secure)
        .finish();
    HttpResponse::Ok().cookie(removal_cookie).json(serde_json::json!({}))
}
```

- [ ] **Step 2: Register the logout route in server.rs**

In `server/src/server.rs`, add `.service(auth::logout)` after `.service(auth::verify_token_endpoint)`:

```rust
.service(auth::verify_token_endpoint)
.service(auth::logout)
```

- [ ] **Step 3: Write a failing test for logout**

In the `integration_tests` module of `auth.rs`, add:

```rust
#[sqlx::test(migrations = "../migrations")]
async fn logout_clears_auth_cookie(pool: PgPool) {
    let user_id = seed_user(&pool, "hank", "hank@test.com").await;
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool)))
            .app_data(jwt_data())
            .app_data(cookie_data())
            .service(get_current_user)
            .service(logout),
    )
    .await;

    let token = generate_token(&user_id, SECRET).unwrap();
    let req = test::TestRequest::post()
        .uri("/auth/logout")
        .insert_header(("Cookie", format!("auth_token={token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // The Set-Cookie header should clear auth_token (Max-Age=0)
    let set_cookie = resp.headers().get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(set_cookie.contains("auth_token="), "cookie name missing");
    assert!(set_cookie.contains("Max-Age=0"), "Max-Age=0 missing — cookie not cleared");
}

#[sqlx::test(migrations = "../migrations")]
async fn logout_without_cookie_returns_401(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(UserMapper::from_pool(pool)))
            .app_data(jwt_data())
            .app_data(cookie_data())
            .service(logout),
    )
    .await;
    let req = test::TestRequest::post().uri("/auth/logout").to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 4: Run auth controller tests**

```bash
cd G:/rustDev/anime-calendar
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server -- controllers::auth --test-threads=4 2>&1 | tail -20
```

Expected: `test result: ok. 13 passed` (11 existing + 2 new logout tests).

- [ ] **Step 5: Run the full backend test suite**

```bash
cd G:/rustDev/anime-calendar
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server -- --test-threads=4 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add server/src/controllers/auth.rs server/src/server.rs
git commit -m "feat: add POST /auth/logout endpoint; clears auth_token cookie with Max-Age=0"
```

---

## Task 7: Frontend — Vite proxy, axios withCredentials, baseURL, getApiUrl fix

**Files:**
- Modify: `frontend/vite.config.ts`
- Modify: `frontend/src/config/api.ts`
- Modify: `frontend/src/__tests__/MyCalendarsPage.spec.ts`

- [ ] **Step 1: Add Vite dev proxy**

Replace the contents of `frontend/vite.config.ts`:

```typescript
import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import tailwindcss from "@tailwindcss/vite"
import vue from '@vitejs/plugin-vue'
import vueDevTools from 'vite-plugin-vue-devtools'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    tailwindcss(),
    vue(),
    vueDevTools(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    },
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        rewrite: (path) => path.replace(/^\/api/, ''),
      },
    },
  },
})
```

- [ ] **Step 2: Update api.ts**

Replace the entire contents of `frontend/src/config/api.ts`:

```typescript
import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  withCredentials: true,
})

/**
 * Returns an absolute URL for a backend path.
 * Used for subscription/export links that are pasted into external calendar clients.
 * Must be absolute so Google Calendar / Apple Calendar can fetch them.
 */
export const getApiUrl = (path: string): string => {
  return `${window.location.origin}/api${path}`
}

export default api
```

- [ ] **Step 3: Update MyCalendarsPage.spec.ts mock for getApiUrl**

In `frontend/src/__tests__/MyCalendarsPage.spec.ts`, find the mock line:

```typescript
getApiUrl: (path: string) => `http://localhost:8080${path}`
```

Replace it with:

```typescript
getApiUrl: (path: string) => `http://localhost/api${path}`
```

- [ ] **Step 4: TypeScript build check**

```bash
cd G:/rustDev/anime-calendar/frontend
npm run build 2>&1 | tail -10
```

Expected: build succeeds with no TypeScript errors.

- [ ] **Step 5: Commit**

```bash
git add frontend/vite.config.ts frontend/src/config/api.ts frontend/src/__tests__/MyCalendarsPage.spec.ts
git commit -m "feat: Vite proxy /api/ → backend; axios withCredentials; fix getApiUrl for external clients"
```

---

## Task 8: Frontend — auth store rewrite

**Files:**
- Modify: `frontend/src/stores/auth.ts`
- Modify: `frontend/src/__tests__/LoginPage.spec.ts`

- [ ] **Step 1: Write the new auth store**

Replace the entire contents of `frontend/src/stores/auth.ts`:

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import axios from 'axios'
import api from '../config/api'
import { invalidateSettingsCache } from '@/services/userSettingsService'

export const useAuthStore = defineStore('auth', () => {
  const user = ref('')
  const name = ref('')
  const user_avatar = ref('')
  const router = useRouter()

  let initPromise: Promise<void> | null = null
  let initialized = false

  const isAuthenticated = () => user.value !== ''

  /**
   * Rehydrate auth state from the server on page load.
   * Calls GET /user; on success populates user.value, on failure clears it.
   * Idempotent — subsequent calls return immediately.
   * Concurrent calls return the same in-flight Promise.
   */
  const initAuth = async (): Promise<void> => {
    if (initialized) return
    if (initPromise) return initPromise

    initPromise = (async () => {
      try {
        const response = await api.get('/user')
        user.value = response.data.username ?? ''
      } catch {
        user.value = ''
      } finally {
        initialized = true
        initPromise = null
      }
    })()

    return initPromise
  }

  const login = async (email: string, password: string) => {
    try {
      const response = await api.post('/login', { email, password })
      user.value = response.data.username
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Login failed')
      }
      throw err
    }
  }

  const register = async (username: string, email: string, password: string) => {
    try {
      const response = await api.post('/register', { username, email, password })
      user.value = response.data.username
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || 'Registration failed')
      }
      throw err
    }
  }

  const oauthLogin = async (provider: 'google', token_string: string) => {
    try {
      const response = await api.post(`/auth/${provider}`, { token: token_string })
      const { username: userData, avatar: avatarUrl } = response.data
      user.value = userData
      name.value = userData
      user_avatar.value = avatarUrl
      // Non-sensitive display data only — no auth token in localStorage
      localStorage.setItem('name', userData)
      localStorage.setItem('avatar', avatarUrl)
      return response.data
    } catch (err) {
      if (axios.isAxiosError(err)) {
        throw new Error(err.response?.data?.error || `${provider} login failed`)
      }
      throw err
    }
  }

  const logout = async () => {
    try {
      await api.post('/auth/logout')
    } catch {
      // Clear local state regardless of server response
    }
    user.value = ''
    name.value = ''
    user_avatar.value = ''
    initialized = false
    localStorage.removeItem('name')
    localStorage.removeItem('avatar')
    invalidateSettingsCache()
    router.push('/login')
  }

  return {
    user,
    user_avatar,
    name,
    isAuthenticated,
    login,
    register,
    oauthLogin,
    logout,
    initAuth,
  }
})
```

- [ ] **Step 2: Update LoginPage.spec.ts mock responses**

The LoginPage tests mock `api.post` responses. Update every occurrence of `{ data: { token: 'tok', username: 'user' } }` to `{ data: { username: 'user' } }` (two places: the login call test and the register call test). Also update `{ data: { token: 't', username: 'u' } }` in the loading-disable test to `{ data: { username: 'u' } }`.

Concretely, find and replace in `frontend/src/__tests__/LoginPage.spec.ts`:

Old:
```typescript
vi.mocked(api.post).mockResolvedValue({ data: { token: 'tok', username: 'user' } })
```
New:
```typescript
vi.mocked(api.post).mockResolvedValue({ data: { username: 'user' } })
```

Old (in the loading-disable test):
```typescript
resolve!({ data: { token: 't', username: 'u' } })
```
New:
```typescript
resolve!({ data: { username: 'u' } })
```

Also remove the `localStorage` stub from `beforeEach` — after the migration the auth store never calls `localStorage.setItem('authToken', ...)`, so the stub is no longer needed. The stub itself won't cause test failures if left in, but removing it keeps the test clean:

```typescript
beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
})
```

- [ ] **Step 3: Run frontend tests**

```bash
cd G:/rustDev/anime-calendar/frontend
npm run test:unit 2>&1 | tail -20
```

Expected: all tests pass. If `GoogleLoginButton.spec.ts` or `UserDetailsPage.spec.ts` fail because they reference `token` in store state, update those tests' store pre-seeding to use `user` directly (e.g., `pinia.state.value['auth'].user = 'testuser'`) instead of setting `token`.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/stores/auth.ts frontend/src/__tests__/LoginPage.spec.ts
git commit -m "feat: auth store — cookie-based auth, initAuth() calls /user, logout calls /auth/logout"
```

---

## Task 9: Frontend — router guard calls initAuth on first navigation

**Files:**
- Modify: `frontend/src/router/index.ts`
- Modify: `frontend/src/__tests__/routerGuard.spec.ts`

- [ ] **Step 1: Update the beforeEach guard**

Replace the `router.beforeEach` block in `frontend/src/router/index.ts`:

```typescript
router.beforeEach(async (to, from, next) => {
  const authStore = useAuthStore()
  const userSettingsStore = useUserSettingsStore()

  // Rehydrate auth state from the server cookie on first navigation.
  // initAuth() is idempotent — subsequent navigations return immediately.
  await authStore.initAuth()

  if (!to.meta.public) {
    const settings = await userSettingsStore.fetchSettings()
    if (settings) {
      applySettings(settings)
    }
  }

  if (to.path === '/login' && authStore.isAuthenticated()) {
    next('/my-calendars')
  } else if (to.meta.requiresAuth && !authStore.isAuthenticated()) {
    next('/login')
  } else {
    next()
  }
})
```

- [ ] **Step 2: Update routerGuard.spec.ts**

The test file copies the guard logic and mocks `useAuthStore`. Add `initAuth: vi.fn().mockResolvedValue(undefined)` to every mock return in the file. For example, wherever the mock looks like:

```typescript
vi.mocked(useAuthStore).mockReturnValue({
  isAuthenticated: mockIsAuthenticated,
  // ...other fields
} as any)
```

Update to:

```typescript
vi.mocked(useAuthStore).mockReturnValue({
  isAuthenticated: mockIsAuthenticated,
  initAuth: vi.fn().mockResolvedValue(undefined),
  // ...other fields
} as any)
```

Also update the copied guard logic in the test file to include the `await authStore.initAuth()` call at the top of the guard body, matching the new real guard.

- [ ] **Step 3: Run frontend tests**

```bash
cd G:/rustDev/anime-calendar/frontend
npm run test:unit 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/router/index.ts frontend/src/__tests__/routerGuard.spec.ts
git commit -m "feat: router beforeEach calls initAuth() to rehydrate auth from cookie on first load"
```

---

## Task 10: Frontend — nginx proxy block

**Files:**
- Modify: `frontend/nginx.conf`

- [ ] **Step 1: Add /api/ proxy location**

Replace the contents of `frontend/nginx.conf`:

```nginx
server {
    listen 80;
    server_name localhost;
    root /usr/share/nginx/html;
    index index.html;

    # Proxy API requests to the backend — must come before the SPA catch-all
    location /api/ {
        proxy_pass http://server:8080/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
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

Note: `proxy_pass http://server:8080/;` — the trailing slash strips the `/api/` prefix before forwarding to the Actix backend, so `/api/login` becomes `/login`.

- [ ] **Step 2: Commit**

```bash
git add frontend/nginx.conf
git commit -m "feat: nginx proxies /api/ to backend; strips /api prefix"
```

---

## Task 11: Full verification

- [ ] **Step 1: Run full backend test suite**

```bash
cd G:/rustDev/anime-calendar
AWS_LC_SYS_PREBUILT_NASM=1 cargo test -p server -- --test-threads=4 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 2: Run full frontend test suite**

```bash
cd G:/rustDev/anime-calendar/frontend
npm run test:unit 2>&1 | tail -10
```

Expected: all tests pass (117+ passing).

- [ ] **Step 3: Frontend build**

```bash
cd G:/rustDev/anime-calendar/frontend
npm run build 2>&1 | tail -10
```

Expected: build succeeds.

- [ ] **Step 4: Frontend lint**

```bash
cd G:/rustDev/anime-calendar/frontend
npm run lint 2>&1 | tail -10
```

Expected: no lint errors.

- [ ] **Step 5: Update the audit plan and implementation status memory**

Mark the `httpOnly cookies` outstanding item as resolved in `~/.claude/plans/deep-wondering-liskov.md` (Task 4 under CRITICAL) and update `memory/project_implementation_status.md`.

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "chore: mark httpOnly cookie migration complete"
```

---

## Notes for the implementer

- **Atomic deployment:** backend and frontend must be deployed together. During the cutover window (between updating backend middleware and updating frontend), auth will be broken for any users with active localStorage tokens. For a personal project this is acceptable; schedule during low-traffic time.
- **`build_auth_cookie` must be `pub`:** it's called from `oauth.rs`, so the visibility must be `pub fn build_auth_cookie(...)`.
- **`VITE_API_BASE_URL` env var:** no longer needed by the frontend. Remove from `.env.example`, `docker-compose.yml` build args, and `frontend/Dockerfile` ARG declarations.
- **OpenAPI security scheme:** the `security(("bearer_auth" = []))` annotation in `get_current_user` and `logout` is now conceptually a cookie scheme. This is cosmetic — update the OpenAPI `ApiDoc` security scheme to `cookieAuth` if OpenAPI accuracy matters, but it does not affect runtime behavior.
- **`/auth/verify` endpoint:** still exists on the backend and accepts a token in the body. It is no longer called by the frontend (frontend has no token to verify). It can be left as-is or removed in a follow-up cleanup.
