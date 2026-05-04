#![allow(clippy::cast_possible_truncation)]

use crate::{
    cache::{CACHE_TTL_CALENDAR, CACHE_TTL_ITEM, CACHE_TTL_SEARCH, Cache},
    entity::calendar::{Calendar as CalendarEntity, Language as LanguageEntity},
    error::Error,
    mappers::{calendar::CalendarMapper, user::UserMapper},
    middleware::auth::Claims,
    services::{cached_anilist::CachedAnilist, ics_export::IcsExportService},
};

use actix_web::{HttpResponse, ResponseError, delete, get, put, web};
use chrono::{NaiveDateTime, Utc};
use common::{
    calendar::Calendar,
    id::Id,
    item::{AnimeDataSource, Item},
    language::Language,
    schedule::Schedule,
};
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Count how many of `item_ids` have at least one airing schedule entry strictly after `now_secs`.
///
/// Items missing from `schedules_by_id` (e.g. an Anilist cache miss) are not counted.
#[must_use]
fn count_airing(
    item_ids: &[Id],
    schedules_by_id: &HashMap<u64, &[Schedule]>,
    now_secs: u64,
) -> usize {
    item_ids
        .iter()
        .filter(|id| {
            schedules_by_id
                .get(&id.to_int())
                .is_some_and(|schedules| schedules.iter().any(|s| s.airing_at.to_int() > now_secs))
        })
        .count()
}

#[cfg(test)]
mod airing_count_tests {
    use super::*;
    use common::timestamp::Timestamp;

    fn id(n: i64) -> Id {
        Id::new(n).unwrap()
    }

    fn schedule_at(secs: i64) -> Schedule {
        Schedule {
            id: id(secs.max(1)),
            airing_at: Timestamp::new(secs).unwrap(),
            episode: 1,
            media_id: None,
        }
    }

    #[test]
    fn empty_calendar_yields_zero() {
        let map: HashMap<u64, &[Schedule]> = HashMap::new();
        assert_eq!(count_airing(&[], &map, 1_000), 0);
    }

    #[test]
    fn item_with_only_past_schedules_is_not_counted() {
        let schedules = vec![schedule_at(500), schedule_at(900)];
        let map: HashMap<u64, &[Schedule]> = HashMap::from([(1, schedules.as_slice())]);
        assert_eq!(count_airing(&[id(1)], &map, 1_000), 0);
    }

    #[test]
    fn item_with_a_future_schedule_is_counted() {
        let schedules = vec![schedule_at(500), schedule_at(2_000)];
        let map: HashMap<u64, &[Schedule]> = HashMap::from([(1, schedules.as_slice())]);
        assert_eq!(count_airing(&[id(1)], &map, 1_000), 1);
    }

    #[test]
    fn mixed_calendar_counts_only_airing_items() {
        let airing_a = vec![schedule_at(2_000)];
        let finished_b = vec![schedule_at(100), schedule_at(500)];
        let airing_c = vec![schedule_at(3_000)];
        let map: HashMap<u64, &[Schedule]> = HashMap::from([
            (1, airing_a.as_slice()),
            (2, finished_b.as_slice()),
            (3, airing_c.as_slice()),
        ]);
        assert_eq!(count_airing(&[id(1), id(2), id(3)], &map, 1_000), 2);
    }

    #[test]
    fn item_missing_from_anilist_cache_is_not_counted() {
        let map: HashMap<u64, &[Schedule]> = HashMap::new();
        assert_eq!(count_airing(&[id(42)], &map, 1_000), 0);
    }

