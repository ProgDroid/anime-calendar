use crate::config::server::{AppBaseUrl, CookieSettings, JwtSecret};
use crate::error::Error;
use crate::mappers::email_verification::EmailVerificationMapper;
use crate::mappers::user::UserMapper;
use crate::middleware::auth::Claims;
use crate::services::auth::{
    generate_random_token, generate_token, hash_password, hash_token, validate_password,
    validate_password_strength, verify_token,
};
use crate::services::email::EmailService;
use actix_web::cookie::{time::Duration, Cookie, SameSite};
use actix_web::{get, post, web, HttpResponse, ResponseError};
use log::error;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

/// Generic error response body
#[derive(Serialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    domain
        .rfind('.')
        .is_some_and(|pos| pos > 0 && pos < domain.len() - 1)
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
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

/// Build an httpOnly auth cookie from a JWT token and cookie settings.
///
/// # Errors
/// This function is infallible; it returns a `Cookie` directly.
#[must_use]
pub fn build_auth_cookie(token: String, cookie_settings: &CookieSettings) -> Cookie<'static> {
    Cookie::build("auth_token", token)
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/")
        .max_age(Duration::hours(24))
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
                // Block login if email has not been verified yet.
                if user.email_verified_at.is_none() {
                    return Error::EmailNotVerified.error_response();
                }

                let token = match generate_token(&user.id, jwt_secret.expose_secret()) {
                    Ok(token) => token,
                    Err(e) => return e.error_response(),
                };

                let cookie = build_auth_cookie(token, &cookie_settings);
                HttpResponse::Ok().cookie(cookie).json(AuthResponse {
                    username: user.username,
                })
            } else {
                Error::Unauthorised.error_response()
            }
        }
        None => Error::Unauthorised.error_response(),
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
        // Compensate: delete the newly-created user so the email address is
        // not permanently stuck in an unverifiable state. Best-effort only.
        if let Err(del_err) = db.delete_user(user.id).await {
            error!("Failed to rollback user {} after token error: {del_err}", user.id);
        }
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
pub async fn logout(_claims: Claims, cookie_settings: web::Data<CookieSettings>) -> HttpResponse {
    let removal_cookie = Cookie::build("auth_token", "")
        .http_only(true)
        .same_site(SameSite::Strict)
        .path("/api/")
        .max_age(Duration::ZERO)
        .secure(cookie_settings.secure)
        .finish();
    HttpResponse::Ok()
        .cookie(removal_cookie)
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
        mappers::{email_verification::EmailVerificationMapper, user::UserMapper},
        services::{auth::{generate_token, hash_password}, email::EmailService},
    };
    use actix_web::{http::StatusCode, test, web, App};
    use secrecy::SecretString;
    use serde_json::Value;
    use sqlx::PgPool;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";
    const STRONG_PW: &str = "SecurePass12!@";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    fn cookie_data() -> web::Data<CookieSettings> {
        web::Data::new(CookieSettings { secure: false })
    }

    /// Seed a password-based user and return its id.
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

    // ─── POST /login ──────────────────────────────────────────────────────────

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
        assert_eq!(body["username"], "alice");
        assert!(body.get("token").is_none(), "token should not be in body");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn login_wrong_password_returns_401(pool: PgPool) {
        seed_user(&pool, "bob", "bob@test.com").await;
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
            .set_json(serde_json::json!({ "email": "bob@test.com", "password": "WrongPass99!" }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn login_unknown_email_returns_401_not_404(pool: PgPool) {
        // User enumeration prevention: unknown user must return 401, not 404.
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
            .set_json(serde_json::json!({ "email": "nobody@test.com", "password": STRONG_PW }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn login_oauth_user_without_password_returns_401(pool: PgPool) {
        // OAuth users have no password_hash; password login must be uniformly refused.
        UserMapper::from_pool(pool.clone())
            .create_user("oauth_user", "oauth@test.com", None)
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
            .set_json(serde_json::json!({ "email": "oauth@test.com", "password": STRONG_PW }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn login_unverified_user_returns_403(pool: PgPool) {
        // Create user WITHOUT calling mark_email_verified
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

    // ─── POST /register ───────────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn register_new_user_returns_200_with_message_and_no_cookie(pool: PgPool) {
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

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body.get("message").is_some(), "response must contain a 'message' field");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn register_duplicate_email_returns_400(pool: PgPool) {
        seed_user(&pool, "dave", "dave@test.com").await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(pool)))
                .app_data(web::Data::new(EmailService::new(crate::config::server::SmtpConfig::default())))
                .app_data(web::Data::new(AppBaseUrl::new("http://localhost:5173".to_string())))
                .service(register),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": "dave2",
                "email": "dave@test.com",
                "password": STRONG_PW
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn register_weak_password_returns_400(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(pool)))
                .app_data(web::Data::new(EmailService::new(crate::config::server::SmtpConfig::default())))
                .app_data(web::Data::new(AppBaseUrl::new("http://localhost:5173".to_string())))
                .service(register),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": "eve",
                "email": "eve@test.com",
                "password": "weakpassword"
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn register_invalid_email_returns_400(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(pool)))
                .app_data(web::Data::new(EmailService::new(crate::config::server::SmtpConfig::default())))
                .app_data(web::Data::new(AppBaseUrl::new("http://localhost:5173".to_string())))
                .service(register),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": "frank",
                "email": "not-an-email",
                "password": STRONG_PW
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn register_username_too_long_returns_400(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(EmailVerificationMapper::from_pool(pool)))
                .app_data(web::Data::new(EmailService::new(crate::config::server::SmtpConfig::default())))
                .app_data(web::Data::new(AppBaseUrl::new("http://localhost:5173".to_string())))
                .service(register),
        )
        .await;
        let long_name = "a".repeat(51);
        let req = test::TestRequest::post()
            .uri("/register")
            .set_json(serde_json::json!({
                "username": long_name,
                "email": "toolong@test.com",
                "password": STRONG_PW
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    // ─── GET /user ────────────────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn get_current_user_with_valid_jwt_returns_username(pool: PgPool) {
        let user_id = seed_user(&pool, "grace", "grace@test.com").await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
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
        assert!(body.get("token").is_none(), "token should not be in body");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_current_user_without_cookie_returns_401(pool: PgPool) {
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
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
