use crate::config::server::JwtSecret;
use crate::error::Error;
use crate::mappers::user::UserMapper;
use crate::middleware::auth::Claims;
use crate::services::auth::{
    generate_token, hash_password, validate_password, validate_password_strength, verify_token,
};
use actix_web::{get, post, web, HttpResponse, ResponseError};
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
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

#[utoipa::path(
    post,
    path = "/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
    )
)]
#[post("/login")]
pub async fn login(
    db: web::Data<UserMapper>,
    credentials: web::Json<LoginRequest>,
    jwt_secret: web::Data<JwtSecret>,
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

                let response = LoginResponse {
                    token,
                    username: user.username,
                };
                HttpResponse::Ok().json(response)
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
        (status = 200, description = "Registration successful", body = LoginResponse),
        (status = 400, description = "Invalid request or weak password", body = ErrorResponse),
    )
)]
#[post("/register")]
pub async fn register(
    db: web::Data<UserMapper>,
    user_data: web::Json<RegisterRequest>,
    jwt_secret: web::Data<JwtSecret>,
) -> HttpResponse {
    // Check if user already exists
    if (db.get_user_by_email(&user_data.email).await).is_ok() {
        return Error::UserAlreadyExists.error_response();
    }

    // Validate username length
    if user_data.username.len() > 50 {
        return Error::InvalidRequest.error_response();
    }

    // Validate email format
    if !is_valid_email(&user_data.email) {
        return Error::InvalidRequest.error_response();
    }

    // Validate password length
    if user_data.password.expose_secret().len() > 128 {
        return Error::InvalidRequest.error_response();
    }

    // Validate password strength
    if !validate_password_strength(user_data.password.expose_secret()) {
        return Error::InvalidPassword.error_response();
    }

    let hashed_password = match hash_password(user_data.password.expose_secret()) {
        Ok(hashed_password) => hashed_password,
        Err(e) => return e.error_response(),
    };

    let user = match db
        .create_user(
            &user_data.username,
            &user_data.email,
            Some(&hashed_password),
        )
        .await
    {
        Ok(user) => user,
        Err(e) => return e.error_response(),
    };

    let token = match generate_token(&user.id, jwt_secret.expose_secret()) {
        Ok(token) => token,
        Err(e) => return e.error_response(),
    };

    let response = LoginResponse {
        token,
        username: user.username,
    };

    HttpResponse::Ok().json(response)
}

#[utoipa::path(
    get,
    path = "/user",
    tag = "auth",
    responses(
        (status = 200, description = "Current authenticated user", body = LoginResponse),
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

    let response = LoginResponse {
        token: String::new(), // Not returning token for this endpoint
        username: user.username,
    };

    HttpResponse::Ok().json(response)
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
    use crate::mappers::user::UserMapper;
    use crate::services::auth::{generate_token, hash_password};
    use actix_web::{http::StatusCode, test, web, App};
    use secrecy::SecretString;
    use serde_json::Value;
    use sqlx::PgPool;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";
    const STRONG_PW: &str = "SecurePass12!@";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    /// Seed a password-based user and return its id.
    async fn seed_user(pool: &PgPool, username: &str, email: &str) -> i32 {
        let hash = hash_password(STRONG_PW).unwrap();
        UserMapper::from_pool(pool.clone())
            .create_user(username, email, Some(&hash))
            .await
            .unwrap()
            .id
    }

    // ─── POST /login ──────────────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn login_valid_credentials_returns_token_and_username(pool: PgPool) {
        seed_user(&pool, "alice", "alice@test.com").await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
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
        let body: Value = test::read_body_json(resp).await;
        assert!(body["token"].as_str().is_some_and(|t| !t.is_empty()));
        assert_eq!(body["username"], "alice");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn login_wrong_password_returns_401(pool: PgPool) {
        seed_user(&pool, "bob", "bob@test.com").await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(login),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(serde_json::json!({ "email": "bob@test.com", "password": "WrongPass99!" }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn login_unknown_email_returns_401_not_404(pool: PgPool) {
        // User enumeration prevention: unknown user must return 401, not 404.
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(login),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(serde_json::json!({ "email": "nobody@test.com", "password": STRONG_PW }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
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
                .service(login),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/login")
            .set_json(serde_json::json!({ "email": "oauth@test.com", "password": STRONG_PW }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }

    // ─── POST /register ───────────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn register_new_user_returns_token_and_username(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
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
        let body: Value = test::read_body_json(resp).await;
        assert!(body["token"].as_str().is_some_and(|t| !t.is_empty()));
        assert_eq!(body["username"], "carol");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn register_duplicate_email_returns_400(pool: PgPool) {
        seed_user(&pool, "dave", "dave@test.com").await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
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
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn register_weak_password_returns_400(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
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
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn register_invalid_email_returns_400(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
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
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn register_username_too_long_returns_400(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
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
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::BAD_REQUEST);
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
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], "grace");
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_current_user_without_token_returns_401(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_current_user),
        )
        .await;
        let req = test::TestRequest::get().uri("/user").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }
}
