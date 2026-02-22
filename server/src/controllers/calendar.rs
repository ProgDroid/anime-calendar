#![allow(clippy::cast_possible_truncation)]
use crate::{
    entity::calendar::{Calendar as CalendarEntity, Language as LanguageEntity},
    error::Error,
    mappers::{calendar::CalendarMapper, user::UserMapper},
    middleware::auth::Claims,
    server::Repos,
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
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct CalendarRequest {
    #[serde(default)]
    pub id: Id,
    pub items: Vec<Item>,
    pub language: Language,
    pub name: String,
}

#[derive(Deserialize, Serialize)]
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

#[get("/calendar/{id}/export")]
async fn export(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    data: web::Data<Repos>,
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

            let items = data.anilist.get_items(item_ids).await;

            if items.is_empty() {
                return Error::NotFound.error_response();
            }

            if let Some(id) = Id::new(calendar_data.id.into()) {
                let calendar = Calendar {
                    id: id.clone(),
                    items,
                    language: calendar_data.language.to_common_language(),
                    name: calendar_data.name,
                    created_at: calendar_data
                        .created_at
                        .expect("Did not load created_at for calendar"), // TODO consider doing this differently,
                    updated_at: calendar_data
                        .updated_at
                        .expect("Did not load updated_at for calendar"), // TODO consider doing this differently,
                };

                let file = generate_calendar_export(&calendar);

                // Cache the response for 2 hours (7200 seconds)
                let cache_key = crate::cache::generate_calendar_key(id.to_int() as i32);
                let cache_ttl = 7200; // 2 hours

                // If cache is available, try to get from cache
                match data
                    .cache
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
                        eprintln!("Cache error: {e:?}");
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

#[allow(clippy::cast_possible_truncation)]
#[put("/calendar")]
async fn put(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    data: web::Data<Repos>,
    body: web::Json<CalendarRequest>,
    claims: Claims,
) -> HttpResponse {
    let user = match user_mapper.get_user_from_claims(&claims).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

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
        user_id: user.id,
        created_at: None,
        updated_at: None,
    };

    // Create calendar with the authenticated user's ID
    match calendar_mapper.save_calendar(calendar_entity).await {
        Ok(calendar) => {
            let item_ids: Vec<Id> = calendar
                .item_ids
                .iter()
                .filter_map(|id| Id::new(i64::from(*id)))
                .collect();

            let items = data.anilist.get_items(item_ids).await;

            if items.is_empty() {
                return Error::NotFound.error_response();
            }

            if let Some(id) = Id::new(calendar.id.into()) {
                // Invalidate cache for this calendar (controller-level invalidation)
                let _ = data.cache.invalidate_calendar(calendar.id).await;
                let _ = data.cache.invalidate_pattern("/calendars:page:*").await;

                HttpResponse::Ok().json(Calendar {
                    id,
                    items,
                    language: calendar.language.to_common_language(),
                    name: calendar.name,
                    created_at: calendar
                        .created_at
                        .expect("Did not load created_at for calendar"), // TODO consider doing this differently
                    updated_at: calendar
                        .updated_at
                        .expect("Did not load updated_at for calendar"), // TODO consider doing this differently
                })
            } else {
                Error::NotFound.error_response()
            }
        }
        Err(e) => e.error_response(),
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct PageCalendar {
    pub id: Id,
    pub item_count: usize,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Clone)]
struct PaginatedResponse {
    data: Vec<PageCalendar>,
    pagination: PaginationInfo,
}

#[derive(Serialize, Deserialize, Clone)]
struct PaginationInfo {
    page: usize,
    page_size: usize,
    total: usize,
    total_pages: usize,
}

#[get("/calendars")]
async fn get_calendars(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    data: web::Data<Repos>,
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
                        created_at: calendar
                            .created_at
                            .expect("Did not load created_at for calendar"), // TODO consider doing this differently
                        updated_at: calendar
                            .updated_at
                            .expect("Did not load updated_at for calendar"), // TODO consider doing this differently
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
            let cache_key =
                crate::cache::generate_paginated_key("/calendars", params.page, params.page_size);
            let cache_ttl = 1800; // 30 minutes

            // If cache is available, try to get from cache
            match data
                .cache
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
                    eprintln!("Cache error: {e:?}");
                }
            }

            // If no cache or cache error, return the response normally
            HttpResponse::Ok().json(paginated_response)
        }
        Err(e) => e.error_response(),
    }
}

#[get("/calendars/{id}")]
async fn get_calendar(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    data: web::Data<Repos>,
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

            let items = data.anilist.get_items(item_ids).await;

            if items.is_empty() {
                return Error::NotFound.error_response();
            }

            if let Some(id) = Id::new(calendar.id.into()) {
                let calendar_response = Calendar {
                    id: id.clone(),
                    items,
                    language: calendar.language.to_common_language(),
                    name: calendar.name,
                    created_at: calendar
                        .created_at
                        .expect("Did not load created_at for calendar"), // TODO consider doing this differently
                    updated_at: calendar
                        .updated_at
                        .expect("Did not load updated_at for calendar"), // TODO consider doing this differently
                };

                // Cache the response for 1 hour (3600 seconds)
                let cache_key = crate::cache::generate_calendar_key(id.to_int() as i32);
                let cache_ttl = 3600; // 1 hour

                // If cache is available, try to get from cache
                match data
                    .cache
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
                        eprintln!("Cache error: {e:?}");
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

#[delete("/calendars/{id}")]
async fn delete_calendar(
    user_mapper: web::Data<UserMapper>,
    calendar_mapper: web::Data<CalendarMapper>,
    data: web::Data<Repos>,
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
        Ok(()) => {
            // Invalidate cache for this calendar (controller-level invalidation)
            let _ = data.cache.invalidate_calendar(*id as i32).await;
            let _ = data.cache.invalidate_pattern("/calendars:page:*").await;
            HttpResponse::Ok().finish()
        }
        Err(e) => e.error_response(),
    }
}

// TODO character limits
// TODO remove user settings that I don't intend to implement
// - date display format
// - date separator