    #[test]
    fn schedule_exactly_at_now_is_not_counted_as_future() {
        let schedules = vec![schedule_at(1_000)];
        let map: HashMap<u64, &[Schedule]> = HashMap::from([(1, schedules.as_slice())]);
        assert_eq!(count_airing(&[id(1)], &map, 1_000), 0);
    }
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct CalendarRequest {
    #[serde(default)]
    #[schema(value_type = i64)]
    pub id: Id,
    pub items: Vec<Item>,
    pub language: Language,
    pub name: String,
}

impl CalendarRequest {
    /// Validates that the calendar name meets character limits
    /// # Errors
    /// Returns Ok(()) if valid, Err with error message if invalid
    pub fn validate_name(&self) -> Result<(), String> {
        // Set character limit for calendar names
        const MAX_NAME_LENGTH: usize = 100;

        if self.name.is_empty() {
            return Err("Calendar name cannot be empty".to_string());
        }

        if self.name.chars().count() > MAX_NAME_LENGTH {
            return Err(format!(
                "Calendar name must be {MAX_NAME_LENGTH} characters or less"
            ));
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, utoipa::ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

const fn default_page() -> usize {
    1
}

const fn default_page_size() -> usize {
    6
}

#[utoipa::path(
    get,
    path = "/calendars/{id}/export",
    tag = "calendars",
    params(("id" = u64, Path, description = "Calendar ID")),
    responses(
        (status = 200, description = "iCalendar file", content_type = "text/calendar"),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 404, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/calendars/{id}/export")]
async fn export(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    ics_export: web::Data<IcsExportService>,
    cache: web::Data<Cache>,
    id: web::Path<u64>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Ownership check stays in the handler — IcsExportService::render
    // intentionally omits owner enforcement (it's also used by the public
    // subscribe_feed and by FrozenIcsService).
    let calendar_data = match calendar_mapper
        .get_calendar_by_id(*id as i32, user.id)
        .await
    {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };

    let calendar_id = calendar_data.id;
    let filename = format!(
        "{}.ics",
        calendar_data.name.replace(' ', "_").to_lowercase()
    );

    let cache_key = crate::cache::generate_export_key(calendar_id);
    let cache_ttl = CACHE_TTL_CALENDAR;

    // Cache hit short-circuit. Cache errors are non-fatal (we still render
    // fresh below).
    if let Ok(Some(cached)) = cache.get::<String>(&cache_key).await {
        return HttpResponse::Ok()
            .append_header(("Content-Type", "text/calendar"))
            .append_header((
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ))
            .body(cached);
    }

    let body = match ics_export.render(calendar_id).await {
        Ok(s) => s,
        Err(err) => return err.error_response(),
    };

    if let Err(e) = cache.set(&cache_key, &body, cache_ttl).await {
        error!("export: failed to write cache: {e:?}");
    }

    HttpResponse::Ok()
        .append_header(("Content-Type", "text/calendar"))
        .append_header((
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        ))
        .body(body)
}

#[utoipa::path(
    get,
    path = "/calendars/subscribe/{token}",
    tag = "calendars",
    params(("token" = String, Path, description = "Calendar subscription token")),
    responses(
        (status = 200, description = "iCalendar subscription feed", content_type = "text/calendar"),
        (status = 404, body = crate::controllers::auth::ErrorResponse),
    )
)]
#[get("/calendars/subscribe/{token}")]
async fn subscribe_feed(
    calendar_mapper: web::Data<CalendarMapper>,
    ics_export: web::Data<IcsExportService>,
    cache: web::Data<Cache>,
    token: web::Path<String>,
) -> HttpResponse {
    let calendar_data = match calendar_mapper.get_calendar_by_token(&token).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    let calendar_id = calendar_data.id;

    let cache_key = format!("subscribe:{}", token.as_str());
    let cache_ttl = CACHE_TTL_ITEM;

    if let Ok(Some(cached)) = cache.get::<String>(&cache_key).await {
        return HttpResponse::Ok()
            .append_header(("Content-Type", "text/calendar; charset=utf-8"))
            .body(cached);
    }

    let body = match ics_export.render(calendar_id).await {
        Ok(s) => s,
        Err(err) => return err.error_response(),
    };

    if let Err(e) = cache.set(&cache_key, &body, cache_ttl).await {
        error!("subscribe_feed: failed to write cache: {e:?}");
    }

    HttpResponse::Ok()
        .append_header(("Content-Type", "text/calendar; charset=utf-8"))
        .body(body)
}

#[utoipa::path(
    put,
    path = "/calendar",
    tag = "calendars",
    request_body = CalendarRequest,
    responses(
        (status = 200, body = common::calendar::Calendar),
        (status = 400, body = crate::controllers::auth::ErrorResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 404, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[put("/calendar")]
async fn put(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    anilist: web::Data<CachedAnilist>,
    cache: web::Data<Cache>,
    body: web::Json<CalendarRequest>,
    claims: Claims,
) -> HttpResponse {
    const MAX_ITEMS: usize = 2000;

    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Validate calendar name before proceeding
    if let Err(validation_error) = body.validate_name() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": validation_error
        }));
    }

