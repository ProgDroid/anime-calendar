use actix_web::{post, web, HttpResponse, ResponseError};
use log::error;
use serde::Deserialize;
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

    let Ok(token) = verification_mapper.find_valid_token(&token_hash).await else {
        return Error::InvalidVerificationToken.error_response();
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
    HttpResponse::Ok().cookie(cookie).json(AuthResponse {
        username: user.username,
    })
}

#[utoipa::path(
    post,
    path = "/auth/resend-verification",
    tag = "auth",
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Always returned — prevents user enumeration", body = MessageResponse),
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
    // Use a closure so the response can be constructed at each return site
    // without bumping into `HttpResponse`'s non-Clone / non-Copy restriction.
    let ok = || {
        HttpResponse::Ok().json(MessageResponse {
            message: RESEND_RESPONSE.into(),
        })
    };

    let Ok(user) = user_mapper.get_user_by_email(&body.email).await else {
        return ok();
    };

    // Already verified — silently do nothing
    if user.email_verified_at.is_some() {
        return ok();
    }

    let raw_token = generate_random_token();
    let token_hash = hash_token(&raw_token);
    let verify_url = format!("{}/verify-email?token={raw_token}", app_base_url.as_str());

    // Best-effort operations — errors are logged but always return 200 to
    // preserve the anti-enumeration guarantee.
    if let Err(e) = verification_mapper
        .replace_token(user.id, &token_hash)
        .await
    {
        error!("{e}");
        return ok();
    }

    if let Err(e) = email_service
        .send_verification_email(&user.email, &verify_url)
        .await
    {
        error!("{e}");
        return ok();
    }

    ok()
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        config::server::{AppBaseUrl, CookieSettings, JwtSecret, SmtpConfig},
        mappers::{email_verification::EmailVerificationMapper, user::UserMapper},
        services::{
            auth::{hash_password, hash_token},
            email::EmailService,
        },
    };
    use actix_web::{http::StatusCode, test, web, App};
    use secrecy::SecretString;

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

    struct SeedUser {
        id: i32,
        email: String,
    }

    async fn seed_unverified_user(pool: &sqlx::PgPool) -> SeedUser {
        let n: u64 = rand::random();
        let email = format!("verify_{n}@test.com");
        let mapper = UserMapper::from_pool(pool.clone());
        let user = mapper
            .create_user(
                &format!("verifyme_{n}"),
                &email,
                Some(&hash_password(STRONG_PW).unwrap()),
            )
            .await
            .unwrap();
        SeedUser { id: user.id, email }
    }

    // ─── POST /auth/verify-email ─────────────────────────────────────────────

    #[tokio::test]
    async fn verify_email_valid_token_issues_cookie_and_marks_verified() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_unverified_user(&pool).await;
        let ev_mapper = EmailVerificationMapper::from_pool(pool.clone());
        let n: u64 = rand::random();
        let raw = format!("validtok_{n}");
        ev_mapper
            .replace_token(user.id, &hash_token(&raw))
            .await
            .unwrap();

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
                .bind(user.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(verified_at.is_some(), "user must be verified in DB");
    }

    #[tokio::test]
    async fn verify_email_invalid_token_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
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
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    // ─── POST /auth/resend-verification ─────────────────────────────────────

    #[tokio::test]
    async fn resend_verification_known_unverified_email_returns_200() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_unverified_user(&pool).await;
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
            .set_json(serde_json::json!({ "email": user.email }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

        // Token row should exist after resend
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM email_verification_tokens WHERE user_id = $1")
                .bind(user.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1, "one token row must exist after resend");
    }

    #[tokio::test]
    async fn resend_verification_unknown_email_also_returns_200() {
        let pool = crate::test_helpers::test_pool().await;
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
