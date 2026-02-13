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

    // Cache the response for 1 hour (3600 seconds)
    let cache_key = crate::cache::generate_items_key(&ids);
    let cache_ttl = 3600; // 1 hour

    // If cache is available, try to get from cache
    match data
        .cache
        .cached_response(&cache_key, cache_ttl, || async {
            let items = data.anilist.get_items(ids.clone()).await;
            Ok(items)
        })
        .await
    {
        Ok(cached_items) => {
            return HttpResponse::Ok().json(cached_items);
        }
        Err(e) => {
            // Log error but continue with regular processing
            eprintln!("Cache error: {e:?}");
        }
    }

    // If no cache or cache error, fetch and return normally
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

    let media_type_string = name
        .media_type
        .as_ref()
        .map(std::string::ToString::to_string);

    // Cache the response for 30 minutes (1800 seconds)
    let cache_key = crate::cache::generate_search_key(query, media_type_string.as_deref());
    let cache_ttl = 1800; // 30 minutes

    // If cache is available, try to get from cache
    match data
        .cache
        .cached_response(&cache_key, cache_ttl, || async {
            let items = data
                .anilist
                .search_items(query.to_string(), name.media_type.clone())
                .await;
            Ok(items)
        })
        .await
    {
        Ok(cached_items) => {
            return HttpResponse::Ok().json(cached_items);
        }
        Err(e) => {
            // Log error but continue with regular processing
            eprintln!("Cache error: {e:?}");
        }
    }

    // If no cache or cache error, fetch and return normally
    let items = data
        .anilist
        .search_items(query.to_string(), name.media_type.clone())
        .await;

    HttpResponse::Ok().json(items)
}
