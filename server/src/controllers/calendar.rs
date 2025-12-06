use crate::{error::Error, server::Repos, services::calendar_export::generate_calendar_export};

use actix_web::{get, put, web, HttpResponse, ResponseError};
use common::{calendar::Calendar, id::Id, item::Repository};
use log::error;

#[get("/calendar/{id}/export")]
async fn export(data: web::Data<Repos>, id: web::Path<u64>) -> HttpResponse {
    // TODO * User management?
    // TODO * Calendar public/private visibility
    // TODO * Generate links, maybe endpoint to generate needs to be wildly different
    // TODO * gcal integration?
    // TODO * anilist list?

    match data.database.get_calendar(*id).await {
        Ok(result) => match result {
            Some(calendar_data) => {
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
            None => Error::NotFound.error_response(),
        },
        Err(e) => e.error_response(),
    }
}

#[put("/calendar")]
async fn put(data: web::Data<Repos>, body: web::Json<Calendar>) -> HttpResponse {
    let calendar = crate::entity::calendar::Calendar::from_common(&body);

    match data.database.save_calendar(calendar).await {
        Ok(calendar) => {
            if let Some(id) = Id::new(calendar.id.into()) {
                HttpResponse::Ok().json(web::Json(Calendar { id, ..body.0 }))
            } else {
                Error::NotFound.error_response() // TODO questionable
            }
        }
        Err(e) => {
            error!("{e}");
            e.error_response()
        }
    }
}

#[get("/calendar/{id}")]
async fn get(data: web::Data<Repos>, id: web::Path<u64>) -> HttpResponse {
    match data.database.get_calendar(*id).await {
        Ok(result) => match result {
            Some(calendar) => {
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
                    let calendar = Calendar {
                        id,
                        items,
                        language: calendar.language.to_common_language(),
                        name: calendar.name,
                    };

                    HttpResponse::Ok().json(web::Json(calendar))
                } else {
                    Error::NotFound.error_response()
                }
            }
            None => Error::NotFound.error_response(),
        },
        Err(e) => {
            error!("{e}");
            e.error_response()
        }
    }
}
