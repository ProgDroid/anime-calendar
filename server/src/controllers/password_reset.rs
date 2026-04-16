use actix_web::{post, web, HttpResponse, ResponseError};
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
    rand::rng().fill_bytes(&mut bytes);
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
        assert!(token_mapper.find_valid_token("anything").await.is_err());
    }

    // ─── POST /auth/reset-password ───────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn reset_password_valid_token_returns_200(pool: PgPool) {
        let user_id = seed_user(&pool, "bob@test.com").await;
        let token_mapper = PasswordResetMapper::from_pool(pool.clone());

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
