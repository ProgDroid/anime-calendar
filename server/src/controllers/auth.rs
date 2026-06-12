use crate::config::server::{AppBaseUrl, CookieSettings, JwtSecret};
use crate::error::Error;
use crate::mappers::email_verification::EmailVerificationMapper;
use crate::mappers::refresh_token::RefreshTokenMapper;
use crate::mappers::user::UserMapper;
use crate::middleware::auth::Claims;
use crate::services::auth::{
    generate_random_token, generate_token, hash_password, hash_token, validate_password,
    validate_password_strength, verify_token,
};
use crate::services::email::EmailService;
use actix_web::cookie::{Cookie, SameSite, time::Duration};
use actix_web::{HttpResponse, ResponseError, get, post, web};
use log::error;
use rand::RngCore;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

/// Generic error response body
#[derive(Serialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

fn is_valid_email(email: &str) -> bool {
    // RFC 5321 syntax check via the email_address crate.
    // We also require a dot in the domain — local-only hostnames are RFC-valid
    // but not acceptable for internet-facing registration.
    email_address::EmailAddress::is_valid(email)
        && email
            .rsplit_once('@')
            .is_some_and(|(_, domain)| domain.contains('.'))
}

/// Generate 32 random bytes as a lowercase hex string.
#[must_use]
pub fn generate_raw_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{b:02x}"));
        s
    })
}

/// SHA-256 hash of `raw`, returned as a lowercase hex string.
///
/// Delegates to the canonical [`crate::services::auth::hash_token`] so the
/// SHA-256→hex logic lives in exactly one place (L-5).
#[must_use]
pub fn hash_refresh_token(raw: &str) -> String {
    crate::services::auth::hash_token(raw)
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub email: String,
    #[schema(value_type = String, format = Password)]
    pub password: SecretString,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AuthResponse {
    pub username: String,
    pub user_id: i32,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

/// Build an httpOnly auth cookie from a JWT token and cookie settings.
#[must_use]
pub fn build_auth_cookie(token: String, cookie_settings: &CookieSettings) -> Cookie<'static> {
    Cookie::build("auth_token", token)
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/")
        .max_age(Duration::minutes(30))
        .secure(cookie_settings.secure)
        .finish()
}

/// Build an httpOnly refresh token cookie scoped to the refresh endpoint.
#[must_use]
pub fn build_refresh_cookie(token: String, cookie_settings: &CookieSettings) -> Cookie<'static> {
    Cookie::build("refresh_token", token)
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::days(30))
        .secure(cookie_settings.secure)
        .finish()
}

/// Build a zero-max-age refresh cookie to clear it on logout.
#[must_use]
fn clear_refresh_cookie(cookie_settings: &CookieSettings) -> Cookie<'static> {
    Cookie::build("refresh_token", "")
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::ZERO)
        .secure(cookie_settings.secure)
        .finish()
}

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
    refresh_mapper: web::Data<RefreshTokenMapper>,
    credentials: web::Json<LoginRequest>,
    jwt_secret: web::Data<JwtSecret>,
    cookie_settings: web::Data<CookieSettings>,
) -> HttpResponse {
    use crate::metrics::names::{
        AUTH_LOGIN_ATTEMPTS_TOTAL, LABEL_OUTCOME, OUTCOME_FAILED, OUTCOME_OK,
    };
    let record = |outcome: &'static str| {
        metrics::counter!(AUTH_LOGIN_ATTEMPTS_TOTAL, LABEL_OUTCOME => outcome).increment(1);
    };

    let Ok(user) = db.get_user_by_email(&credentials.email).await else {
        record(OUTCOME_FAILED);
        return Error::Unauthorised.error_response();
    };

    if let Some(hash) = user.password_hash {
        if validate_password(credentials.password.expose_secret(), &hash) {
            // Block login if email has not been verified yet.
            if user.email_verified_at.is_none() {
                record(OUTCOME_FAILED);
                return Error::EmailNotVerified.error_response();
            }

            let token = match generate_token(&user.id, jwt_secret.expose_secret()) {
                Ok(token) => token,
                Err(e) => {
                    record(OUTCOME_FAILED);
                    return e.error_response();
                }
            };

            let raw_refresh = generate_raw_token();
            let refresh_hash = hash_refresh_token(&raw_refresh);
            if let Err(e) = refresh_mapper.replace_token(user.id, &refresh_hash).await {
                record(OUTCOME_FAILED);
                return e.error_response();
            }

            let auth_cookie = build_auth_cookie(token, &cookie_settings);
            let refresh_cookie = build_refresh_cookie(raw_refresh, &cookie_settings);
            record(OUTCOME_OK);
            HttpResponse::Ok()
                .cookie(auth_cookie)
                .cookie(refresh_cookie)
                .json(AuthResponse {
                    username: user.username,
                    user_id: user.id,
                })
        } else {
            record(OUTCOME_FAILED);
            Error::Unauthorised.error_response()
        }
    } else {
        record(OUTCOME_FAILED);
        Error::Unauthorised.error_response()
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    #[schema(value_type = String, format = Password)]
    pub password: SecretString,
}

