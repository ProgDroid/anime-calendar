use actix_web::{HttpResponse, get, web};
use actix_web_lab::extract::Query;
use common::{
    id::Id,
    item::{AnimeDataSource, Type},
};
use serde::Deserialize;

use crate::services::cached_anilist::CachedAnilist;

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct Params {
    /// One or more Anilist media IDs
    #[param(rename = "id")]
    #[serde(rename = "id")]
    ids: Vec<u64>,
}

#[utoipa::path(
    get,
    path = "/items",
    tag = "items",
    operation_id = "getItems",
    params(Params),
    responses(
        (status = 200, body = Vec<common::item::Item>),
        (status = 400, description = "No IDs provided"),
    )
)]
#[allow(clippy::cast_possible_wrap)]
#[get("/items")]
async fn get(anilist: web::Data<CachedAnilist>, ids: Query<Params>) -> HttpResponse {
    let ids: Vec<Id> = ids
        .into_inner()
        .ids
        .iter()
        .filter_map(|id| Id::new(*id as i64))
        .collect();

    if ids.is_empty() {
        return HttpResponse::BadRequest().finish();
    }

    let items = anilist.get_items(ids).await;
    HttpResponse::Ok().json(items)
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct SearchParams {
    /// Title to search for
    name: String,
    /// Filter by media type (ANIME or MANGA)
    media_type: Option<Type>,
}

#[utoipa::path(
    get,
    path = "/search",
    tag = "items",
    params(SearchParams),
    responses(
        (status = 200, body = Vec<common::item::Item>),
        (status = 400, description = "Empty search query"),
    )
)]
#[get("/search")]
async fn search(anilist: web::Data<CachedAnilist>, name: Query<SearchParams>) -> HttpResponse {
    let query = name.name.trim();

    if query.is_empty() {
        return HttpResponse::BadRequest().finish();
    }

    let items = anilist
        .search_items(query.to_string(), name.media_type.clone())
        .await;

    HttpResponse::Ok().json(items)
}
