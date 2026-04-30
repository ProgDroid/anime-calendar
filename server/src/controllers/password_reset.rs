use actix_web::{HttpResponse, ResponseError, post, web};
use log::error;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    config::server::AppBaseUrl,
    controllers::auth::{ErrorResponse, MessageResponse},
    error::Error,
    mappers::{password_reset::PasswordResetMapper, user::UserMapper},
    services::{
        auth::{generate_random_token, hash_password, hash_token, validate_password_strength},
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

const RESET_RESPONSE: &str =
    "If an account exists with that email, you'll receive a reset link shortly.";

#[utoipa::path(
    post,
    path = "/auth/forgot-password",
    tag = "auth",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Always returned — prevents user enumeration", body = MessageResponse),
    )
)]
#[post("/auth/forgot-password")]
#[allow(clippy::future_not_send)]
pub async fn forgot_password(
    user_mapper: web::Data<UserMapper>,
    token_mapper: web::Data<PasswordResetMapper>,
    email_service: web::Data<EmailService>,
    app_base_url: web::Data<AppBaseUrl>,
    body: web::Json<ForgotPasswordRequest>,
) -> HttpResponse {
    let ok = || {
        HttpResponse::Ok().json(MessageResponse {
            message: RESET_RESPONSE.into(),
        })
    };

    // Always return 200 — do not reveal whether the account exists.
    let Ok(user) = user_mapper.get_user_by_email(&body.email).await else {
        return ok();
    };

    // OAuth-only users have no password — silently do nothing.
    if user.password_hash.is_none() {
        return ok();
    }

    let raw_token = generate_random_token();
    let token_hash = hash_token(&raw_token);
    let reset_url = format!("{}/reset-password?token={raw_token}", app_base_url.as_str());

    // Best-effort cleanup — don't abort the flow if old tokens can't be deleted.
    if let Err(e) = token_mapper.invalidate_previous_tokens(user.id).await {
        error!(
            "Failed to invalidate previous reset tokens for user {}: {e}",
            user.id
        );
    }

    if let Err(e) = token_mapper.create_token(user.id, &token_hash).await {
        error!("{e}");
        return ok();
    }

    if let Err(e) = email_service
        .send_password_reset(&user.email, &reset_url)
        .await
    {
        error!("{e}");
        return ok();
    }

    metrics::counter!(crate::metrics::names::PASSWORD_RESETS_REQUESTED_TOTAL).increment(1);
    ok()
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
    use crate::metrics::names::{
        LABEL_OUTCOME, OUTCOME_FAILED, OUTCOME_OK, PASSWORD_RESETS_COMPLETED_TOTAL,
    };
    let record = |outcome: &'static str| {
        metrics::counter!(PASSWORD_RESETS_COMPLETED_TOTAL, LABEL_OUTCOME => outcome).increment(1);
    };

    // Validate password before touching the DB.
    if body.new_password.len() > 128 || !validate_password_strength(&body.new_password) {
        record(OUTCOME_FAILED);
        return Error::InvalidPassword.error_response();
    }

    let token_hash = hash_token(&body.token);

    let Ok(token) = token_mapper.find_valid_token(&token_hash).await else {
        record(OUTCOME_FAILED);
        return Error::InvalidResetToken.error_response();
    };

    let new_hash = match hash_password(&body.new_password) {
        Ok(h) => h,
        Err(e) => {
            record(OUTCOME_FAILED);
            return e.error_response();
        }
    };

    match token_mapper
        .complete_reset(token.id, token.user_id, &new_hash)
        .await
    {
        Ok(()) => {
            record(OUTCOME_OK);
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            error!("{e}");
            record(OUTCOME_FAILED);
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
    use actix_web::{App, http::StatusCode, test, web};

    const STRONG_PW: &str = "SecurePass12!@";

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

    async fn seed_user(pool: &sqlx::PgPool) -> SeedUser {
        let n: u64 = rand::random();
        let email = format!("pwreset_{n}@test.com");
        let mapper = UserMapper::from_pool(pool.clone());
        let user = mapper
            .create_user(
                &format!("pwreset_{n}"),
                &email,
                Some(&hash_password(STRONG_PW).unwrap()),
            )
            .await
            .unwrap();
        mapper.mark_email_verified(user.id).await.unwrap();
        SeedUser { id: user.id, email }
    }

    // ─── POST /auth/forgot-password ──────────────────────────────────────────

    #[tokio::test]
    async fn forgot_password_known_email_returns_200() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(PasswordResetMapper::from_pool(pool.clone())))
                .app_data(dev_email())
                .app_data(base_url())
                .service(forgot_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/forgot-password")
            .set_json(serde_json::json!({ "email": user.email }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

        // Verify a token row was actually inserted for this user (not just 200 returned).
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM password_reset_tokens WHERE user_id = $1")
                .bind(user.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1, "expected one token row for the seeded user");
    }

    #[tokio::test]
    async fn forgot_password_unknown_email_also_returns_200() {
        let pool = crate::test_helpers::test_pool().await;
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

    #[tokio::test]
    async fn forgot_password_oauth_user_returns_200_without_creating_token() {
        let pool = crate::test_helpers::test_pool().await;
        let n: u64 = rand::random();
        UserMapper::from_pool(pool.clone())
            .create_user(
                &format!("oauthpw_{n}"),
                &format!("oauthpw_{n}@test.com"),
                None,
            )
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
            .set_json(serde_json::json!({ "email": format!("oauthpw_{n}@test.com") }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
        assert!(token_mapper.find_valid_token("anything").await.is_err());
    }

    // ─── POST /auth/reset-password ───────────────────────────────────────────

    #[tokio::test]
    async fn reset_password_valid_token_returns_200() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let token_mapper = PasswordResetMapper::from_pool(pool.clone());

        let n: u64 = rand::random();
        let raw = format!("tok_{n}");
        let hash = hash_token(&raw);
        token_mapper.create_token(user.id, &hash).await.unwrap();

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

    #[tokio::test]
    async fn reset_password_invalid_token_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
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

    #[tokio::test]
    async fn reset_password_used_token_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let token_mapper = PasswordResetMapper::from_pool(pool.clone());

        let n: u64 = rand::random();
        let raw = format!("tok_{n}");
        let hash = hash_token(&raw);
        token_mapper.create_token(user.id, &hash).await.unwrap();
        let token = token_mapper.find_valid_token(&hash).await.unwrap();
        token_mapper
            .complete_reset(token.id, user.id, &hash_password("TempPass12!@").unwrap())
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

    #[tokio::test]
    async fn reset_password_weak_new_password_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
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
