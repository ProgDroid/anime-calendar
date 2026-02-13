use crate::cache::CacheMetrics;
use log::{error, info, warn};

/// Log cache operation metrics
pub fn log_cache_metrics(metrics: &CacheMetrics, operation: &str) {
    info!(
        "Cache {} - Hits: {}, Misses: {}, Hit Rate: {:.2}%",
        operation, metrics.hits, metrics.misses, metrics.cache_hit_rate
    );
}

/// Log cache performance with timing
pub fn log_cache_performance(operation: &str, duration: std::time::Duration, success: bool) {
    let ms = duration.as_millis();
    if success {
        info!("Cache {operation} completed in {ms}ms");
    } else {
        warn!("Cache {operation} failed after {ms}ms");
    }
}

/// Log cache invalidation events
pub fn log_cache_invalidation(key: &str, operation: &str) {
    info!("Cache invalidated for key: {key} (operation: {operation})");
}

/// Log cache errors
pub fn log_cache_error(operation: &str, error: &dyn std::error::Error) {
    error!("Cache operation {operation} failed: {error}");
}

/// Log cache size monitoring
pub fn log_cache_size(size: usize, threshold: usize) {
    if size > threshold {
        warn!("Cache size {size} exceeds threshold {threshold}");
    } else {
        info!("Cache size: {size} items");
    }
}
