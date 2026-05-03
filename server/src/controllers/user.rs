use crate::{
    cache::Cache,
    error::Error,
    mappers::user::UserMapper,
    middleware::auth::Claims,
    services::auth::{hash_password, validate_password},
};

use crate::entity::user_settings::UserSettings;
use crate::mappers::refresh_token::RefreshTokenMapper;
use crate::mappers::user_settings::UserSettingsMapper;
use actix_web::{delete, get, post, put, web, HttpResponse, ResponseError};
use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UserRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct UserResponse {
    pub username: String,
    pub email: String,
    pub is_oauth: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateUserRequest {
    pub username: String,
    pub email: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[utoipa::path(
    get,
    path = "/user/details",
    tag = "user",
    responses(
        (status = 200, body = UserResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/user/details")]
pub async fn get_user_details(user_mapper: web::Data<UserMapper>, claims: Claims) -> HttpResponse {
    match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => {
            let response = UserResponse {
                username: if user.password_hash.is_none() {
                    String::new()
                } else {
                    user.username
                },
                email: user.email,
                is_oauth: user.password_hash.is_none(),
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => e.error_response(),
    }
}

#[utoipa::path(
    put,
    path = "/user",
    tag = "user",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, body = UserResponse),
        (status = 400, body = crate::controllers::auth::ErrorResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[put("/user")]
pub async fn update_user(
    user_mapper: web::Data<UserMapper>,
    cache: web::Data<Cache>,
    claims: Claims,
    user_data: web::Json<UpdateUserRequest>,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Validate username length (chars, not bytes — emoji/multibyte-safe)
    if user_data.username.chars().count() > 50 {
        return Error::InvalidRequest.error_response();
    }

    let user = user_mapper
        .update_user(
            user.id,
            user_data.username.as_ref(),
            user_data.email.as_ref(),
        )
        .await;

    match user {
        Ok(user) => {
            // Invalidate user cache
            let _ = cache.invalidate_user_details(user.id).await;

            let response = UserResponse {
                username: if user.password_hash.is_none() {
                    String::new()
                } else {
                    user.username
                },
                email: user.email,
                is_oauth: user.password_hash.is_none(),
            };

            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[utoipa::path(
    post,
    path = "/user/password",
    tag = "user",
    request_body = UpdatePasswordRequest,
    responses(
        (status = 200, description = "Password updated"),
        (status = 400, body = crate::controllers::auth::ErrorResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[post("/user/password")]
pub async fn update_password(
    user_mapper: web::Data<UserMapper>,
    refresh_token_mapper: web::Data<RefreshTokenMapper>,
    cache: web::Data<Cache>,
    claims: Claims,
    password_data: web::Json<UpdatePasswordRequest>,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Check that new password is different from current password
    if password_data.new_password == password_data.current_password {
        return Error::InvalidRequest.error_response();
    }

    // Validate password length
    if password_data.new_password.len() > 128 {
        return Error::InvalidRequest.error_response();
    }

    // Verify current password
    if let Some(hash) = user.password_hash {
        if !validate_password(&password_data.current_password, &hash) {
            return Error::Unauthorised.error_response();
        }
    } else {
        info!("OAuth user attempted to change password: {}", user.id);
        return Error::InvalidRequest.error_response();
    }

    // Hash new password
    let hashed_password = match hash_password(&password_data.new_password) {
        Ok(hashed_password) => hashed_password,
        Err(e) => return e.error_response(),
    };

    // Update password in database
    match user_mapper
        .update_user_password(user.id, &hashed_password)
        .await
    {
        Ok(()) => {
            // Log out all other devices — anyone who had a session before the
            // password change can no longer silently stay authenticated.
            if let Err(e) = refresh_token_mapper.invalidate_all_for_user(user.id).await {
                error!(
                    "Failed to invalidate refresh tokens for user {}: {e}",
                    user.id
                );
            }
            let _ = cache.invalidate_user_details(user.id).await;
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/user/{id}",
    tag = "user",
    params(("id" = i32, Path, description = "User ID to delete (must match authenticated user)")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 403, description = "Forbidden — cannot delete another user"),
    ),
    security(("bearer_auth" = []))
)]
#[delete("/user/{id}")]
pub async fn delete_user(
    user_mapper: web::Data<UserMapper>,
    cache: web::Data<Cache>,
    user_id: web::Path<i32>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    let user_id_inner = user_id.into_inner();

    if user.id != user_id_inner {
        return HttpResponse::Forbidden().finish();
    }

    match user_mapper.delete_user(user_id_inner).await {
        Ok(()) => {
            // Invalidate user cache
            let _ = cache.invalidate_user_details(user_id_inner).await;
            HttpResponse::NoContent().finish()
        }
        Err(e) => {
            error!("{e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

// User settings endpoints
#[utoipa::path(
    get,
    path = "/user/settings",
    tag = "user",
    responses(
        (status = 200, body = crate::entity::user_settings::UserSettings),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/user/settings")]
pub async fn get_user_settings(
    user_mapper: web::Data<UserMapper>,
    user_settings_mapper: web::Data<UserSettingsMapper>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    match user_settings_mapper.get_user_settings(user.id).await {
        Ok(settings) => HttpResponse::Ok().json(settings),
        Err(e) => e.error_response(),
    }
}

#[utoipa::path(
    put,
    path = "/user/settings",
    tag = "user",
    request_body = crate::entity::user_settings::UserSettings,
    responses(
        (status = 200, description = "Settings updated"),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[put("/user/settings")]
pub async fn update_user_settings(
    user_mapper: web::Data<UserMapper>,
    user_settings_mapper: web::Data<UserSettingsMapper>,
    entitlement: web::Data<crate::services::entitlement::EntitlementService>,
    cache: web::Data<Cache>,
    claims: Claims,
    settings_data: web::Json<UserSettings>,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };
    let user_id = user.id;

    // Pro accent gate. The frontend should never POST a Pro accent for a
    // free user (AccentPicker emits 'interrupt' instead), but this is the
    // defense-in-depth path for direct API callers.
    if settings_data.accent_preference.is_pro() {
        match entitlement.effective_tier(user_id).await {
            Ok(crate::entity::subscription::Tier::Free) => {
                return crate::error::Error::PaymentRequired {
                    required_tier: "paid",
                }
                .error_response();
            }
            Ok(_) => { /* paid tier — allowed */ }
            Err(e) => {
                error!("entitlement check failed during settings update: {e:?}");
                return e.error_response();
            }
        }
    }

    info!("{settings_data:?}");
    match user_settings_mapper
        .update_user_settings(user_id, &settings_data)
        .await
    {
        Ok(()) => {
            let _ = cache.invalidate_user_settings(user_id).await;
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            error!("Failed to update user settings: {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

// TODO actually use these settings
// TODO default to user's browser locale (navigator.langauges?)
// TODO finish translations for rest of files (it refuses to go beyond Login and Register)
// TODO refactor frontend. Use frontend and Vue expertise skills. It's a mess rn

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::server::JwtSecret;
    use crate::mappers::refresh_token::RefreshTokenMapper;
    use crate::mappers::user::UserMapper;
    use crate::mappers::user_settings::UserSettingsMapper;
    use crate::services::auth::{generate_token, hash_password};
    use actix_web::{http::StatusCode, test, web, App};
    use secrecy::SecretString;
    use serde_json::Value;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";
    const STRONG_PW: &str = "SecurePass12!@";
    const NEW_PW: &str = "NewSecure99#$";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    struct SeedUser {
        id: i32,
        username: String,
        email: String,
        token: String,
    }

    async fn seed_user(pool: &sqlx::PgPool) -> SeedUser {
        let n: u64 = rand::random();
        let username = format!("usertest_{n}");
        let email = format!("usertest_{n}@test.com");
        let hash = hash_password(STRONG_PW).unwrap();
        let user = UserMapper::from_pool(pool.clone())
            .create_user(&username, &email, Some(&hash))
            .await
            .unwrap();
        let token = generate_token(&user.id, SECRET).unwrap();
        SeedUser {
            id: user.id,
            username,
            email,
            token,
        }
    }

    async fn seed_oauth_user(pool: &sqlx::PgPool) -> SeedUser {
        let n: u64 = rand::random();
        let username = format!("oauthusr_{n}");
        let email = format!("oauthusr_{n}@test.com");
        let user = UserMapper::from_pool(pool.clone())
            .create_user(&username, &email, None)
            .await
            .unwrap();
        let token = generate_token(&user.id, SECRET).unwrap();
        SeedUser {
            id: user.id,
            username,
            email,
            token,
        }
    }

    // ─── GET /user/details ───────────────────────────────────────────────────

    #[tokio::test]
    async fn get_user_details_password_user_returns_username_and_email() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_details),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/user/details")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], user.username);
        assert_eq!(body["email"], user.email);
        assert_eq!(body["is_oauth"], false);
    }

    #[tokio::test]
    async fn get_user_details_oauth_user_returns_empty_username_and_is_oauth_true() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_oauth_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_details),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/user/details")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], "");
        assert_eq!(body["is_oauth"], true);
    }

    #[tokio::test]
    async fn get_user_details_without_token_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_details),
        )
        .await;
        let req = test::TestRequest::get().uri("/user/details").to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    // ─── PUT /user ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_user_changes_username_and_email() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let m: u64 = rand::random();
        let new_username = format!("updated_{m}");
        let new_email = format!("updated_{m}@test.com");
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_user),
        )
        .await;
        let req = test::TestRequest::put()
            .uri("/user")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({ "username": new_username, "email": new_email }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], new_username);
        assert_eq!(body["email"], new_email);
        assert_eq!(body["is_oauth"], false);
    }

    #[tokio::test]
    async fn update_user_username_too_long_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_user),
        )
        .await;
        let long_name = "x".repeat(51);
        let req = test::TestRequest::put()
            .uri("/user")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({ "username": long_name, "email": user.email }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    // ─── POST /user/password ─────────────────────────────────────────────────

    #[tokio::test]
    async fn update_password_with_correct_current_returns_200() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "current_password": STRONG_PW,
                "new_password": NEW_PW
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn update_password_invalidates_refresh_tokens() {
        use crate::controllers::auth::hash_refresh_token;

        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let rt_mapper = RefreshTokenMapper::from_pool(pool.clone());

        // Seed an active refresh token for the user.
        rt_mapper
            .replace_token(user.id, &hash_refresh_token("existing_session"))
            .await
            .unwrap();

        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool.clone())))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "current_password": STRONG_PW,
                "new_password": NEW_PW
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
                .bind(user.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "refresh tokens must be wiped after password change"
        );
    }

    #[tokio::test]
    async fn update_password_wrong_current_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "current_password": "WrongPass99!",
                "new_password": NEW_PW
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn update_password_same_as_current_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "current_password": STRONG_PW,
                "new_password": STRONG_PW
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn update_password_oauth_user_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_oauth_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "current_password": "anything",
                "new_password": NEW_PW
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    // ─── DELETE /user/{id} ───────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_user_own_account_returns_204() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(delete_user),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!("/user/{}", user.id))
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::NO_CONTENT
        );
    }

    #[tokio::test]
    async fn delete_user_another_account_returns_403() {
        let pool = crate::test_helpers::test_pool().await;
        let user_a = seed_user(&pool).await;
        let user_b = seed_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(delete_user),
        )
        .await;
        // user_a tries to delete user_b
        let req = test::TestRequest::delete()
            .uri(&format!("/user/{}", user_b.id))
            .insert_header(("Cookie", format!("auth_token={}", user_a.token)))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::FORBIDDEN
        );
    }

    // ─── GET /user/settings ──────────────────────────────────────────────────

    #[tokio::test]
    async fn get_user_settings_returns_defaults_when_none_saved() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(UserSettingsMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_settings),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/user/settings")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        // Default values from UserSettings::default()
        assert_eq!(body["theme_preference"], "dark");
        assert_eq!(body["language_preference"], "en");
    }

    #[tokio::test]
    async fn get_user_settings_deleted_user_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        // Delete the user — their JWT remains valid for up to 30 min.
        UserMapper::from_pool(pool.clone())
            .delete_user(user.id)
            .await
            .unwrap();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(UserSettingsMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_settings),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/user/settings")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    // ─── PUT /user/settings ──────────────────────────────────────────────────

    #[tokio::test]
    async fn update_user_settings_persists_new_values() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let mapper = web::Data::new(UserSettingsMapper::from_pool(pool.clone()));
        let entitlement = web::Data::new(crate::services::entitlement::EntitlementService::new(
            crate::mappers::subscription::SubscriptionMapper::from_pool(pool.clone()),
        ));
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(mapper.clone())
                .app_data(entitlement)
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_user_settings)
                .service(get_user_settings),
        )
        .await;

        // Update settings — body matches the frontend UserSettings interface:
        // title_language_preference uses PascalCase (entity::calendar::Language has no serde rename)
        let put_req = test::TestRequest::put()
            .uri("/user/settings")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "theme_preference": "light",
                "language_preference": "pt",
                "title_language_preference": "Romaji",
                "timezone": "Europe/Lisbon"
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, put_req).await.status(),
            StatusCode::OK
        );

        // Read back and verify
        let get_req = test::TestRequest::get()
            .uri("/user/settings")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        let resp = test::call_service(&app, get_req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["theme_preference"], "light");
        assert_eq!(body["language_preference"], "pt");
        assert_eq!(body["timezone"], "Europe/Lisbon");
    }

    /// Free user requesting a Pro accent → 402 with the documented body
    /// shape `{"error":"upgrade_required","required_tier":"paid"}`. Defense
    /// in depth — the frontend `AccentPicker` also gates this, but a direct
    /// API caller hits this path.
    #[tokio::test]
    async fn update_user_settings_free_user_pro_accent_returns_402() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let cache = web::Data::new(Cache::for_tests().await);
        let mapper = web::Data::new(UserSettingsMapper::from_pool(pool.clone()));
        let entitlement = web::Data::new(crate::services::entitlement::EntitlementService::new(
            crate::mappers::subscription::SubscriptionMapper::from_pool(pool.clone()),
        ));
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(mapper.clone())
                .app_data(entitlement)
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_user_settings),
        )
        .await;

        let put_req = test::TestRequest::put()
            .uri("/user/settings")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "theme_preference": "dark",
                "accent_preference": "matcha",
                "language_preference": "en",
                "title_language_preference": "Romaji",
                "timezone": "UTC"
            }))
            .to_request();
        let resp = test::call_service(&app, put_req).await;
        assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "upgrade_required");
        assert_eq!(body["required_tier"], "paid");
    }

    /// Paid user requesting a Pro accent → 200. Uses an active subscription
    /// row so `EntitlementService` returns `Tier::Paid`.
    #[tokio::test]
    async fn update_user_settings_paid_user_pro_accent_returns_200() {
        use chrono::{Duration, Utc};

        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;

        // Seed an active subscription row so the user reads as paid.
        let now = Utc::now().naive_utc();
        sqlx::query(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, \
              cancel_at_period_end) \
             VALUES ($1, 'paid', 'active', $2, $3, 'price_test', $4, $5, false)",
        )
        .bind(user.id)
        .bind(format!("cus_test_{}", user.id))
        .bind(format!("sub_test_{}", user.id))
        .bind(now)
        .bind(now + Duration::days(30))
        .execute(&pool)
        .await
        .unwrap();

        let cache = web::Data::new(Cache::for_tests().await);
        let mapper = web::Data::new(UserSettingsMapper::from_pool(pool.clone()));
        let entitlement = web::Data::new(crate::services::entitlement::EntitlementService::new(
            crate::mappers::subscription::SubscriptionMapper::from_pool(pool.clone()),
        ));
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(mapper.clone())
                .app_data(entitlement)
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_user_settings),
        )
        .await;

        let put_req = test::TestRequest::put()
            .uri("/user/settings")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "theme_preference": "dark",
                "accent_preference": "matcha",
                "language_preference": "en",
                "title_language_preference": "Romaji",
                "timezone": "UTC"
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, put_req).await.status(),
            StatusCode::OK
        );
    }
}
