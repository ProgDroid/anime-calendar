use crate::config::server::{CookieSettings, JwtSecret};
use crate::controllers::auth::{
    build_auth_cookie, build_refresh_cookie, generate_raw_token, hash_refresh_token,
};
use crate::error::Error;
use crate::mappers::refresh_token::RefreshTokenMapper;
use crate::services::auth::generate_token;
use actix_web::{HttpRequest, HttpResponse, ResponseError, post, web};
use log::error;

#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    responses(
        (status = 200, description = "Token refreshed successfully — new auth and refresh cookies set"),
        (status = 401, description = "Refresh token missing, expired, or already used",
         body = crate::controllers::auth::ErrorResponse),
    )
)]
#[post("/auth/refresh")]
#[allow(clippy::future_not_send)]
pub async fn refresh(
    req: HttpRequest,
    refresh_mapper: web::Data<RefreshTokenMapper>,
    jwt_secret: web::Data<JwtSecret>,
    cookie_settings: web::Data<CookieSettings>,
) -> HttpResponse {
    // Extract the httpOnly refresh_token cookie sent by the browser.
    let raw_token = match req.cookie("refresh_token") {
        Some(c) => c.value().to_owned(),
        None => return Error::Unauthorised.error_response(),
    };

    // Validate: token must exist, be unused, and not expired.
    let token_hash = hash_refresh_token(&raw_token);
    let token_row = match refresh_mapper.find_valid_token(&token_hash).await {
        Ok(row) => row,
        Err(e) => {
            // Replay of a used token is a theft indicator — kill the entire family.
            if let Ok(Some(user_id)) = refresh_mapper
                .find_user_id_for_used_token(&token_hash)
                .await
                && let Err(inv_err) = refresh_mapper.invalidate_all_for_user(user_id).await
            {
                error!("Failed to invalidate token family for user {user_id}: {inv_err}");
            }
            return e.error_response();
        }
    };

    // Issue a fresh 30-min JWT.
    let jwt = match generate_token(&token_row.user_id, jwt_secret.expose_secret()) {
        Ok(t) => t,
        Err(e) => return e.error_response(),
    };

    // Generate and store new refresh token (marks old one as used atomically).
    let new_raw = generate_raw_token();
    let new_hash = hash_refresh_token(&new_raw);
    if let Err(e) = refresh_mapper
        .rotate_token(token_row.id, token_row.user_id, &new_hash)
        .await
    {
        return e.error_response();
    }

    HttpResponse::Ok()
        .cookie(build_auth_cookie(jwt, &cookie_settings))
        .cookie(build_refresh_cookie(new_raw, &cookie_settings))
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::server::CookieSettings;
    use crate::controllers::auth::hash_refresh_token;
    use crate::mappers::refresh_token::RefreshTokenMapper;
    use actix_web::{App, http::StatusCode, test, web};
    use secrecy::SecretString;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    fn cookie_data() -> web::Data<CookieSettings> {
        web::Data::new(CookieSettings {
            secure: false,
            domain: None,
        })
    }

    async fn seed_user_with_refresh_token(pool: &sqlx::PgPool) -> (i32, String) {
        let n: u64 = rand::random();
        let user_id: i32 = sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash) \
             VALUES ($1, $2, 'hash') RETURNING id",
        )
        .bind(format!("refresher_{n}"))
        .bind(format!("refresher_{n}@test.com"))
        .fetch_one(pool)
        .await
        .unwrap();

        let raw = format!("raw_refresh_{n}");
        let h = hash_refresh_token(&raw);
        RefreshTokenMapper::from_pool(pool.clone())
            .replace_token(user_id, &h)
            .await
            .unwrap();

        (user_id, raw)
    }

    #[tokio::test]
    async fn valid_refresh_token_sets_new_cookies() {
        let pool = crate::test_helpers::test_pool().await;
        let (_user_id, raw) = seed_user_with_refresh_token(&pool).await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(refresh),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/auth/refresh")
            .insert_header(("Cookie", format!("refresh_token={raw}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let set_cookie = resp
            .headers()
            .get("Set-Cookie")
            .expect("Set-Cookie header missing")
            .to_str()
            .unwrap();
        assert!(
            set_cookie.contains("auth_token="),
            "auth_token cookie not set"
        );
    }

    #[tokio::test]
    async fn missing_refresh_token_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool)))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(refresh),
        )
        .await;

        let req = test::TestRequest::post().uri("/auth/refresh").to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn used_refresh_token_returns_401_and_kills_family() {
        let pool = crate::test_helpers::test_pool().await;
        let (user_id, raw) = seed_user_with_refresh_token(&pool).await;
        let mapper = RefreshTokenMapper::from_pool(pool.clone());

        // Rotate the original token so it becomes "used" and a second active token exists.
        let h = hash_refresh_token(&raw);
        let row = mapper.find_valid_token(&h).await.unwrap();
        let n: u64 = rand::random();
        let new_raw = format!("new_token_{n}");
        let new_hash = hash_refresh_token(&new_raw);
        mapper
            .rotate_token(row.id, row.user_id, &new_hash)
            .await
            .unwrap();

        // Confirm the second (active) token exists before the replay.
        let active_before = mapper.find_valid_token(&new_hash).await;
        assert!(
            active_before.is_ok(),
            "active token must exist before replay"
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(RefreshTokenMapper::from_pool(pool.clone())))
                .app_data(jwt_data())
                .app_data(cookie_data())
                .service(refresh),
        )
        .await;

        // Replay the original (now-used) token.
        let req = test::TestRequest::post()
            .uri("/auth/refresh")
            .insert_header(("Cookie", format!("refresh_token={raw}")))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );

        // The entire token family must be wiped — the active second token is gone too.
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count, 0,
            "all tokens for the user must be invalidated on replay"
        );
    }
}
