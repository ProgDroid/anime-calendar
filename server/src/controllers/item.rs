use crate::server::Repos;

use actix_web::{get, web, HttpResponse};
use common::item::Repository;

#[get("/item/{id}")]
async fn get(data: web::Data<Repos>, id: web::Path<u64>) -> HttpResponse {
    let item = data.anilist.get_item(id.into_inner()).await;

    item.map_or_else(
        || HttpResponse::NotFound().finish(),
        |item| HttpResponse::Ok().json(item),
    )
}
