//! Read-side endpoint(s) for account-level state that the SPA needs but
//! doesn't fit into `/user/details` or `/user/settings`.
//!
//! Currently exposes `/account/usage` — the per-user counts driving free-tier
//! cap UX (calendar counter chip, show counter, 80% banner, disabled-add).
//! The endpoint is deliberately separate from `/user/settings`: counts change
//! on every add/remove, while settings are fetched on auth and cached.
//! Coupling them would force settings refetches on every editor edit.

use crate::{
    mappers::user::UserMapper, middleware::auth::Claims, services::show_count::ShowCountService,
};
use actix_web::{HttpResponse, ResponseError, get, web};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct UsageResponse {
    /// Distinct `AniList` media ids tracked across the user's non-deleted
    /// calendars. Backs the show-cap meter.
    pub shows: i64,
    /// Non-deleted calendars owned by the user. Backs the calendar-cap meter.
    pub calendars: i64,
}

#[utoipa::path(
    get,
    path = "/account/usage",
    tag = "user",
    operation_id = "get_account_usage",
    responses(
        (status = 200, body = UsageResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/account/usage")]
#[allow(clippy::future_not_send)]
pub async fn get_usage(
    user_mapper: web::Data<UserMapper>,
    show_count: web::Data<ShowCountService>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(u) => u,
        Err(e) => return e.error_response(),
    };

    let shows = match show_count.count_distinct_for_user(user.id).await {
        Ok(n) => n,
        Err(e) => {
            log::error!("count_distinct_for_user failed: {e:?}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let calendars = match show_count.count_calendars_for_user(user.id).await {
        Ok(n) => n,
        Err(e) => {
            log::error!("count_calendars_for_user failed: {e:?}");
            return HttpResponse::InternalServerError().finish();
        }
    };

    HttpResponse::Ok().json(UsageResponse { shows, calendars })
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::config::server::JwtSecret;
    use crate::entity::calendar::Calendar;
    use crate::mappers::calendar::CalendarMapper;
    use crate::mappers::user::UserMapper;
    use crate::services::auth::generate_token;
    use actix_web::{App, http::StatusCode, test, web};
    use secrecy::SecretString;
    use serde_json::Value;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    async fn seed_user(pool: &sqlx::PgPool) -> (i32, String) {
        let n: u64 = rand::random();
        let username = format!("usagetest_{n}");
        let email = format!("usagetest_{n}@test.com");
        let user = UserMapper::from_pool(pool.clone())
            .create_user(&username, &email, None)
            .await
            .unwrap();
        let token = generate_token(&user.id, SECRET).unwrap();
        (user.id, token)
    }

    async fn insert_calendar(pool: &sqlx::PgPool, user_id: i32, items: Vec<i32>) {
        CalendarMapper::from_pool(pool.clone())
            .insert_calendar(Calendar {
                user_id,
                item_ids: items,
                ..Calendar::default()
            })
            .await
            .unwrap();
    }

    async fn cleanup_user(pool: &sqlx::PgPool, user_id: i32) {
        sqlx::query(
            "DELETE FROM calendar_items WHERE calendar_id IN (SELECT id FROM calendars WHERE user_id = $1)",
        )
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("DELETE FROM calendars WHERE user_id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn returns_zero_counts_for_fresh_user() {
        let pool = crate::test_helpers::test_pool().await;
        let (user_id, token) = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(ShowCountService::new(pool.clone())))
                .app_data(jwt_data())
                .service(get_usage),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/account/usage")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["shows"], 0);
        assert_eq!(body["calendars"], 0);
        cleanup_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn returns_distinct_show_count_and_calendar_count() {
        let pool = crate::test_helpers::test_pool().await;
        let (user_id, token) = seed_user(&pool).await;
        // 2 calendars; 5 distinct items total (one duplicate across calendars)
        insert_calendar(&pool, user_id, vec![100, 200, 300]).await;
        insert_calendar(&pool, user_id, vec![300, 400, 500]).await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(ShowCountService::new(pool.clone())))
                .app_data(jwt_data())
                .service(get_usage),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/account/usage")
            .insert_header(("Cookie", format!("auth_token={token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["calendars"], 2);
        assert_eq!(body["shows"], 5);

        cleanup_user(&pool, user_id).await;
    }

    #[tokio::test]
    async fn returns_401_without_token() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(ShowCountService::new(pool.clone())))
                .app_data(jwt_data())
                .service(get_usage),
        )
        .await;
        let req = test::TestRequest::get().uri("/account/usage").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
