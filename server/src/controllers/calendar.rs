#![allow(clippy::cast_possible_truncation)]
use crate::{
    cache::{Cache, CACHE_TTL_CALENDAR, CACHE_TTL_ITEM, CACHE_TTL_SEARCH},
    entity::calendar::{Calendar as CalendarEntity, Language as LanguageEntity},
    error::Error,
    mappers::{anilist::Anilist, calendar::CalendarMapper, user::UserMapper},
    middleware::auth::Claims,
    services::calendar_export::generate_calendar_export,
};

use actix_web::{delete, get, put, web, HttpResponse, ResponseError};
use chrono::NaiveDateTime;
use common::{
    calendar::Calendar,
    id::Id,
    item::{Item, Repository},
    language::Language,
};
use log::error;
use serde::{Deserialize, Serialize};

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
    anilist: web::Data<Anilist>,
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

    // TODO * Calendar sharing? (with permissions)
    // TODO * Calendar public/private visibility
    // TODO * Generate links, maybe endpoint to generate needs to be wildly different
    // TODO * gcal integration?
    // TODO * anilist list?
    // TODO * Calendar templates? (predefined items)
    // TODO * config.toml in frontend is accessible via URL and downloadable. This needs to be changed
    // TODO * search calendar
    // TODO * are item endpoints needed?

    match calendar_mapper
        .get_calendar_by_id(*id as i32, user.id)
        .await
    {
        Ok(calendar_data) => {
            // TODO Check if calendar belongs to the current user or is public
            // For now, we'll assume all calendars are user-specific
            let item_ids: Vec<Id> = calendar_data
                .item_ids
                .iter()
                .filter_map(|id| Id::new(i64::from(*id)))
                .collect();

            let items = anilist.get_items(item_ids).await;

            if items.is_empty() {
                return Error::NotFound.error_response();
            }

            if let Some(id) = Id::new(calendar_data.id.into()) {
                let calendar = Calendar {
                    id: id.clone(),
                    items,
                    language: calendar_data.language.to_common_language(),
                    name: calendar_data.name,
                    created_at: calendar_data.created_at,
                    updated_at: calendar_data.updated_at,
                };

                let file = generate_calendar_export(&calendar);

                // Cache the export separately from the JSON response to avoid key collision
                let cache_key = crate::cache::generate_export_key(id.to_int() as i32);
                let cache_ttl = CACHE_TTL_CALENDAR;

                // If cache is available, try to get from cache
                match cache
                    .cached_response(&cache_key, cache_ttl, || async { Ok(format!("{file}")) })
                    .await
                {
                    Ok(cached_file) => {
                        return HttpResponse::Ok()
                            .append_header(("Content-Type", "text/calendar"))
                            .append_header((
                                "Content-Disposition",
                                format!(
                                    "attachment; filename=\"{}.{}\"",
                                    calendar.name.replace(' ', "_").to_lowercase(),
                                    "ics"
                                ),
                            ))
                            .body(cached_file);
                    }
                    Err(e) => {
                        // Log error but continue with regular processing
                        error!("Cache error: {e:?}");
                    }
                }

                HttpResponse::Ok()
                    .append_header(("Content-Type", "text/calendar"))
                    .append_header((
                        "Content-Disposition",
                        format!(
                            "attachment; filename=\"{}.{}\"",
                            calendar.name.replace(' ', "_").to_lowercase(),
                            "ics"
                        ),
                    ))
                    .body(format!("{file}"))
            } else {
                Error::NotFound.error_response()
            }
        }
        Err(e) => e.error_response(),
    }
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
    anilist: web::Data<Anilist>,
    cache: web::Data<Cache>,
    token: web::Path<String>,
) -> HttpResponse {
    match calendar_mapper.get_calendar_by_token(&token).await {
        Ok(calendar_data) => {
            let item_ids: Vec<Id> = calendar_data
                .item_ids
                .iter()
                .filter_map(|id| Id::new(i64::from(*id)))
                .collect();

            let items = anilist.get_items(item_ids).await;

            if items.is_empty() {
                return Error::NotFound.error_response();
            }

            let Some(id) = Id::new(calendar_data.id.into()) else {
                return Error::NotFound.error_response();
            };

            let calendar = Calendar {
                id: id.clone(),
                items,
                language: calendar_data.language.to_common_language(),
                name: calendar_data.name,
                created_at: calendar_data.created_at,
                updated_at: calendar_data.updated_at,
            };

            let file = generate_calendar_export(&calendar);

            let cache_key = format!("subscribe:{}", token.as_str());
            let cache_ttl = CACHE_TTL_ITEM;

            match cache
                .cached_response(&cache_key, cache_ttl, || async { Ok(format!("{file}")) })
                .await
            {
                Ok(cached_file) => {
                    return HttpResponse::Ok()
                        .append_header(("Content-Type", "text/calendar; charset=utf-8"))
                        .body(cached_file);
                }
                Err(e) => {
                    error!("Cache error: {e:?}");
                }
            }

            HttpResponse::Ok()
                .append_header(("Content-Type", "text/calendar; charset=utf-8"))
                .body(format!("{file}"))
        }
        Err(e) => e.error_response(),
    }
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
#[allow(clippy::cast_possible_truncation)]
#[put("/calendar")]
async fn put(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    anilist: web::Data<Anilist>,
    cache: web::Data<Cache>,
    body: web::Json<CalendarRequest>,
    claims: Claims,
) -> HttpResponse {
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
                let _ = cache
                    .invalidate_pattern(format!("{}:calendars:page:*", user.id).as_str())
                    .await;
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
    pub name: String,
    pub subscription_token: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
            let mut results: Vec<PageCalendar> = Vec::new();

            for calendar in calendars {
                if let Some(id) = Id::new(calendar.id.into()) {
                    results.push(PageCalendar {
                        id,
                        item_count: calendar.item_ids.len(),
                        name: calendar.name,
                        subscription_token: calendar.subscription_token,
                        created_at: calendar.created_at,
                        updated_at: calendar.updated_at,
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
                "/calendars",
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
    anilist: web::Data<Anilist>,
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