#[utoipa::path(
    post,
    path = "/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration accepted — verification email sent", body = MessageResponse),
        (status = 400, description = "Invalid request or weak password", body = ErrorResponse),
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
    use crate::metrics::names::{
        AUTH_REGISTRATIONS_TOTAL, LABEL_OUTCOME, OUTCOME_FAILED, OUTCOME_OK,
    };
    let record = |outcome: &'static str| {
        metrics::counter!(AUTH_REGISTRATIONS_TOTAL, LABEL_OUTCOME => outcome).increment(1);
    };

    let ok = || {
        HttpResponse::Ok().json(MessageResponse {
            message: "Verification email sent. Please check your inbox.".into(),
        })
    };

    if user_data.username.len() > 50 {
        record(OUTCOME_FAILED);
        return Error::InvalidRequest.error_response();
    }

    if !is_valid_email(&user_data.email) {
        record(OUTCOME_FAILED);
        return Error::InvalidRequest.error_response();
    }

    if user_data.password.expose_secret().len() > 128 {
        record(OUTCOME_FAILED);
        return Error::InvalidRequest.error_response();
    }

    if !validate_password_strength(user_data.password.expose_secret()) {
        record(OUTCOME_FAILED);
        return Error::InvalidPassword.error_response();
    }

    // Email already registered — silently resend verification (if unverified) and
    // return the same 200 shape as a new registration to prevent enumeration.
    if let Ok(existing) = db.get_user_by_email(&user_data.email).await {
        if existing.email_verified_at.is_none() {
            let raw_token = generate_random_token();
            let token_hash = hash_token(&raw_token);
            let verify_url = format!("{}/verify-email?token={raw_token}", app_base_url.as_str());
            if let Err(e) = verification_mapper
                .replace_token(existing.id, &token_hash)
                .await
            {
                error!("{e}");
            } else if let Err(e) = email_service
                .send_verification_email(&existing.email, &verify_url)
                .await
            {
                error!("{e}");
            }
        }
        record(OUTCOME_FAILED);
        return ok();
    }

    let hashed_password = match hash_password(user_data.password.expose_secret()) {
        Ok(h) => h,
        Err(e) => {
            record(OUTCOME_FAILED);
            return e.error_response();
        }
    };

    let user = match db
        .create_user(
            &user_data.username,
            &user_data.email,
            Some(&hashed_password),
        )
        .await
    {
        Ok(u) => u,
        Err(e) => {
            record(OUTCOME_FAILED);
            return e.error_response();
        }
    };

    let raw_token = generate_random_token();
    let token_hash = hash_token(&raw_token);
    let verify_url = format!("{}/verify-email?token={raw_token}", app_base_url.as_str());

    if let Err(e) = verification_mapper
        .replace_token(user.id, &token_hash)
        .await
    {
        error!("{e}");
        // Compensate: delete the newly-created user so the email address is
        // not permanently stuck in an unverifiable state. Best-effort only.
        if let Err(del_err) = db.delete_user(user.id).await {
            error!(
                "Failed to rollback user {} after token error: {del_err}",
                user.id
            );
        }
        record(OUTCOME_FAILED);
        return e.error_response();
    }

    if let Err(e) = email_service
        .send_verification_email(&user.email, &verify_url)
        .await
    {
        error!("{e}");
        record(OUTCOME_FAILED);
        return e.error_response();
    }

    record(OUTCOME_OK);
    ok()
}

