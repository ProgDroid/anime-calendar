use crate::server::Repos;

use actix_web::{get, web, HttpResponse};
use actix_web_lab::extract::Query;
use common::{
    id::Id,
    item::{Repository, Type},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Params {
    #[serde(rename = "id")]
    ids: Vec<u64>,
}

#[allow(clippy::cast_possible_wrap)]
#[get("/items")]
async fn get(data: web::Data<Repos>, ids: Query<Params>) -> HttpResponse {
    let ids: Vec<Id> = ids
        .into_inner()
        .ids
        .iter()
        .filter_map(|id| Id::new(*id as i64))
        .collect();

    if ids.is_empty() {
        return HttpResponse::BadRequest().finish();
    }

    let items = data.anilist.get_items(ids).await;

    HttpResponse::Ok().json(items)
}

#[derive(Deserialize)]
struct SearchParams {
    name: String,
    media_type: Option<Type>,
}

#[get("/search")]
async fn search(data: web::Data<Repos>, name: Query<SearchParams>) -> HttpResponse {
    let query = name.name.trim();

    if query.is_empty() {
        return HttpResponse::BadRequest().finish();
    }

    let items = data
        .anilist
        .search_items(query.to_string(), name.media_type.clone())
        .await;

    HttpResponse::Ok().json(items)
}
