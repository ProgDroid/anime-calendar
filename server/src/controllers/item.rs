use actix_web::{HttpResponse, get, web};
use common::{id::Id, item::AnimeDataSource};

use crate::services::cached_anilist::CachedAnilist;

#[utoipa::path(
    get,
    path = "/item/{id}",
    tag = "items",
    params(("id" = u64, Path, description = "Anilist media ID")),
    responses(
        (status = 200, body = common::item::Item),
        (status = 400, description = "Invalid ID"),
        (status = 404, description = "Item not found"),
    )
)]
#[allow(clippy::cast_possible_wrap)]
#[get("/item/{id}")]
async fn get(anilist: web::Data<CachedAnilist>, id: web::Path<u64>) -> HttpResponse {
    if let Some(id) = Id::new(id.into_inner() as i64) {
        anilist.get_item(id).await.map_or_else(
            || HttpResponse::NotFound().finish(),
            |item| HttpResponse::Ok().json(item),
        )
    } else {
        HttpResponse::BadRequest().finish()
    }
}
