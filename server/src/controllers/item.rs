use crate::server::Repos;

use actix_web::{get, web, HttpResponse};
use common::{id::Id, item::Repository};

#[allow(clippy::cast_possible_wrap)]
#[get("/item/{id}")]
async fn get(data: web::Data<Repos>, id: web::Path<u64>) -> HttpResponse {
    if let Some(id) = Id::new(id.into_inner() as i64) {
        let item = data.anilist.get_item(id).await;

        item.map_or_else(
            || HttpResponse::NotFound().finish(),
            |item| HttpResponse::Ok().json(item),
        )
    } else {
        HttpResponse::BadRequest().finish()
    }
}
