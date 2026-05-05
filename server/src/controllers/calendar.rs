#![allow(clippy::cast_possible_truncation)]

use crate::{
    cache::{CACHE_TTL_CALENDAR, CACHE_TTL_ITEM, CACHE_TTL_SEARCH, Cache},
    entity::{
        calendar::{Calendar as CalendarEntity, Language as LanguageEntity},
        subscription::Tier,
    },
    error::Error,
    mappers::{calendar::CalendarMapper, subscription::SubscriptionMapper, user::UserMapper},
    middleware::auth::Claims,
    services::{
        cached_anilist::CachedAnilist, entitlement::EntitlementService,
        frozen_ics::FrozenIcsService, ics_export::IcsExportService,
        sharing_authz::{Action, SharingAuthz},
    },
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

/// Accepted values for `CalendarRequest::event_style`. Mirrors the
/// `calendars.event_style` column shape — `"timed"` (DTSTART:datetime) or
/// `"all_day"` (DTSTART;VALUE=DATE). Everything else returns 400 with
/// `{"error":"event_style_invalid"}`.
const VALID_EVENT_STYLES: &[&str] = &["timed", "all_day"];

fn default_event_style() -> String {
    "timed".to_owned()
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct CalendarRequest {
    #[serde(default)]
    #[schema(value_type = i64)]
    pub id: Id,
    pub items: Vec<Item>,
    pub language: Language,
    pub name: String,
    /// `"timed"` (default) or `"all_day"`. Drives DTSTART format in the
    /// rendered .ics. Validated on the server; unknown values → 400.
    #[serde(default = "default_event_style")]
    pub event_style: String,
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
    frozen_ics: web::Data<FrozenIcsService>,
    cache: web::Data<Cache>,
    entitlement: web::Data<EntitlementService>,
    subscription_mapper: web::Data<SubscriptionMapper>,
    token: web::Path<String>,
) -> HttpResponse {
    let calendar_data = match calendar_mapper.get_calendar_by_token(&token).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    let calendar_id = calendar_data.id;
    let owner_id = calendar_data.user_id;

    let owner_tier = match entitlement.effective_tier(owner_id).await {
        Ok(t) => t,
        Err(e) => return e.error_response(),
    };

    // Paid: live render with controller-level cache.
    if matches!(owner_tier, Tier::Paid) {
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

        return HttpResponse::Ok()
            .append_header(("Content-Type", "text/calendar; charset=utf-8"))
            .body(body);
    }

    // Free: serve the frozen blob if present (the blob IS the cache — no
    // controller-level cache layer). If null, decide between 404 (never
    // had Pro) and lazy regen (had Pro before; blob was never written).
    if let Some(blob) = calendar_data.frozen_subscribe_ics.clone() {
        return HttpResponse::Ok()
            .append_header(("Content-Type", "text/calendar; charset=utf-8"))
            .body(blob);
    }

    let has_history = match subscription_mapper
        .find_latest_customer_id_for_user(owner_id)
        .await
    {
        Ok(opt) => opt.is_some(),
        Err(e) => return e.error_response(),
    };

    if !has_history {
        return Error::NotFound.error_response();
    }

    log::warn!(
        "subscribe_feed: frozen_subscribe_ics null for previously-paid user {owner_id} on calendar {calendar_id}; lazy regenerating",
    );
    if let Err(e) = frozen_ics.regenerate(calendar_id).await {
        return e.error_response();
    }
    let body = match ics_export.render(calendar_id).await {
        Ok(s) => s,
        Err(e) => return e.error_response(),
    };
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
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn put(
    user_mapper: web::Data<UserMapper>,
    anilist: web::Data<CachedAnilist>,
    cache: web::Data<Cache>,
    entitlement: web::Data<EntitlementService>,
    frozen_ics: web::Data<FrozenIcsService>,
    pool: web::Data<sqlx::PgPool>,
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

    if !VALID_EVENT_STYLES.contains(&body.event_style.as_str()) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "event_style_invalid"}));
    }

    if body.items.len() > MAX_ITEMS {
        return Error::InvalidRequest.error_response();
    }

    let item_ids: Vec<i32> = body
        .items
        .iter()
        .map(|item| item.id.to_int() as i32)
        .collect();

    let is_create = body.id.to_int() == 0;

    // Fast-path tier check (outside lock). Best-effort: short-circuits
    // obviously-blocked requests cheaply. The authoritative re-check runs
    // inside the advisory-locked transaction below.
    if is_create
        && let Err(e) = entitlement.assert_can_create_calendar(user.id).await
    {
        return e.error_response();
    }
    for &item_id in &item_ids {
        if let Err(e) = entitlement.assert_can_add_show(user.id, item_id).await {
            return e.error_response();
        }
    }

    let calendar_entity = CalendarEntity {
        id: body.id.to_int() as i32,
        item_ids,
        language: LanguageEntity::from_common_language(&body.language),
        name: body.name.clone(),
        subscription_token: String::new(),
        user_id: user.id,
        created_at: NaiveDateTime::default(),
        updated_at: NaiveDateTime::default(),
        event_style: body.event_style.clone(),
        frozen_subscribe_ics: None,
        // Discarded by the UPDATE (meta_version column is computed server-side
        // via CASE WHEN in the mapper); the request body never sets it.
        meta_version: 0,
    };

    // Acquire transaction + per-user advisory lock. Anything inside the lock
    // serialises against concurrent PUTs from the same user (multi-tab race).
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return Error::Database(e).error_response(),
    };
    if let Err(e) = sqlx::query!("SELECT pg_advisory_xact_lock($1)", i64::from(user.id))
        .execute(&mut *tx)
        .await
    {
        return Error::Database(e).error_response();
    }

    // Authoritative cap re-check inside the lock. Pool-bound checks would
    // observe pre-INSERT state from concurrent transactions and bypass the
    // lock; the in-tx variants run on the locked connection.
    if is_create
        && let Err(e) = entitlement
            .assert_can_create_calendar_in_tx(&mut tx, user.id)
            .await
    {
        return e.error_response();
    }
    for item_id in &calendar_entity.item_ids {
        if let Err(e) = entitlement
            .assert_can_add_show_in_tx(&mut tx, user.id, *item_id)
            .await
        {
            return e.error_response();
        }
    }

    // Update path: route through SharingAuthz so editor support drops in
    // cleanly in Phase 2. Today the load itself filters by owner, so a
    // non-owner gets NotFound from the load — assert_can_in_tx is the
    // forward-compatible spine, not the gate.
    if !is_create {
        let existing = match CalendarMapper::get_calendar_by_id_with(
            &mut tx,
            calendar_entity.id,
            user.id,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => return e.error_response(),
        };
        if let Err(e) =
            SharingAuthz::assert_can_in_tx(&mut tx, user.id, &existing, Action::MetaMutate).await
        {
            return e.error_response();
        }
    }

    // Save calendar via _with helpers so the write participates in the
    // locked transaction.
    let calendar = match CalendarMapper::save_calendar_with(&mut tx, calendar_entity).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };

    if let Err(e) = tx.commit().await {
        return Error::Database(e).error_response();
    }

    // Best-effort post-commit refresh of the frozen subscribe blob for Free
    // owners. Failures here are non-fatal: subscribe_feed has a lazy-regen
    // safety net for null blobs on previously-paid users.
    let owner_tier = entitlement
        .effective_tier(user.id)
        .await
        .unwrap_or(Tier::Free);
    if matches!(owner_tier, Tier::Free)
        && let Err(e) = frozen_ics.regenerate(calendar.id).await
    {
        error!(
            "put: failed to regenerate frozen_subscribe_ics for calendar {}: {e:?}",
            calendar.id
        );
    }

    let response_item_ids: Vec<Id> = calendar
        .item_ids
        .iter()
        .filter_map(|id| Id::new(i64::from(*id)))
        .collect();

    let items = anilist.get_items(response_item_ids).await;

    if items.is_empty() {
        return Error::NotFound.error_response();
    }

    if let Some(id) = Id::new(calendar.id.into()) {
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
    sharing_authz: web::Data<SharingAuthz>,
    id: web::Path<i64>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Route through SharingAuthz so editor support drops in cleanly in
    // Phase 2. The lookup itself still filters by owner for now —
    // assert_can is the spine, not the gate.
    let existing = match calendar_mapper.get_calendar_by_id(*id as i32, user.id).await {
        Ok(c) => c,
        Err(e) => return e.error_response(),
    };
    if let Err(e) = sharing_authz
        .assert_can(user.id, &existing, Action::ManageEditors)
        .await
    {
        return e.error_response();
    }

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

    use crate::config::server::LimitsConfig;
    use crate::mappers::calendar_editor::CalendarEditorMapper;
    use crate::mappers::subscription::SubscriptionMapper;
    use crate::services::entitlement::EntitlementService;
    use crate::services::frozen_ics::FrozenIcsService;
    use crate::services::ics_export::IcsExportService;
    use crate::services::sharing_authz::SharingAuthz;
    use crate::services::show_count::ShowCountService;

    fn limits(free_calendar_limit: u32, free_show_cap: u32) -> LimitsConfig {
        LimitsConfig {
            free_calendar_limit,
            free_show_cap,
            pro_max_reminders: 5,
        }
    }

    /// Build the App configured for PUT-handler tests with the supplied
    /// limits. Returned as a closure-callable inline via `init_service` so
    /// tests can keep the concrete `actix_web::test` service type.
    async fn build_put_services(
        pool: sqlx::PgPool,
        l: &LimitsConfig,
    ) -> (
        web::Data<UserMapper>,
        web::Data<CalendarMapper>,
        web::Data<CachedAnilist>,
        web::Data<Cache>,
        web::Data<EntitlementService>,
        web::Data<FrozenIcsService>,
        web::Data<sqlx::PgPool>,
        web::Data<SharingAuthz>,
    ) {
        let cached = cached_anilist_data().await;
        let entitlement = EntitlementService::new(
            SubscriptionMapper::from_pool(pool.clone()),
            ShowCountService::new(pool.clone()),
            l,
        );
        let ics_export = IcsExportService::new(
            pool.clone(),
            (**cached).clone(),
            crate::mappers::user_settings::UserSettingsMapper::from_pool(pool.clone()),
            entitlement.clone(),
        );
        let frozen_ics = FrozenIcsService::new(pool.clone(), ics_export);
        let sharing_authz = SharingAuthz::new(
            CalendarEditorMapper::from_pool(pool.clone()),
            entitlement.clone(),
        );
        (
            web::Data::new(UserMapper::from_pool(pool.clone())),
            web::Data::new(CalendarMapper::from_pool(pool.clone())),
            cached,
            web::Data::new(Cache::for_tests().await),
            web::Data::new(entitlement),
            web::Data::new(frozen_ics),
            web::Data::new(pool),
            web::Data::new(sharing_authz),
        )
    }

    /// Build a JSON Item payload that satisfies `common::item::Item`'s serde
    /// deserialiser. Required because Item has many nested fields and tests
    /// would otherwise hit a 400 from JSON deserialisation rather than the
    /// 402 path under test.
    fn item_json(id: i64) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "id_mal": null,
            "title": { "english": "x", "native": "x", "romaji": "x" },
            "airing_schedule": [],
            "episode_duration": 0,
            "media_type": "ANIME",
            "cover_image": {
                "extra_large": "",
                "large": "",
                "medium": "",
                "color": ""
            },
            "banner_image": "",
            "recommendations": []
        })
    }

    macro_rules! put_app {
        ($pool:expr, $limits:expr) => {{
            let (um, cm, ca, ch, ent, fi, pp, sa) = build_put_services($pool, $limits).await;
            test::init_service(
                App::new()
                    .app_data(um)
                    .app_data(cm)
                    .app_data(ca)
                    .app_data(ch)
                    .app_data(ent)
                    .app_data(fi)
                    .app_data(pp)
                    .app_data(sa)
                    .app_data(jwt_data())
                    .service(put),
            )
            .await
        }};
    }

    /// Cleanup helper: removes calendar items, calendars, subscriptions, and
    /// the user. Matches the pattern from `frozen_ics` tests.
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
        sqlx::query("DELETE FROM subscriptions WHERE user_id = $1")
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
    async fn put_calendar_without_token_returns_401() {
        let pool = crate::test_helpers::test_pool().await;
        let app = put_app!(pool.clone(), &limits(3, 25));
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
        let app = put_app!(pool.clone(), &limits(3, 25));
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
        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn put_calendar_name_too_long_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = put_app!(pool.clone(), &limits(3, 25));
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
        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn put_calendar_too_many_items_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = put_app!(pool.clone(), &limits(3, 25));
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
        cleanup_user(&pool, user.id).await;
    }

    // ─── PUT /calendar (entitlement enforcement, Phase 1.3) ──────────────────

    #[tokio::test]
    async fn put_calendar_create_at_cap_returns_402() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        // Seed 3 calendars at the configured cap.
        seed_calendar(&pool, user.id, "A").await;
        seed_calendar(&pool, user.id, "B").await;
        seed_calendar(&pool, user.id, "C").await;

        let app = put_app!(pool.clone(), &limits(3, 25));
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "id": 0, "name": "D", "language": "english", "items": []
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "upgrade_required");
        assert_eq!(body["reason"], "cap_calendars");

        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn put_calendar_add_new_item_at_show_cap_returns_402() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        // Seed a calendar containing 2 items so the user is at the show cap.
        let (cal_id, _) = seed_calendar(&pool, user.id, "Existing").await;
        sqlx::query("INSERT INTO calendar_items (calendar_id, item_id) VALUES ($1, 1001), ($1, 1002)")
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();

        let app = put_app!(pool.clone(), &limits(5, 2));
        // Try to update the same calendar with a brand-new item id 9999 — over cap.
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "id": cal_id,
                "name": "Existing",
                "language": "english",
                "items": [item_json(1001), item_json(1002), item_json(9999)]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "upgrade_required");
        assert_eq!(body["reason"], "cap_shows");

        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn put_calendar_add_already_tracked_item_at_cap_passes() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let (cal_id, _) = seed_calendar(&pool, user.id, "Existing").await;
        sqlx::query("INSERT INTO calendar_items (calendar_id, item_id) VALUES ($1, 1001), ($1, 1002)")
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();

        let app = put_app!(pool.clone(), &limits(5, 2));
        // Re-PUT same calendar with the same item ids — idempotent, should pass.
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "id": cal_id,
                "name": "Existing",
                "language": "english",
                "items": [item_json(1001), item_json(1002)]
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        cleanup_user(&pool, user.id).await;
    }

    // ─── PUT /calendar — event_style validation (Phase 2a) ───────────────────

    #[tokio::test]
    async fn put_calendar_invalid_event_style_returns_400() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = put_app!(pool.clone(), &limits(3, 25));
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "id": 0,
                "name": "Bad",
                "language": "english",
                "items": [],
                "event_style": "foo"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "event_style_invalid");

        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn put_calendar_event_style_timed_persists() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = put_app!(pool.clone(), &limits(3, 25));
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "id": 0,
                "name": "Timed Cal",
                "language": "english",
                "items": [],
                "event_style": "timed"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        // Empty items branch returns 404 from the existing handler logic
        // (anilist.get_items is empty), but the calendar row IS created
        // before that branch. Verify the row's event_style.
        // Tolerate either OK (with items) or NOT_FOUND (empty items branch).
        assert!(
            matches!(resp.status(), StatusCode::OK | StatusCode::NOT_FOUND),
            "unexpected status: {}",
            resp.status()
        );
        let stored: String = sqlx::query_scalar(
            "SELECT event_style FROM calendars WHERE user_id = $1 AND name = 'Timed Cal'",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, "timed");

        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn put_calendar_event_style_all_day_persists() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let app = put_app!(pool.clone(), &limits(3, 25));
        let req = test::TestRequest::put()
            .uri("/calendar")
            .insert_header(("Cookie", format!("auth_token={}", user.token)))
            .set_json(serde_json::json!({
                "id": 0,
                "name": "All Day Cal",
                "language": "english",
                "items": [],
                "event_style": "all_day"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(
            matches!(resp.status(), StatusCode::OK | StatusCode::NOT_FOUND),
            "unexpected status: {}",
            resp.status()
        );
        let stored: String = sqlx::query_scalar(
            "SELECT event_style FROM calendars WHERE user_id = $1 AND name = 'All Day Cal'",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored, "all_day");

        cleanup_user(&pool, user.id).await;
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

        let entitlement = EntitlementService::new(
            SubscriptionMapper::from_pool(pool.clone()),
            ShowCountService::new(pool.clone()),
            &limits(3, 25),
        );
        let sharing_authz = SharingAuthz::new(
            CalendarEditorMapper::from_pool(pool.clone()),
            entitlement,
        );
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(web::Data::new(sharing_authz))
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
        let entitlement = EntitlementService::new(
            SubscriptionMapper::from_pool(pool.clone()),
            ShowCountService::new(pool.clone()),
            &limits(3, 25),
        );
        let sharing_authz = SharingAuthz::new(
            CalendarEditorMapper::from_pool(pool.clone()),
            entitlement,
        );
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(web::Data::new(sharing_authz))
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
        let entitlement = EntitlementService::new(
            SubscriptionMapper::from_pool(pool.clone()),
            ShowCountService::new(pool.clone()),
            &limits(3, 25),
        );
        let sharing_authz = SharingAuthz::new(
            CalendarEditorMapper::from_pool(pool.clone()),
            entitlement,
        );
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(UserMapper::from_pool(pool.clone())))
                .app_data(web::Data::new(CalendarMapper::from_pool(pool)))
                .app_data(web::Data::new(Cache::for_tests().await))
                .app_data(web::Data::new(sharing_authz))
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


    // ─── GET /calendars/subscribe/{token} (Phase 1.3 tier branching) ─────────

    /// Build a service-init-app for `subscribe_feed` with all required
    /// Phase-1.3 extractors wired.
    macro_rules! subscribe_app {
        ($pool:expr) => {{
            let pool = $pool;
            let cached = cached_anilist_data().await;
            let entitlement = EntitlementService::new(
                SubscriptionMapper::from_pool(pool.clone()),
                ShowCountService::new(pool.clone()),
                &limits(3, 25),
            );
            let ics_export = IcsExportService::new(
                pool.clone(),
                (**cached).clone(),
                crate::mappers::user_settings::UserSettingsMapper::from_pool(pool.clone()),
                entitlement.clone(),
            );
            let frozen_ics = FrozenIcsService::new(pool.clone(), ics_export.clone());
            test::init_service(
                App::new()
                    .app_data(web::Data::new(CalendarMapper::from_pool(pool.clone())))
                    .app_data(web::Data::new(ics_export))
                    .app_data(web::Data::new(frozen_ics))
                    .app_data(web::Data::new(Cache::for_tests().await))
                    .app_data(web::Data::new(entitlement))
                    .app_data(web::Data::new(SubscriptionMapper::from_pool(pool)))
                    .service(subscribe_feed),
            )
            .await
        }};
    }

    async fn make_paid(pool: &sqlx::PgPool, user_id: i32) {
        use chrono::{Duration, Utc};
        let n: u64 = rand::random();
        let now = Utc::now().naive_utc();
        sqlx::query(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, \
              cancel_at_period_end) \
             VALUES ($1, 'paid', 'active', $2, $3, 'price_test', $4, $5, false)",
        )
        .bind(user_id)
        .bind(format!("cus_test_{n}"))
        .bind(format!("sub_test_{n}"))
        .bind(now)
        .bind(now + Duration::days(30))
        .execute(pool)
        .await
        .unwrap();
    }

    /// Insert a historic (canceled, expired) subscription row so the
    /// `find_latest_customer_id_for_user` lookup returns Some(_) without
    /// granting active entitlement.
    async fn make_history(pool: &sqlx::PgPool, user_id: i32) {
        use chrono::{Duration, Utc};
        let n: u64 = rand::random();
        let then = (Utc::now() - Duration::days(60)).naive_utc();
        sqlx::query(
            "INSERT INTO subscriptions \
             (user_id, tier, status, stripe_customer_id, stripe_subscription_id, \
              stripe_price_id, current_period_start, current_period_end, \
              cancel_at_period_end) \
             VALUES ($1, 'paid', 'canceled', $2, $3, 'price_test', $4, $5, false)",
        )
        .bind(user_id)
        .bind(format!("cus_hist_{n}"))
        .bind(format!("sub_hist_{n}"))
        .bind(then)
        .bind(then + Duration::days(30))
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn subscribe_feed_paid_user_serves_live_render() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        make_paid(&pool, user.id).await;
        let (_, token) = seed_calendar(&pool, user.id, "Live").await;

        let app = subscribe_app!(pool.clone());
        let req = test::TestRequest::get()
            .uri(&format!("/calendars/subscribe/{token}"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("BEGIN:VCALENDAR"));

        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn subscribe_feed_free_with_frozen_blob_serves_blob() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let (cal_id, token) = seed_calendar(&pool, user.id, "Frozen").await;
        // Stamp a sentinel blob that the live render would never produce.
        let sentinel = "BEGIN:VCALENDAR\r\nX-FROZEN-SENTINEL:yes\r\nEND:VCALENDAR\r\n";
        sqlx::query("UPDATE calendars SET frozen_subscribe_ics = $1 WHERE id = $2")
            .bind(sentinel)
            .bind(cal_id)
            .execute(&pool)
            .await
            .unwrap();

        let app = subscribe_app!(pool.clone());
        let req = test::TestRequest::get()
            .uri(&format!("/calendars/subscribe/{token}"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("X-FROZEN-SENTINEL:yes"));

        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn subscribe_feed_free_no_blob_no_history_returns_404() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        let (_, token) = seed_calendar(&pool, user.id, "NoHistory").await;

        let app = subscribe_app!(pool.clone());
        let req = test::TestRequest::get()
            .uri(&format!("/calendars/subscribe/{token}"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        cleanup_user(&pool, user.id).await;
    }

    #[tokio::test]
    async fn subscribe_feed_free_no_blob_with_history_lazy_regenerates() {
        let pool = crate::test_helpers::test_pool().await;
        let user = seed_user(&pool).await;
        make_history(&pool, user.id).await; // canceled sub → has history
        let (cal_id, token) = seed_calendar(&pool, user.id, "Lazy").await;

        // Pre-condition: blob is null.
        let pre: Option<String> =
            sqlx::query_scalar("SELECT frozen_subscribe_ics FROM calendars WHERE id = $1")
                .bind(cal_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(pre.is_none());

        let app = subscribe_app!(pool.clone());
        let req = test::TestRequest::get()
            .uri(&format!("/calendars/subscribe/{token}"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("BEGIN:VCALENDAR"));

        // Post-condition: blob written by lazy regen.
        let post: Option<String> =
            sqlx::query_scalar("SELECT frozen_subscribe_ics FROM calendars WHERE id = $1")
                .bind(cal_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(post.is_some());

        cleanup_user(&pool, user.id).await;
    }
}
