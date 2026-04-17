use crate::{
    cache::Cache,
    error::Error,
    mappers::user::UserMapper,
    middleware::auth::Claims,
    services::auth::{hash_password, validate_password},
};

use crate::entity::user_settings::UserSettings;
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

    // Validate username length
    if user_data.username.len() > 50 {
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
            // Invalidate user cache
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
    user_settings_mapper: web::Data<UserSettingsMapper>,
    claims: Claims,
) -> HttpResponse {
    let Ok(user_id) = claims.sub.parse::<i32>() else {
        return Error::Unauthorised.error_response();
    };

    match user_settings_mapper.get_user_settings(user_id).await {
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
    user_settings_mapper: web::Data<UserSettingsMapper>,
    cache: web::Data<Cache>,
    claims: Claims,
    settings_data: web::Json<UserSettings>,
) -> HttpResponse {
    let Ok(user_id) = claims.sub.parse::<i32>() else {
        return Error::Unauthorised.error_response();
    };

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
    use crate::mappers::user::UserMapper;
    use crate::mappers::user_settings::UserSettingsMapper;
    use crate::services::auth::{generate_token, hash_password};
    use actix_web::{http::StatusCode, test, web, App};
    use secrecy::SecretString;
    use serde_json::Value;
    use sqlx::PgPool;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";
    const STRONG_PW: &str = "SecurePass12!@";
    const NEW_PW: &str = "NewSecure99#$";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    async fn seed_user(pool: &PgPool, username: &str, email: &str) -> (i32, String) {
        let hash = hash_password(STRONG_PW).unwrap();
        let user = UserMapper::from_pool(pool.clone())
            .create_user(username, email, Some(&hash))
            .await
            .unwrap();
        let token = generate_token(&user.id, SECRET).unwrap();
        (user.id, token)
    }

    async fn seed_oauth_user(pool: &PgPool, username: &str, email: &str) -> (i32, String) {
        let user = UserMapper::from_pool(pool.clone())
            .create_user(username, email, None)
            .await
            .unwrap();
        let token = generate_token(&user.id, SECRET).unwrap();
        (user.id, token)
    }

    // ─── GET /user/details ───────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn get_user_details_password_user_returns_username_and_email(pool: PgPool) {
        let (_, token) = seed_user(&pool, "alice", "alice@test.com").await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_details),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/user/details")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], "alice");
        assert_eq!(body["email"], "alice@test.com");
        assert_eq!(body["is_oauth"], false);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_user_details_oauth_user_returns_empty_username_and_is_oauth_true(pool: PgPool) {
        let (_, token) = seed_oauth_user(&pool, "bob_oauth", "bob@test.com").await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_details),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/user/details")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], "");
        assert_eq!(body["is_oauth"], true);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn get_user_details_without_token_returns_401(pool: PgPool) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_details),
        )
        .await;
        let req = test::TestRequest::get().uri("/user/details").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }

    // ─── PUT /user ───────────────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn update_user_changes_username_and_email(pool: PgPool) {
        let (_, token) = seed_user(&pool, "carol", "carol@test.com").await;
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
            .insert_header(("Cookie", format!("auth_token={token}")))
            .set_json(serde_json::json!({ "username": "carol2", "email": "carol2@test.com" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["username"], "carol2");
        assert_eq!(body["email"], "carol2@test.com");
        assert_eq!(body["is_oauth"], false);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_user_username_too_long_returns_400(pool: PgPool) {
        let (_, token) = seed_user(&pool, "dave", "dave@test.com").await;
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
            .insert_header(("Cookie", format!("auth_token={token}")))
            .set_json(serde_json::json!({ "username": long_name, "email": "dave@test.com" }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::BAD_REQUEST);
    }

    // ─── POST /user/password ─────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn update_password_with_correct_current_returns_200(pool: PgPool) {
        let (_, token) = seed_user(&pool, "eve", "eve@test.com").await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .set_json(serde_json::json!({
                "current_password": STRONG_PW,
                "new_password": NEW_PW
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_password_wrong_current_returns_401(pool: PgPool) {
        let (_, token) = seed_user(&pool, "frank", "frank@test.com").await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .set_json(serde_json::json!({
                "current_password": "WrongPass99!",
                "new_password": NEW_PW
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_password_same_as_current_returns_400(pool: PgPool) {
        let (_, token) = seed_user(&pool, "grace", "grace@test.com").await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .set_json(serde_json::json!({
                "current_password": STRONG_PW,
                "new_password": STRONG_PW
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::BAD_REQUEST);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn update_password_oauth_user_returns_400(pool: PgPool) {
        let (_, token) = seed_oauth_user(&pool, "henry_oauth", "henry@test.com").await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(update_password),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/user/password")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .set_json(serde_json::json!({
                "current_password": "anything",
                "new_password": NEW_PW
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::BAD_REQUEST);
    }

    // ─── DELETE /user/{id} ───────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn delete_user_own_account_returns_204(pool: PgPool) {
        let (user_id, token) = seed_user(&pool, "ida", "ida@test.com").await;
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
            .uri(&format!("/user/{user_id}"))
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::NO_CONTENT);
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn delete_user_another_account_returns_403(pool: PgPool) {
        let (_, token_a) = seed_user(&pool, "jack", "jack@test.com").await;
        let (id_b, _) = seed_user(&pool, "kate", "kate@test.com").await;
        let cache = web::Data::new(Cache::for_tests().await);
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cache)
                .service(delete_user),
        )
        .await;
        // jack (token_a) tries to delete kate (id_b)
        let req = test::TestRequest::delete()
            .uri(&format!("/user/{id_b}"))
            .insert_header(("Cookie", format!("auth_token={token_a}")))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::FORBIDDEN);
    }

    // ─── GET /user/settings ──────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn get_user_settings_returns_defaults_when_none_saved(pool: PgPool) {
        let (user_id, _) = seed_user(&pool, "leo", "leo@test.com").await;
        let token = generate_token(&user_id, SECRET).unwrap();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserSettingsMapper::from_pool(pool)))
                .app_data(jwt_data())
                .service(get_user_settings),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/user/settings")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        // Default values from UserSettings::default()
        assert_eq!(body["theme_preference"], "dark");
        assert_eq!(body["language_preference"], "en");
    }

    // ─── PUT /user/settings ──────────────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations")]
    async fn update_user_settings_persists_new_values(pool: PgPool) {
        let (user_id, _) = seed_user(&pool, "mia", "mia@test.com").await;
        let token = generate_token(&user_id, SECRET).unwrap();
        let cache = web::Data::new(Cache::for_tests().await);
        let mapper = web::Data::new(UserSettingsMapper::from_pool(pool.clone()));
        let app = test::init_service(
            App::new()
                .app_data(mapper.clone())
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
            .insert_header(("Cookie", format!("auth_token={token}")))
            .set_json(serde_json::json!({
                "theme_preference": "light",
                "language_preference": "pt",
                "title_language_preference": "Romaji",
                "timezone": "Europe/Lisbon"
            }))
            .to_request();
        assert_eq!(test::call_service(&app, put_req).await.status(), StatusCode::OK);

        // Read back and verify
        let get_req = test::TestRequest::get()
            .uri("/user/settings")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        let resp = test::call_service(&app, get_req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["theme_preference"], "light");
        assert_eq!(body["language_preference"], "pt");
        assert_eq!(body["timezone"], "Europe/Lisbon");
    }
}
