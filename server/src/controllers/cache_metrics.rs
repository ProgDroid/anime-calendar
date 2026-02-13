use crate::server::Repos;
use actix_web::{web, HttpResponse, Result};
use serde::Serialize;

#[derive(Serialize)]
pub struct CacheMetricsResponse {
    pub hit_rate: f64,
    pub total_requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub cache_size: usize,
}

#[derive(Serialize)]
pub struct CachePerformanceResponse {
    pub hit_rate: f64,
    pub total_requests: u64,
    pub cache_size: usize,
    pub hits: u64,
    pub misses: u64,
    pub status: String,
}

/// Get current cache metrics
#[actix_web::get("/cache/metrics")]
pub async fn get_cache_metrics(repos: web::Data<Repos>) -> Result<HttpResponse> {
    // Get current metrics from cache
    let metrics = repos.cache.get_metrics().await;

    let response = CacheMetricsResponse {
        hit_rate: metrics.cache_hit_rate,
        total_requests: metrics.total_requests,
        hits: metrics.hits,
        misses: metrics.misses,
        cache_size: 0, // We'll calculate this in the performance endpoint
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get detailed cache performance metrics
#[actix_web::get("/cache/performance")]
pub async fn get_cache_performance(repos: web::Data<Repos>) -> Result<HttpResponse> {
    match repos.cache.monitor_performance().await {
        Ok(performance) => {
            let response = CachePerformanceResponse {
                hit_rate: performance.hit_rate,
                total_requests: performance.total_requests,
                cache_size: performance.cache_size,
                hits: performance.hits,
                misses: performance.misses,
                status: "healthy".to_string(),
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            eprintln!("Error monitoring cache performance: {e}");
            Ok(HttpResponse::InternalServerError().json("Failed to monitor cache performance"))
        }
    }
}

/// Get cache health status
#[actix_web::get("/cache/health")]
pub async fn get_cache_health(repos: web::Data<Repos>) -> Result<HttpResponse> {
    // Simple health check - just verify we can access the cache
    let is_available = repos.cache.is_available();

    let response = serde_json::json!({
        "status": if is_available { "healthy" } else { "unhealthy" },
        "available": is_available,
        "metrics": repos.cache.get_metrics().await
    });

    Ok(HttpResponse::Ok().json(response))
}

/// Reset cache metrics
#[actix_web::post("/cache/reset")]
pub async fn reset_metrics(repos: web::Data<Repos>) -> Result<HttpResponse> {
    repos.cache.reset_metrics().await;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Cache metrics reset successfully"
    })))
}

/// Get detailed cache statistics
#[actix_web::get("/cache/stats")]
pub async fn get_cache_stats(repos: web::Data<Repos>) -> Result<HttpResponse> {
    let metrics = repos.cache.get_metrics().await;
    let keys = repos.cache.get_keys("*").await.unwrap_or_else(|_| vec![]);

    let response = serde_json::json!({
        "metrics": metrics,
        "cache_size": keys.len(),
        "keys": keys
    });

    Ok(HttpResponse::Ok().json(response))
}

#[actix_web::get("/cache/flush")]
pub async fn flush_cache(repos: web::Data<Repos>) -> Result<HttpResponse> {
    let _ = repos.cache.invalidate_pattern("*").await;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Cache flushed successfully"
    })))
}
