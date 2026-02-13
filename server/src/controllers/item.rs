use crate::server::Repos;

use actix_web::{get, web, HttpResponse};
use common::{id::Id, item::Repository};

#[allow(clippy::cast_possible_wrap)]
#[get("/item/{id}")]
async fn get(data: web::Data<Repos>, id: web::Path<u64>) -> HttpResponse {
    if let Some(id) = Id::new(id.into_inner() as i64) {
        // Cache the response for 1 hour (3600 seconds)
        let cache_key = crate::cache::generate_item_key(id.to_int() as i64);
        let cache_ttl = 3600; // 1 hour

        // If cache is available, try to get from cache
        match data
            .cache
            .cached_response(&cache_key, cache_ttl, || async {
                let item = data.anilist.get_item(id.clone()).await;
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
                eprintln!("Cache error: {e:?}");
            }
        }

        // If no cache or cache error, fetch and return normally
        let item = data.anilist.get_item(id).await;

        item.map_or_else(
            || HttpResponse::NotFound().finish(),
            |item| HttpResponse::Ok().json(item),
        )
    } else {
        HttpResponse::BadRequest().finish()
    }
}