    if body.items.len() > MAX_ITEMS {
        return Error::InvalidRequest.error_response();
    }

    let item_ids: Vec<i32> = body
        .items
        .iter()
        .map(|item| item.id.to_int() as i32)
        .collect();

    let calendar_entity = CalendarEntity {
        id: body.id.to_int() as i32,
        item_ids,
        language: LanguageEntity::from_common_language(&body.language),
        name: body.name.clone(),
        subscription_token: String::new(),
        user_id: user.id,
        created_at: NaiveDateTime::default(),
        updated_at: NaiveDateTime::default(),
        event_style: "timed".to_owned(),
        frozen_subscribe_ics: None,
    };

    // Create calendar with the authenticated user's ID
    match calendar_mapper.save_calendar(calendar_entity).await {
        Ok(calendar) => {
            let item_ids: Vec<Id> = calendar
                .item_ids
                .iter()
                .filter_map(|id| Id::new(i64::from(*id)))
                .collect();

            let items = anilist.get_items(item_ids).await;

            if items.is_empty() {
                return Error::NotFound.error_response();
            }

            if let Some(id) = Id::new(calendar.id.into()) {
                // Invalidate cache for this calendar (controller-level invalidation)
                let _ = cache.invalidate_calendar(calendar.id).await;
                let _ = cache.invalidate_user_paged_calendars(user.id).await;
                let _ = cache
                    .invalidate_subscription(&calendar.subscription_token)
                    .await;

                HttpResponse::Ok().json(Calendar {
                    id,
                    items,
                    language: calendar.language.to_common_language(),
                    name: calendar.name,
                    created_at: calendar.created_at,
                    updated_at: calendar.updated_at,
                })
            } else {
                Error::NotFound.error_response()
            }
        }
        Err(e) => e.error_response(),
    }
}

#[derive(Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct PageCalendar {
    #[schema(value_type = i64)]
    pub id: Id,
    pub item_count: usize,
    pub airing_count: usize,
    pub name: String,
    pub subscription_token: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub recent_item_ids: Vec<i32>,
}

#[derive(Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct PaginatedResponse {
    data: Vec<PageCalendar>,
    pagination: PaginationInfo,
}

#[derive(Serialize, Deserialize, Clone, utoipa::ToSchema)]
pub struct PaginationInfo {
    page: usize,
    page_size: usize,
    total: usize,
    total_pages: usize,
}

