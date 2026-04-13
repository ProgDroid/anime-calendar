use actix_web::{get, web, HttpResponse};
use common::{id::Id, item::Repository};
use log::error;

use crate::{cache::{Cache, CACHE_TTL_ITEM}, mappers::anilist::Anilist};

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
async fn get(
    anilist: web::Data<Anilist>,
    cache: web::Data<Cache>,
    id: web::Path<u64>,
) -> HttpResponse {
    if let Some(id) = Id::new(id.into_inner() as i64) {
        // Cache the response for 1 hour (3600 seconds)
        let cache_key = crate::cache::generate_item_key(id.to_int() as i64);
        let cache_ttl = CACHE_TTL_ITEM;

        // If cache is available, try to get from cache
        match cache
            .cached_response(&cache_key, cache_ttl, || async {
                let item = anilist.get_item(id.clone()).await;
                Ok(item)
            })
            .await
        {
            Ok(cached_item) => {
                return cached_item.map_or_else(
                    || HttpResponse::NotFound().finish(),
                    |item| HttpResponse::Ok().json(item),
                );
            }
            Err(e) => {
                // Log error but continue with regular processing
                error!("Cache error: {e:?}");
            }
        }

        // If no cache or cache error, fetch and return normally
        let item = anilist.get_item(id).await;

        item.map_or_else(
            || HttpResponse::NotFound().finish(),
            |item| HttpResponse::Ok().json(item),
        )
    } else {
        HttpResponse::BadRequest().finish()
    }
}
