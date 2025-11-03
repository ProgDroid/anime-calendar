use crate::{error::Error, server::Repos, services::calendar_export::generate_calendar_export};

use actix_web::{get, put, web, HttpResponse, ResponseError};
use common::{calendar::Calendar, id::Id, item::Repository};

#[get("/calendar/{id}/export")]
async fn export(data: web::Data<Repos>, id: web::Path<u64>) -> HttpResponse {
    // * User management?
    // * Calendar public/private visibility
    // * Generate links, maybe endpoint to generate needs to be wildly different
    // * gcal integration?
    // * anilist list?

    match data.database.get_calendar(*id).await {
        Ok(result) => match result {
            Some(calendar_data) => {
                let item_ids: Vec<u64> = calendar_data
                    .item_ids
                    .iter()
                    .map(|id| (*id).try_into().unwrap())
                    .collect(); // TODO fix

                let items = data.anilist.get_items(item_ids).await;

                if items.is_empty() {
                    return Error::NotFound.error_response();
                }

                let calendar = Calendar {
                    id: Id::new(calendar_data.id.into()).unwrap(), // TODO handle
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
        Ok(calendar) => HttpResponse::Ok().json(web::Json(Calendar {
            id: Id::new(calendar.id.into()).unwrap(), // TODO fix
            ..body.0
        })),
        Err(e) => {
            // TODO log
            e.error_response() // TODO sort
        }
    }
}

#[get("/calendar/{id}")]
async fn get(data: web::Data<Repos>, id: web::Path<u64>) -> HttpResponse {
    match data.database.get_calendar(*id).await {
        Ok(result) => {
            match result {
                Some(calendar) => {
                    let item_ids: Vec<u64> = calendar
                        .item_ids
                        .iter()
                        .map(|id| (*id).try_into().unwrap())
                        .collect(); // TODO fix

                    let items = data.anilist.get_items(item_ids).await;

                    if items.is_empty() {
                        return Error::NotFound.error_response();
                    }

                    let calendar = Calendar {
                        id: Id::new(calendar.id.into()).unwrap(), // TODO handle
                        items,
                        language: calendar.language.to_common_language(),
                        name: calendar.name,
                    };

                    HttpResponse::Ok().json(web::Json(calendar))
                }
                None => Error::NotFound.error_response(),
            }
        }
        Err(e) => {
            // TODO log
            e.error_response() // TODO sort
        }
    }
}
