use crate::{
    error::Error, server::Repos, services::calendar_export::generate_calendar_export, ServerResult,
};

use actix_web::{get, web, HttpResponse, ResponseError};
use common::{calendar::Calendar, id::Id, item::Repository, language::Language};

#[get("/calendar/{id}/export")]
async fn export(data: web::Data<Repos>, id: web::Path<u64>) -> HttpResponse {
    // TODO complete calendars
    // * Database storage
    // * User management?
    // * Calendar public/private visibility
    // * Generate links, maybe endpoint to generate needs to be wildly different
    // * gcal integration?
    // * anilist list?
    // * actually load calendar and get data from it here

    // TODO should this first load a DbCalendar then populate the items to make a Calendar?
    // TODO db mapper should not load stuff from Anilist
    // TODO should db mapper return incomplete calendar object or should it return a different object that then makes the calendar object
    // TODO alternatively if a calendar always needs multiple repos it should be done in a service? or is the logic OK in this controller?

    match data.database.get_calendar(*id).await {
        Ok(data) => match data {
            Some(calendar) => {
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