#[utoipa::path(
    get,
    path = "/user",
    tag = "auth",
    responses(
        (status = 200, description = "Current authenticated user", body = AuthResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/user")]
pub async fn get_current_user(db: web::Data<UserMapper>, claims: Claims) -> HttpResponse {
    let user = match db.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => return e.error_response(),
    };

    HttpResponse::Ok().json(AuthResponse {
        username: user.username,
        user_id: user.id,
    })
}

#[derive(Deserialize, utoipa::ToSchema)]
struct AuthVerifyRequest {
    token: String,
}

#[utoipa::path(
    post,
    path = "/auth/verify",
    tag = "auth",
    responses(
        (status = 200, description = "Token is valid"),
        (status = 401, description = "Token is invalid or expired", body = ErrorResponse),
    )
)]
#[post("/auth/verify")]
pub async fn verify_token_endpoint(
    token: web::Json<AuthVerifyRequest>,
    jwt_secret: web::Data<JwtSecret>,
) -> HttpResponse {
    match verify_token(&token.token, jwt_secret.expose_secret()) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => Error::Unauthorised.error_response(),
    }
}

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
    claims: Claims,
    refresh_mapper: web::Data<RefreshTokenMapper>,
    cookie_settings: web::Data<CookieSettings>,
) -> HttpResponse {
    let user_id: i32 = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => return Error::Unauthorised.error_response(),
    };

    if let Err(e) = refresh_mapper.invalidate_all_for_user(user_id).await {
        log::error!("Failed to invalidate refresh tokens on logout for user {user_id}: {e}");
    }

    let removal_cookie = Cookie::build("auth_token", "")
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/")
        .max_age(Duration::ZERO)
        .secure(cookie_settings.secure)
        .finish();
    let removal_refresh = clear_refresh_cookie(&cookie_settings);

    HttpResponse::Ok()
        .cookie(removal_cookie)
        .cookie(removal_refresh)
        .json(serde_json::json!({}))
}

/// Unit tests for pure functions (no DB / no HTTP stack).
#[cfg(test)]
mod unit_tests {
    use super::is_valid_email;

    #[test]
    fn validates_email_format() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("a@b.co"));
        assert!(is_valid_email("user.name+tag@sub.domain.org"));
        assert!(!is_valid_email("notanemail"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@nodot"));
        assert!(!is_valid_email("user@@domain.com"));
        assert!(!is_valid_email("user@.com"));
        assert!(!is_valid_email("user@domain."));
        assert!(!is_valid_email(""));
    }
}

