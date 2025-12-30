#![allow(unused_variables)]
use crate::{
    entity::calendar::{Calendar as CalendarEntity, Language as LanguageEntity},
    error::Error,
    middleware::auth::{get_user_from_claims, Claims},
    server::Repos,
    services::calendar_export::generate_calendar_export,
};

use actix_web::{delete, get, put, web, HttpResponse, ResponseError};
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
    pub name: String, // TODO Name object?
}

#[allow(clippy::cast_possible_truncation)]
#[get("/calendar/{id}/export")]
async fn export(data: web::Data<Repos>, id: web::Path<u64>, claims: Claims) -> HttpResponse {
    let user = match get_user_from_claims(&claims, &data.database).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // TODO * Calendar sharing? (with permissions)
    // TODO * User management?
    // TODO * Calendar public/private visibility
    // TODO * Generate links, maybe endpoint to generate needs to be wildly different
    // TODO * gcal integration?
    // TODO * anilist list?
    // TODO * Calendar templates? (predefined items)

    match data.database.get_calendar_by_id(*id as i32, user.id).await {
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
                    id,
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
    data: web::Data<Repos>,
    body: web::Json<CalendarRequest>,
    claims: Claims,
) -> HttpResponse {
    let user = match get_user_from_claims(&claims, &data.database).await {
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
    match data.database.save_calendar(calendar_entity).await {
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

#[get("/calendars")]
async fn get_calendars(data: web::Data<Repos>, claims: Claims) -> HttpResponse {
    let user = match get_user_from_claims(&claims, &data.database).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // // Get calendars for the authenticated user
    // match data.database.get_calendars_by_user(user.id).await {
    //     Ok(calendars) => HttpResponse::Ok().json(calendars),
    //     Err(e) => e.error_response(),
    // }
    HttpResponse::Ok().finish()
}

#[get("/calendars/{id}")]
async fn get_calendar(data: web::Data<Repos>, id: web::Path<u64>, claims: Claims) -> HttpResponse {
    let user = match get_user_from_claims(&claims, &data.database).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Check if the calendar belongs to the authenticated user
    // match data.database.get_calendar_by_id(*id, user.id).await {
    //     Ok(Some(calendar)) => HttpResponse::Ok().json(calendar),
    //     Ok(None) => Error::Unauthorised.error_response(),
    //     Err(e) => e.error_response(),
    // }
    HttpResponse::Ok().finish()
}

#[delete("/calendars/{id}")]
async fn delete_calendar(
    data: web::Data<Repos>,
    id: web::Path<u64>,
    claims: Claims,
) -> HttpResponse {
    let user = match get_user_from_claims(&claims, &data.database).await {
        Ok(user) => user,
        Err(e) => {
            return e.error_response();
        }
    };

    // Check if the calendar belongs to the authenticated user
    // match data.database.get_calendar_by_user(*id, user.id).await {
    //     Ok(Some(_)) => {
    //         // Calendar belongs to user, proceed with deletion
    //         match data.database.delete_calendar(*id).await {
    //             Ok(_) => HttpResponse::Ok().finish(),
    //             Err(e) => e.error_response(),
    //         }
    //     }
    //     Ok(None) => Error::Unauthorised.error_response(),
    //     Err(e) => e.error_response(),
    // }
    HttpResponse::Ok().finish()
}
