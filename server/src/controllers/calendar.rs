use crate::{server::Repos, services::calendar_export::generate_calendar_export};

use actix_web::{get, web, HttpResponse};
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
    let dan_da_dan = data.anilist.get_item(185660).await;

    if dan_da_dan.is_none() {
        return HttpResponse::NotFound().finish();
    }

    let items = vec![dan_da_dan.unwrap()];

    let calendar = Calendar {
        id: Id::new(1).unwrap(),
        language: Language::English,
        name: String::from("Anime Calendar"),
        items,
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