/// Integration tests — spin up a real actix-web app backed by an isolated test DB.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        config::server::AppBaseUrl,
        mappers::{
            email_verification::EmailVerificationMapper, refresh_token::RefreshTokenMapper,
            user::UserMapper,
        },
        services::{
            auth::{generate_token, hash_password},
            email::EmailService,
        },
    };
    use actix_web::{App, http::StatusCode, test, web};
    use secrecy::SecretString;
    use serde_json::Value;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";
    const STRONG_PW: &str = "SecurePass12!@";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    fn cookie_data() -> web::Data<CookieSettings> {
        web::Data::new(CookieSettings { secure: false })
    }

    struct SeedUser {
        id: i32,
        username: String,
        email: String,
    }

    /// Seed a verified password-based user with random username/email.
    async fn seed_user(pool: &sqlx::PgPool) -> SeedUser {
        let n: u64 = rand::random();
        let username = format!("authtest_{n}");
        let email = format!("authtest_{n}@test.com");
        let hash = hash_password(STRONG_PW).unwrap();
        let mapper = UserMapper::from_pool(pool.clone());
        let user = mapper
            .create_user(&username, &email, Some(&hash))
            .await
            .unwrap();
        mapper.mark_email_verified(user.id).await.unwrap();
        SeedUser {
            id: user.id,
            username,
            email,
        }
    }

    // ─── POST /login ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn login_valid_credentials_sets_cookie_and_returns_username() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(login)
                .service(register)
                .service(get_current_user),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(serde_json::json!({ "email": user.email, "password": STRONG_PW }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Assert Set-Cookie header contains auth_token and HttpOnly
        let set_cookie = resp
            .headers()
            .get("Set-Cookie")
            .expect("Set-Cookie header missing")
            .to_str()
            .unwrap();
        assert!(set_cookie.contains("auth_token="), "cookie name missing");
        assert!(set_cookie.contains("HttpOnly"), "HttpOnly flag missing");

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], user.username);
        assert!(body.get("token").is_none(), "token should not be in body");
    }

    #[tokio::test]
    async fn login_wrong_password_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(login),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(serde_json::json!({ "email": user.email, "password": "WrongPass99!" }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn login_unknown_email_returns_401_not_404() {
        // User enumeration prevention: unknown user must return 401, not 404.
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(login),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(serde_json::json!({ "email": "nobody@test.com", "password": STRONG_PW }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn login_oauth_user_without_password_returns_401() {
        // OAuth users have no password_hash; password login must be uniformly refused.
        let pool = crate::test_helpers::test_pool().await;
        let n: u64 = rand::random();
        let email = format!("oauth_{n}@test.com");
        UserMapper::from_pool(pool.clone())
            .create_user(&format!("oauthusr_{n}"), &email, None)
            .await
            .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(login),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(serde_json::json!({ "email": email, "password": STRONG_PW }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn login_unverified_user_returns_403() {
        // Create user WITHOUT calling mark_email_verified
        let pool = crate::test_helpers::test_pool().await;
        let n: u64 = rand::random();
        let email = format!("unverified_{n}@test.com");
        let hash = hash_password(STRONG_PW).unwrap();
        UserMapper::from_pool(pool.clone())
            .create_user(&format!("unverified_{n}"), &email, Some(&hash))
            .await
            .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(login),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(serde_json::json!({ "email": email, "password": STRONG_PW }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::FORBIDDEN
        );
    }

    // ─── POST /register ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn register_new_user_returns_200_with_message_and_no_cookie() {
        let pool = crate::test_helpers::test_pool().await;
        let n: u64 = rand::random();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(pool)))
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
                "username": format!("newuser_{n}"),
                "email": format!("newuser_{n}@test.com"),
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
            .is_some_and(|s| s.contains("auth_token="));
        assert!(
            !has_auth_cookie,
            "register must not issue auth cookie before verification"
        );

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(
            body.get("message").is_some(),
            "response must contain a 'message' field"
        );
    }

    #[tokio::test]
    async fn register_duplicate_email_returns_200_and_refreshes_token() {
        // Anti-enumeration: duplicate email must return 200 (same shape as
        // success) so an attacker cannot probe which emails are registered.
        let pool = crate::test_helpers::test_pool().await;
        // Seed an unverified user (no mark_email_verified) so the resend path runs.
        let n: u64 = rand::random();
        let unverified_email = format!("dup_unverified_{n}@test.com");
        let user = UserMapper::from_pool(pool.clone())
            .create_user(
                &format!("dup_{n}"),
                &unverified_email,
                Some(&hash_password(STRONG_PW).unwrap()),
            )
            .await
            .unwrap();
        let ev_mapper = EmailVerificationMapper::from_pool(pool.clone());
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(ev_mapper.clone()))
                .app_data(web::Data::new(EmailService::new(
                    crate::config::server::SmtpConfig::default(),
                )))
                .app_data(web::Data::new(AppBaseUrl::new(
                    "http://localhost:5173".to_string(),
                )))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool.clone())))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(register),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": "dupcheck",
                "email": unverified_email,
                "password": STRONG_PW
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // A fresh verification token must have been written for the existing user.
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM email_verification_tokens WHERE user_id = $1")
                .bind(user.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count, 1,
            "verification token must be refreshed on duplicate registration"
        );
    }

    #[tokio::test]
    async fn register_duplicate_verified_email_returns_200_silently() {
        // A verified user re-registering still gets 200 (no enumeration),
        // but we do NOT resend a verification email (they're already verified).
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await; // verified
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(
                    pool.clone(),
                )))
                .app_data(web::Data::new(EmailService::new(
                    crate::config::server::SmtpConfig::default(),
                )))
                .app_data(web::Data::new(AppBaseUrl::new(
                    "http://localhost:5173".to_string(),
                )))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool.clone())))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(register),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": "dupverified",
                "email": user.email,
                "password": STRONG_PW
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn register_weak_password_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let n: u64 = rand::random();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(
                    pool.clone(),
                )))
                .app_data(web::Data::new(EmailService::new(
                    crate::config::server::SmtpConfig::default(),
                )))
                .app_data(web::Data::new(AppBaseUrl::new(
                    "http://localhost:5173".to_string(),
                )))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(register),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": format!("weakpw_{n}"),
                "email": format!("weakpw_{n}@test.com"),
                "password": "weakpassword"
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn register_invalid_email_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let n: u64 = rand::random();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(
                    pool.clone(),
                )))
                .app_data(web::Data::new(EmailService::new(
                    crate::config::server::SmtpConfig::default(),
                )))
                .app_data(web::Data::new(AppBaseUrl::new(
                    "http://localhost:5173".to_string(),
                )))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(register),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": format!("badmail_{n}"),
                "email": "not-an-email",
                "password": STRONG_PW
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn register_username_too_long_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let n: u64 = rand::random();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(
                    pool.clone(),
                )))
                .app_data(web::Data::new(EmailService::new(
                    crate::config::server::SmtpConfig::default(),
                )))
                .app_data(web::Data::new(AppBaseUrl::new(
                    "http://localhost:5173".to_string(),
                )))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(register),
        )
        .await;
        let long_name = "a".repeat(51);
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": long_name,
                "email": format!("toolong_{n}@test.com"),
                "password": STRONG_PW
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    // ─── GET /user ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_current_user_with_valid_jwt_returns_username() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_current_user),
        )
        .await;
        let token = generate_token(&user.id, SECRET).unwrap();
        let req = test::TestRequest::get()
            .uri("/user")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], user.username);
        assert!(body.get("token").is_none(), "token should not be in body");
    }

    #[tokio::test]
    async fn get_current_user_without_cookie_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_current_user),
        )
        .await;
        let req = test::TestRequest::get().uri("/user").to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    // ─── POST /auth/logout ────────────────────────────────────────────────────

    #[tokio::test]
    async fn logout_clears_auth_cookie() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(get_current_user)
                .service(logout),
        )
        .await;

        let token = generate_token(&user.id, SECRET).unwrap();
        let req = test::TestRequest::post()
            .uri("/auth/logout")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // The Set-Cookie header should clear auth_token (Max-Age=0)
        let set_cookie = resp
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(set_cookie.contains("auth_token="), "cookie name missing");
        assert!(
            set_cookie.contains("Max-Age=0"),
            "Max-Age=0 missing — cookie not cleared"
        );
    }

    #[tokio::test]
    async fn logout_without_cookie_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(logout),
        )
        .await;
        let req = test::TestRequest::post().uri("/auth/logout").to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
