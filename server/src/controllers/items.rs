use crate::server::Repos;

use actix_web::{get, web, HttpResponse};
use actix_web_lab::extract::Query;
use common::item::Repository;
use serde::Deserialize;

#[derive(Deserialize)]
struct Params {
    #[serde(rename = "id")]
    ids: Vec<u64>,
}

#[get("/items")]
async fn get(data: web::Data<Repos>, ids: Query<Params>) -> HttpResponse {
    let items = data.anilist.get_items(ids.into_inner().ids).await;

    HttpResponse::Ok().json(items)
}