#[utoipa::path(
    get,
    path = "/calendars",
    tag = "calendars",
    params(PaginationParams),
    responses(
        (status = 200, body = PaginatedResponse),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/calendars")]
async fn get_calendars(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    cache: web::Data<Cache>,
    anilist: web::Data<CachedAnilist>,
    claims: Claims,
    params: web::Query<PaginationParams>,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Get calendars for the authenticated user with pagination
    match calendar_mapper
        .get_calendars_by_user_paginated(user.id, params.page, params.page_size)
        .await
    {
        Ok((calendars, total_count)) => {
            // Collect a deduped list of item ids across all calendars so we can fetch
            // their Anilist metadata in a single batched call. We then derive each
            // calendar's `airing_count` from the cached metadata in-memory — see
            // `count_airing` for the rule (any future-dated airing schedule entry).
            let mut unique_ids: Vec<Id> = Vec::new();
            let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
            for (calendar, _) in &calendars {
                for raw_id in &calendar.item_ids {
                    if let Some(id) = Id::new(i64::from(*raw_id))
                        && seen.insert(id.to_int())
                    {
                        unique_ids.push(id);
                    }
                }
            }

            let items: Vec<Item> = if unique_ids.is_empty() {
                Vec::new()
            } else {
                anilist.get_items(unique_ids).await
            };
            let schedules_by_id: HashMap<u64, &[Schedule]> = items
                .iter()
                .map(|item| (item.id.to_int(), item.airing_schedule.as_slice()))
                .collect();

            #[allow(clippy::cast_sign_loss)]
            let now_secs = Utc::now().timestamp().max(0) as u64;

            let mut results: Vec<PageCalendar> = Vec::new();

            for (calendar, recent_item_ids) in calendars {
                if let Some(id) = Id::new(calendar.id.into()) {
                    let calendar_item_ids: Vec<Id> = calendar
                        .item_ids
                        .iter()
                        .filter_map(|raw| Id::new(i64::from(*raw)))
                        .collect();
                    let airing_count = count_airing(&calendar_item_ids, &schedules_by_id, now_secs);

                    results.push(PageCalendar {
                        id,
                        item_count: calendar.item_ids.len(),
                        airing_count,
                        name: calendar.name,
                        subscription_token: calendar.subscription_token,
                        created_at: calendar.created_at,
                        updated_at: calendar.updated_at,
                        recent_item_ids,
                    });
                }
            }

            // Return paginated response with metadata
            let total_pages = total_count.div_ceil(params.page_size);
            let paginated_response = PaginatedResponse {
                data: results,
                pagination: PaginationInfo {
                    page: params.page,
                    page_size: params.page_size,
                    total: total_count,
                    total_pages,
                },
            };

            // Cache the response for 30 minutes (1800 seconds)
            let cache_key = crate::cache::generate_paginated_key(
                user.id,
                "calendars",
                params.page,
                params.page_size,
            );
            let cache_ttl = CACHE_TTL_SEARCH;

            // If cache is available, try to get from cache
            match cache
                .cached_response(&cache_key, cache_ttl, || async {
                    Ok(paginated_response.clone())
                })
                .await
            {
                Ok(cached_response) => {
                    return HttpResponse::Ok().json(cached_response);
                }
                Err(e) => {
                    // Log error but continue with regular processing
                    error!("Cache error: {e:?}");
                }
            }

            // If no cache or cache error, return the response normally
            HttpResponse::Ok().json(paginated_response)
        }
        Err(e) => e.error_response(),
    }
}

#[utoipa::path(
    get,
    path = "/calendars/{id}",
    tag = "calendars",
    params(("id" = i64, Path, description = "Calendar ID")),
    responses(
        (status = 200, body = common::calendar::Calendar),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 404, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[get("/calendars/{id}")]
async fn get_calendar(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    anilist: web::Data<CachedAnilist>,
    cache: web::Data<Cache>,
    id: web::Path<i64>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Check if the calendar belongs to the authenticated user
    match calendar_mapper
        .get_calendar_by_id(*id as i32, user.id)
        .await
    {
        Ok(calendar) => {
            let item_ids: Vec<Id> = calendar
                .item_ids
                .iter()
                .filter_map(|id| Id::new(i64::from(*id)))
                .collect();

            let items = anilist.get_items(item_ids).await;

            if items.is_empty() {
                return Error::NotFound.error_response();
            }

            if let Some(id) = Id::new(calendar.id.into()) {
                let calendar_response = Calendar {
                    id: id.clone(),
                    items,
                    language: calendar.language.to_common_language(),
                    name: calendar.name,
                    created_at: calendar.created_at,
                    updated_at: calendar.updated_at,
                };

                // Cache the response for 1 hour (3600 seconds)
                let cache_key = crate::cache::generate_calendar_key(id.to_int() as i32);
                let cache_ttl = CACHE_TTL_ITEM;

                // If cache is available, try to get from cache
                match cache
                    .cached_response(&cache_key, cache_ttl, || async {
                        Ok(calendar_response.clone())
                    })
                    .await
                {
                    Ok(cached_response) => {
                        return HttpResponse::Ok().json(cached_response);
                    }
                    Err(e) => {
                        // Log error but continue with regular processing
                        error!("Cache error: {e:?}");
                    }
                }

                // If no cache or cache error, return the response normally
                HttpResponse::Ok().json(calendar_response)
            } else {
                Error::NotFound.error_response()
            }
        }
        Err(e) => e.error_response(),
    }
}

#[utoipa::path(
    delete,
    path = "/calendars/{id}",
    tag = "calendars",
    params(("id" = i64, Path, description = "Calendar ID")),
    responses(
        (status = 200, description = "Calendar deleted"),
        (status = 401, body = crate::controllers::auth::ErrorResponse),
        (status = 404, body = crate::controllers::auth::ErrorResponse),
    ),
    security(("bearer_auth" = []))
)]
#[delete("/calendars/{id}")]
async fn delete_calendar(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    cache: web::Data<Cache>,
    id: web::Path<i64>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    match calendar_mapper.delete_calendar(*id as i32, user.id).await {
        Ok(subscription_token) => {
            let _ = cache.invalidate_calendar(*id as i32).await;
            let _ = cache.invalidate_user_paged_calendars(user.id).await;
            let _ = cache.invalidate_subscription(&subscription_token).await;
            HttpResponse::Ok().finish()
        }
        Err(e) => e.error_response(),
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::cache::Cache;
    use crate::config::server::{CacheConfig, JwtSecret};
    use crate::mappers::anilist::Anilist;
    use crate::mappers::calendar::CalendarMapper;
    use crate::mappers::user::UserMapper;
    use crate::services::auth::hash_password;
    use crate::services::cached_anilist::CachedAnilist;
    use actix_web::{App, http::StatusCode, test, web};
    use secrecy::SecretString;
    use serde_json::Value;
    use sqlx::Row;

    const SECRET: &str = "test-jwt-secret-at-least-32-bytes";

    fn jwt_data() -> web::Data<JwtSecret> {
        web::Data::new(JwtSecret::new(SecretString::from(SECRET)))
    }

    async fn cached_anilist_data() -> web::Data<CachedAnilist> {
        web::Data::new(CachedAnilist::new(
            Anilist::default(),
            Cache::for_tests().await,
            &CacheConfig::default(),
        ))
    }

    struct SeedUser {
        id: i32,
        token: String,
    }

    async fn seed_user(pool: &sqlx::PgPool) -> SeedUser {
        use crate::services::auth::generate_token;
        let n: u64 = rand::random();
        let hash = hash_password("SecurePass12!@").unwrap();
        let user = UserMapper::from_pool(pool.clone())
            .create_user(
                &format!("caltest_{n}"),
                &format!("caltest_{n}@test.com"),
                Some(&hash),
            )
            .await
            .unwrap();
        let token = generate_token(&user.id, SECRET).unwrap();
        SeedUser { id: user.id, token }
    }

    /// Insert a bare calendar row directly (no items), returning its id and subscription token.
    async fn seed_calendar(pool: &sqlx::PgPool, user_id: i32, name: &str) -> (i32, String) {
        let row = sqlx::query(
            "INSERT INTO calendars (name, language, user_id, subscription_token) \
             VALUES ($1, 'english'::language, $2, encode(gen_random_bytes(32), 'hex')) \
             RETURNING id, subscription_token",
        )
        .bind(name)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();
        let id: i32 = row.try_get("id").unwrap();
        let token: String = row.try_get("subscription_token").unwrap();
        (id, token)
    }

    // ─── PUT /calendar (validation / auth) ───────────────────────────────────

    #[tokio::test]
    async fn put_calendar_without_token_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(cached_anilist_data().await)
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(put),
        )
        .await;
        let req = test::TestRequest::put()
            .uri("/calendar")
            .set_json(serde_json::json!({ "id": 0, "name": "My Cal", "language": "english", "items": [] }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn put_calendar_empty_name_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(cached_anilist_data().await)
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(put),
        )
        .await;
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(
                serde_json::json!({ "id": 0, "name": "", "language": "english", "items": [] }),
            )
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn put_calendar_name_too_long_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(cached_anilist_data().await)
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(put),
        )
        .await;
        let long_name = "a".repeat(101);
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({ "id": 0, "name": long_name, "language": "english", "items": [] }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn put_calendar_too_many_items_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(cached_anilist_data().await)
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(put),
        )
        .await;
        // Build a vec of 2001 minimal items — one over the cap.
        let items: Vec<serde_json::Value> = (0..2001)
            .map(|i| serde_json::json!({ "id": i, "title": "x", "episodes": [] }))
            .collect();
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "id": 0,
                "name": "Big",
                "language": "english",
                "items": items
            }))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::BAD_REQUEST
        );
    }

    // ─── GET /calendars ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_calendars_returns_paginated_list() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        seed_calendar(&pool, user.id, "Cal A").await;
        seed_calendar(&pool, user.id, "Cal B").await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(cached_anilist_data().await)
                .app_data(jwt_data())
                .service(get_calendars),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/calendars?page=1&page_size=10")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["pagination"]["total"], 2);
        assert_eq!(body["data"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_calendars_without_token_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(cached_anilist_data().await)
                .app_data(jwt_data())
                .service(get_calendars),
        )
        .await;
        let req = test::TestRequest::get().uri("/calendars").to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn get_calendars_only_returns_own_calendars() {
        let pool = crate::test_helpers::test_pool().await;
        let user_a = seed_user(&pool).await;
        let user_b = seed_user(&pool).await;
        seed_calendar(&pool, user_a.id, "Dave's Cal").await;
        seed_calendar(&pool, user_b.id, "Eve's Cal").await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(cached_anilist_data().await)
                .app_data(jwt_data())
                .service(get_calendars),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/calendars")
            .insert_header(("Cookie", format!("auth_token={}", user_a.token)))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let body: Value = test::read_body_json(resp).await;
        // user_a should only see their own calendar
        assert_eq!(body["pagination"]["total"], 1);
        let names: Vec<&str> = body["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"Dave's Cal"));
        assert!(!names.contains(&"Eve's Cal"));
    }

    // ─── GET /calendars/{id} ─────────────────────────────────────────────────

    #[tokio::test]
    async fn get_calendar_without_token_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(cached_anilist_data().await)
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(get_calendar),
        )
        .await;
        let req = test::TestRequest::get().uri("/calendars/1").to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn get_calendar_not_found_returns_404() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(cached_anilist_data().await)
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(get_calendar),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/calendars/999999")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::NOT_FOUND
        );
    }

    // ─── DELETE /calendars/{id} ───────────────────────────────────────────────

    #[tokio::test]
    async fn delete_calendar_own_returns_200() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let (cal_id, _) = seed_calendar(&pool, user.id, "To Delete").await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(delete_calendar),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri(&format!("/calendars/{cal_id}"))
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn delete_calendar_non_existent_returns_404() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(delete_calendar),
        )
        .await;
        let req = test::TestRequest::delete()
            .uri("/calendars/999999")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn delete_calendar_without_token_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(jwt_data())
                .service(delete_calendar),
        )
        .await;
        let req = test::TestRequest::delete().uri("/calendars/1").to_request();
        assert_eq!(
            test::call_service(&app, req).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }
}
